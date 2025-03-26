use crate::resource::api::resources::RelayedSubscriptionEvents;
use crate::resource::persistence::database::ConnectError;
use crate::resource::persistence::error::{PersistenceError, PersistenceResult};
use crate::resource::persistence::resources::Persistable;
use crate::resource::persistence::{Db, Storage};
use crate::resource::storage::volatile::VolatileResourcesStorage;
use crate::resource::storage::{DatabaseConnectInfo, Resource, ResourcesStorageApi};
use diesel::{Connection, PgConnection};
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::debug;

pub struct PersistentResourcesStorage {
    db_connection: redb::Database,
    memory: Arc<Mutex<VolatileResourcesStorage>>,
}
impl PersistentResourcesStorage {
    pub async fn connect(database_connect_info: &DatabaseConnectInfo) -> Result<Self, ConnectError> {
        let _ = crate::resource::persistence::database::connect(database_connect_info).await?; //TODO remove or use for migration

        let file = "/opt/opendut-carl/config/opendut.redb";
        let db_connection = redb::Database::create(file).unwrap(); //FIXME //TODO don't unwrap //TODO make name configurable and set it to a temporary path during tests
        debug!("Database file opened from: {file}");

        let memory = VolatileResourcesStorage::default();
        let memory = Arc::new(Mutex::new(memory));

        Ok(Self { db_connection, memory })
    }

    pub async fn resources<T, F>(&self, code: F) -> T
    where
        F: AsyncFnOnce(PersistentResourcesTransaction) -> T,
    {
        let mut relayed_subscription_events = RelayedSubscriptionEvents::default();

        let mut transaction = self.db_connection.begin_write().unwrap(); //TODO don't unwrap //TODO don't begin_write(), but rather begin_read() ?
        let result = {
            let transaction = PersistentResourcesTransaction {
                db_connection: Mutex::new(&mut transaction), //TODO don't unwrap
                memory: self.memory.clone(),
                relayed_subscription_events: &mut relayed_subscription_events,
            };

            code(transaction).await
        };
        transaction.commit().unwrap(); //TODO don't unwrap

        debug_assert!(relayed_subscription_events.is_empty(), "Read-only storage operations should not trigger any subscription events.");

        result
    }

    pub async fn resources_mut<T, E, F>(&mut self, code: F) -> PersistenceResult<(Result<T, E>, RelayedSubscriptionEvents)>
    where
        F: AsyncFnOnce(PersistentResourcesTransaction) -> Result<T, E>,
        E: Send + Sync + 'static,
    {
        let mut relayed_subscription_events = RelayedSubscriptionEvents::default();
        let mut transaction = self.db_connection.begin_write().unwrap(); //TODO don't unwrap
        let result = {
            let persistent_transaction = PersistentResourcesTransaction {
                db_connection: Mutex::new(&mut transaction),
                memory: self.memory.clone(),
                relayed_subscription_events: &mut relayed_subscription_events,
            };

            code(persistent_transaction).await
        };
        transaction.commit().unwrap(); //TODO don't unwrap

        Ok((result, relayed_subscription_events))
    }
}

impl ResourcesStorageApi for PersistentResourcesStorage {
    fn insert<R>(&mut self, id: R::Id, resource: R) -> PersistenceResult<()>
    where R: Resource + Persistable {
        let mut transaction = self.db_connection.begin_write().unwrap(); //TODO don't unwrap
        let mut storage = Storage {
            db: &mut transaction,
            memory: self.memory.clone(),
        };
        let result = resource.insert(id, &mut storage);
        transaction.commit().unwrap(); //TODO don't unwrap
        result
    }

    fn remove<R>(&mut self, id: R::Id) -> PersistenceResult<Option<R>>
    where R: Resource + Persistable {
        let mut transaction = self.db_connection.begin_write().unwrap(); //TODO don't unwrap
        let mut storage = Storage {
            db: &mut transaction,
            memory: self.memory.clone(),
        };
        let result = R::remove(id, &mut storage);
        transaction.commit().unwrap(); //TODO don't unwrap
        result
    }

    fn get<R>(&self, id: R::Id) -> PersistenceResult<Option<R>>
    where R: Resource + Persistable + Clone {
        let storage = Storage {
            db: &mut self.db_connection.begin_write().unwrap(), //TODO don't unwrap //TODO begin_read()
            memory: self.memory.clone(),
        };
        R::get(id, &storage)
    }

    fn list<R>(&self) -> PersistenceResult<HashMap<R::Id, R>>
    where R: Resource + Persistable + Clone {
        let storage = Storage {
            db: &mut self.db_connection.begin_write().unwrap(), //TODO don't unwrap //TODO begin_read()
            memory: self.memory.clone(),
        };
        R::list(&storage)
    }
}

pub struct PersistentResourcesTransaction<'transaction> {
    db_connection: Mutex<&'transaction mut redb::WriteTransaction>,
    memory: Arc<Mutex<VolatileResourcesStorage>>,
    pub relayed_subscription_events: &'transaction mut RelayedSubscriptionEvents,
}
impl PersistentResourcesTransaction<'_> {
    pub(crate) fn insert<R>(&mut self, id: R::Id, resource: R) -> PersistenceResult<()>
    where R: Resource + Persistable {
        let mut storage = Storage {
            db: &mut self.db_connection.lock().unwrap(),
            memory: self.memory.clone(),
        };
        resource.insert(id, &mut storage)
    }

    pub(crate) fn remove<R>(&mut self, id: R::Id) -> PersistenceResult<Option<R>>
    where R: Resource + Persistable {
        let mut storage = Storage {
            db: &mut self.db_connection.lock().unwrap(),
            memory: self.memory.clone(),
        };
        R::remove(id, &mut storage)
    }

    pub(crate) fn get<R>(&self, id: R::Id) -> PersistenceResult<Option<R>>
    where
        R: Resource + Persistable + Clone
    {
        let mut storage = Storage {
            db: &mut self.db_connection.lock().unwrap(),
            memory: self.memory.clone(),
        };
        R::get(id, &mut storage)
    }

    pub(crate) fn list<R>(&self) -> PersistenceResult<HashMap<R::Id, R>>
    where
        R: Resource + Persistable + Clone
    {
        let mut storage = Storage {
            db: &mut self.db_connection.lock().unwrap(),
            memory: self.memory.clone(),
        };
        R::list(&mut storage)
    }
}

#[derive(Debug, thiserror::Error)]
enum TransactionPassthroughError {
    #[error("Error returned by Diesel while performing transaction.")]
    Diesel(#[from] diesel::result::Error),
    #[error("Error returned by the code executed within the transaction.")]
    Passthrough(Box<dyn Any + Send + Sync>),
}
