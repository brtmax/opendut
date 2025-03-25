use crate::resource::storage::DatabaseConnectInfo;
use backon::Retryable;
use diesel::ConnectionError;
use diesel_async::{AsyncConnection, AsyncPgConnection};
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use diesel_async::pooled_connection::{bb8, AsyncDieselConnectionManager};
use diesel_async::pooled_connection::bb8::Pool;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use tracing::{debug, info, warn};
use url::Url;


pub mod schema;

pub async fn connection_pool(database_connect_info: &DatabaseConnectInfo) -> Result<Pool<AsyncPgConnection>, ConnectError> {
    let DatabaseConnectInfo { url, username, password } = database_connect_info;

    let confidential_url = {
        let mut url = url.clone();
        url.set_username(username)
            .expect("failed to set username on URL while connecting to database");
        url.set_password(Some(password.secret()))
            .expect("failed to set password on URL while connecting to database");
        url
    };

    {
        let connection = confidential_connect(&confidential_url, url.to_string()).await?;
        run_pending_migrations(connection).await
            .map_err(|source| ConnectError::Migration { source })?;
    }

    let connection_manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(confidential_url);
    let connection_pool = Pool::builder().build(connection_manager).await?;
    info!("Connection pool created for database at: {url}");

    Ok(connection_pool)
}

async fn confidential_connect(confidential_url: &Url, url: String) -> Result<AsyncPgConnection, ConnectError> {
    (|| async {
        AsyncPgConnection::establish(confidential_url.as_str()).await
    })
        .retry(backon::ExponentialBuilder::default())
        .when(|cause| match &cause {
            ConnectionError::BadConnection(_) => {
                true
            }
            ConnectionError::CouldntSetupConfiguration(_)
            | ConnectionError::InvalidConnectionUrl(_)
            | ConnectionError::InvalidCString(_) => {
                false
            }
            other => {
                warn!("Unhandled Diesel ConnectionError variant: {other:?}");
                false
            }
        })
        .notify(|cause, after| {
            warn!("Connecting to database at {url} failed. Retrying in {after:?}.\n  {cause}");
        })
        .await
        .map_err(ConnectError::DieselConnection)
}

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("src/resource/persistence/database/migrations/");

async fn run_pending_migrations(connection: AsyncPgConnection) -> MigrationResult<()> {
    let mut connection: AsyncConnectionWrapper<AsyncPgConnection> = AsyncConnectionWrapper::from(connection);

    tokio::task::spawn_blocking::<_, MigrationResult<()>>(move || {
        let migrated_versions = connection.run_pending_migrations(MIGRATIONS)?;

        if migrated_versions.is_empty() {
            debug!("No database migrations had to be applied.");
        } else {
            let migrated_versions = migrated_versions.into_iter()
                .map(|version| version.to_string())
                .collect::<Vec<String>>()
                .join(", ");
            info!("Completed running pending database migrations: {migrated_versions}");
        }

        Ok(())
    }).await??;

    Ok(())
}

type MigrationError = Box<dyn std::error::Error + Send + Sync>;
type MigrationResult<T> = Result<T, MigrationError>;

#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    #[error("Connection error from Diesel")]
    DieselConnection(#[from] diesel::ConnectionError),
    #[error("Error while creating connection pool from Diesel.")]
    DieselConnectionPoolCreate(#[from] diesel_async::pooled_connection::PoolError),
    #[error("Error while retrieving a connection from the connection pool from Diesel.")]
    DieselConnectionPoolRetrieve(#[from] bb8::RunError),
    #[error("Error while applying migrations")]
    Migration { #[from] source: MigrationError },
}


#[cfg(any(test, doc))] //needed for doctests to compile
pub mod testing {
    use crate::resource::api::global::GlobalResources;
    use crate::resource::manager::{ResourceManager, ResourceManagerRef};
    use crate::resource::storage::{DatabaseConnectInfo, Password, PersistenceOptions};
    use testcontainers_modules::testcontainers::ContainerAsync;
    use testcontainers_modules::{postgres, testcontainers::runners::AsyncRunner};
    use url::Url;

    /// Spawns a Postgres Container and returns a ResourceManager for testing.
    /// ```no_run
    /// # use std::any::Any;
    /// # use opendut_carl::resource::persistence::database;
    ///
    /// #[tokio::test]
    /// async fn test() {
    ///     let mut db = database::testing::spawn_and_connect_resource_manager().await?;
    ///
    ///     do_something_with_resource_manager(db.resource_manager);
    /// }
    ///
    /// # fn do_something_with_resource_manager(resource_manager: impl Any) {}
    /// ```
    pub async fn spawn_and_connect_resource_manager() -> anyhow::Result<PostgresResources> {
        let (container, connect_info) = spawn().await?;

        let global = GlobalResources::default().complete();
        let persistence_options = PersistenceOptions::Enabled {
            database_connect_info: connect_info.clone(),
        };
        let resource_manager = ResourceManager::create(global, persistence_options).await?;

        Ok(PostgresResources { container, resource_manager })
    }
    pub struct PostgresResources {
        #[allow(unused)] //primarily carried along to extend its lifetime until the end of the test (container is stopped when variable is dropped)
        pub container: ContainerAsync<postgres::Postgres>,
        pub resource_manager: ResourceManagerRef,
    }

    async fn spawn() -> anyhow::Result<(ContainerAsync<postgres::Postgres>, DatabaseConnectInfo)> {
        let container = postgres::Postgres::default().start().await?;
        let host = container.get_host().await?;
        let port = container.get_host_port_ipv4(5432).await?;

        let connect_info = DatabaseConnectInfo {
            url: Url::parse(&format!("postgres://{host}:{port}/postgres"))?,
            username: String::from("postgres"),
            password: Password::new_static("postgres"),
        };

        Ok((container, connect_info))
    }
}
