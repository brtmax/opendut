use crate::resource::api::resources::RelayedSubscriptionEvents;
use crate::resource::persistence::database::ConnectError;
use crate::resource::persistence::error::{PersistenceError, PersistenceResult};
use crate::resource::persistence::resources::Persistable;
use crate::resource::storage::volatile::VolatileResourcesStorage;
use crate::resource::storage::{DatabaseConnectInfo, Db, Memory, Resource, ResourcesStorageApi, Storage};
use diesel_async::pooled_connection::bb8::Pool;
use diesel_async::{AsyncConnection, AsyncPgConnection};
use std::any::Any;
use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

pub struct PersistentResourcesStorage {
    db_connection_pool: Pool<AsyncPgConnection>,
    memory: Mutex<VolatileResourcesStorage>,
}
impl PersistentResourcesStorage {
    pub async fn connect(database_connect_info: &DatabaseConnectInfo) -> Result<Self, ConnectError> {
        let db_connection_pool = crate::resource::persistence::database::connection_pool(database_connect_info).await?;
        let memory = VolatileResourcesStorage::default();
        let memory = Mutex::new(memory);

        Ok(Self { db_connection_pool, memory })
    }

    pub async fn resources<T, F>(&self, code: F) -> T
    where
        F: AsyncFnOnce(PersistentResourcesTransaction) -> T,
    {
        let mut connection = self.db_connection_pool.get().await
            .expect("Could not retrieve connection from connection pool."); //FIXME proper error handling

        let mut memory = self.memory.lock().unwrap();
        let mut relayed_subscription_events = RelayedSubscriptionEvents::default();

        let transaction = PersistentResourcesTransaction {
            db_connection: Arc::new(Mutex::new(connection.deref_mut())),
            memory: Mutex::new(&mut memory),
            relayed_subscription_events: &mut relayed_subscription_events,
        };

        let result = code(transaction).await;

        debug_assert!(relayed_subscription_events.is_empty(), "Read-only storage operations should not trigger any subscription events.");

        result
    }

    pub async fn resources_mut<T, E, F>(&mut self, code: F) -> PersistenceResult<(Result<T, E>, RelayedSubscriptionEvents)>
    where
        F: AsyncFnOnce(PersistentResourcesTransaction) -> Result<T, E> + Send,
        T: Send,
        E: Send + Sync + 'static,
    {
        let mut connection = self.db_connection_pool.get().await?;

        let transaction_result = {
            connection.transaction::<_, TransactionPassthroughError, _>(|connection| Box::pin(async {
                let mut memory = self.memory.lock().unwrap();
                let mut relayed_subscription_events = RelayedSubscriptionEvents::default();

                let transaction = PersistentResourcesTransaction {
                    db_connection: Arc::new(Mutex::new(connection)),
                    memory: Arc::new(Mutex::new(&mut memory)),
                    relayed_subscription_events: &mut relayed_subscription_events,
                };

                let result = code(transaction).await;
                match result {
                    Ok(ok) => Ok((ok, relayed_subscription_events)),
                    Err(error) => Err(TransactionPassthroughError::Passthrough(Box::new(error))), //passthrough via an Err-value to trigger transaction rollback
                }
            }))
        };

        let transaction_result = transaction_result.await;

        match transaction_result {
            Ok((result, relayed_subscription_events)) => Ok((Ok(result), relayed_subscription_events)),
            Err(TransactionPassthroughError::Passthrough(error)) => {
                let error = error.downcast::<E>()
                    .expect("should be error of type E, like we handed it out from the transaction");
                Ok((Err(*error), RelayedSubscriptionEvents::default())) //FIXME can we omit RelayedSubscriptionEvents at compile-time?
            }
            Err(TransactionPassthroughError::Diesel(source)) => Err(PersistenceError::DieselInternal { source }),
        }
    }
}
// impl ResourcesStorageApi for PersistentResourcesStorage { //TODO remove?
//     async fn insert<R>(&mut self, id: R::Id, resource: R) -> PersistenceResult<()>
//     where R: Resource + Persistable {
//         let mut db = self.db_connection_pool.get().await?;
//         let db = Db::from_connection(&mut db);
//         let mut storage = Storage { db, memory: &mut self.memory.lock().unwrap() };
//         resource.insert(id, &mut storage).await
//     }
// 
//     async fn remove<R>(&mut self, id: R::Id) -> PersistenceResult<Option<R>>
//     where R: Resource + Persistable {
//         let mut db = self.db_connection_pool.get().await?;
//         let db = Db::from_connection(&mut db);
//         let mut storage = Storage { db, memory: &mut self.memory.lock().unwrap() };
//         R::remove(id, &mut storage).await
//     }
// 
//     async fn get<R>(&self, id: R::Id) -> PersistenceResult<Option<R>>
//     where R: Resource + Persistable + Clone {
//         let mut db = self.db_connection_pool.get().await?;
//         let db = Db::from_connection(&mut db);
//         let storage = Storage { db, memory: &mut self.memory.lock().unwrap() };
//         R::get(id, &storage).await
//     }
// 
//     async fn list<R>(&self) -> PersistenceResult<HashMap<R::Id, R>>
//     where R: Resource + Persistable + Clone {
//         let mut db = self.db_connection_pool.get().await?;
//         let db = Db::from_connection(&mut db);
//         let storage = Storage { db, memory: &mut self.memory.lock().unwrap() };
//         R::list(&storage).await
//     }
// }


pub struct PersistentResourcesTransaction<'transaction> {
    db_connection: Db<'transaction>,
    memory: Memory<'transaction>,
    pub relayed_subscription_events: &'transaction mut RelayedSubscriptionEvents,
}
impl ResourcesStorageApi for PersistentResourcesTransaction<'_> {
    async fn insert<R>(&mut self, id: R::Id, resource: R) -> PersistenceResult<()>
    where R: Resource + Persistable {
        let mut storage = Storage {
            db: self.db_connection.clone(),
            memory: self.memory.clone(),
        };
        resource.insert(id, &mut storage).await
    }

    async fn remove<R>(&mut self, id: R::Id) -> PersistenceResult<Option<R>>
    where R: Resource + Persistable {
        let mut storage = Storage {
            db: self.db_connection.clone(),
            memory: self.memory.clone(),
        };
        R::remove(id, &mut storage).await
    }

    async fn get<R>(&self, id: R::Id) -> PersistenceResult<Option<R>>
    where
        R: Resource + Persistable + Clone
    {
        let storage = Storage {
            db: self.db_connection.clone(),
            memory: self.memory.clone(),
        };
        R::get(id, &storage).await
    }

    async fn list<R>(&self) -> PersistenceResult<HashMap<R::Id, R>>
    where
        R: Resource + Persistable + Clone
    {
        let storage = Storage {
            db: self.db_connection.clone(),
            memory: self.memory.clone(),
        };
        R::list(&storage).await
    }
}

#[derive(Debug, thiserror::Error)]
enum TransactionPassthroughError {
    #[error("Error returned by Diesel while performing transaction.")]
    Diesel(#[from] diesel::result::Error),
    #[error("Error returned by the code executed within the transaction.")]
    Passthrough(Box<dyn Any + Send + Sync>),
}
