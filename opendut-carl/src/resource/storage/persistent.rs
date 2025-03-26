use crate::resource::api::resources::RelayedSubscriptionEvents;
use crate::resource::persistence::database::ConnectError;
use crate::resource::persistence::error::{PersistenceError, PersistenceResult};
use crate::resource::persistence::resources::{Persistable};
use crate::resource::persistence::{Db, Storage, Storage2};
use crate::resource::storage::volatile::VolatileResourcesStorage;
use crate::resource::storage::{DatabaseConnectInfo, Resource, ResourcesStorageApi};
use diesel::{Connection, PgConnection};
use std::any::Any;
use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

pub struct PersistentResourcesStorage {
    db_connection: Mutex<PgConnection>,
    memory: Arc<Mutex<VolatileResourcesStorage>>,
    db: redb::Database,
}
impl PersistentResourcesStorage {
    pub async fn connect(database_connect_info: &DatabaseConnectInfo) -> Result<Self, ConnectError> {
        let db_connection = crate::resource::persistence::database::connect(database_connect_info).await?;
        let db_connection = Mutex::new(db_connection);
        let memory = VolatileResourcesStorage::default();
        let memory = Arc::new(Mutex::new(memory));

        let db = redb::Database::create("opendut.redb").unwrap(); //TODO don't unwrap //TODO make name configurable and set it to a temporary path during tests

        Ok(Self { db_connection, memory, db })
    }

    pub async fn resources<T, F>(&self, code: F) -> T
    where
        F: AsyncFnOnce(PersistentResourcesTransaction) -> T,
    {
        let mut relayed_subscription_events = RelayedSubscriptionEvents::default();

        let mut transaction = self.db.begin_write().unwrap(); //TODO don't unwrap
        let result = {
            let transaction = PersistentResourcesTransaction {
                // db_connection: Mutex::new(connection.deref_mut()),
                db_connection: Mutex::new(&mut transaction), //TODO don't unwrap //TODO don't begin_write(), but rather begin_read() ?
                memory: self.memory.clone(),
                relayed_subscription_events: &mut relayed_subscription_events,
            };

            code(transaction).await
        };
        transaction.commit().unwrap(); //TODO don't unwrap

        debug_assert!(relayed_subscription_events.is_empty(), "Read-only storage operations should not trigger any subscription events.");

        result
    }

    // pub async fn resources_mut<T, E, F>(&mut self, code: F) -> PersistenceResult<(Result<T, E>, RelayedSubscriptionEvents)>
    // where
    //     F: AsyncFnOnce(PersistentResourcesTransaction) -> Result<T, E>,
    //     E: Send + Sync + 'static,
    // {
    //     let transaction_result = self.db_connection.lock().unwrap().transaction::<_, TransactionPassthroughError, _>(|connection| {
    //         let mut memory = self.memory.lock().unwrap();
    //         let mut relayed_subscription_events = RelayedSubscriptionEvents::default();
    //
    //         let transaction = PersistentResourcesTransaction {
    //             db_connection: Mutex::new(connection),
    //             memory: Mutex::new(&mut memory),
    //             relayed_subscription_events: &mut relayed_subscription_events,
    //         };
    //
    //         let result = futures::executor::block_on(code(transaction));
    //         match result {
    //             Ok(ok) => Ok((ok, relayed_subscription_events)),
    //             Err(error) => Err(TransactionPassthroughError::Passthrough(Box::new(error))), //passthrough via an Err-value to trigger transaction rollback
    //         }
    //     });
    //
    //     match transaction_result {
    //         Ok((result, relayed_subscription_events)) => Ok((Ok(result), relayed_subscription_events)),
    //         Err(TransactionPassthroughError::Passthrough(error)) => {
    //             let error = error.downcast::<E>()
    //                 .expect("should be error of type E, like we handed it out from the transaction");
    //             Ok((Err(*error), RelayedSubscriptionEvents::default())) //FIXME can we omit RelayedSubscriptionEvents at compile-time?
    //         }
    //         Err(TransactionPassthroughError::Diesel(source)) => Err(PersistenceError::DieselInternal { source }),
    //     }
    // }

    pub async fn resources_mut<T, E, F>(&mut self, code: F) -> PersistenceResult<(Result<T, E>, RelayedSubscriptionEvents)>
    where
        F: AsyncFnOnce(PersistentResourcesTransaction) -> Result<T, E>,
        E: Send + Sync + 'static,
    {
        let mut relayed_subscription_events = RelayedSubscriptionEvents::default();
        let mut transaction = self.db.begin_write().unwrap(); //TODO don't unwrap
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
        let mut db = self.db_connection.lock().unwrap();
        let db = Db::from_connection(&mut db);
        let mut storage = Storage { db, memory: &mut self.memory.lock().unwrap() };
        resource.insert(id, &mut storage)
    }

    fn remove<R>(&mut self, id: R::Id) -> PersistenceResult<Option<R>>
    where R: Resource + Persistable {
        let mut db = self.db_connection.lock().unwrap();
        let db = Db::from_connection(&mut db);
        let mut storage = Storage { db, memory: &mut self.memory.lock().unwrap() };
        R::remove(id, &mut storage)
    }

    fn get<R>(&self, id: R::Id) -> PersistenceResult<Option<R>>
    where R: Resource + Persistable + Clone {
        let mut db = self.db_connection.lock().unwrap();
        let db = Db::from_connection(&mut db);
        let storage = Storage { db, memory: &mut self.memory.lock().unwrap() };
        R::get(id, &storage)
    }

    fn list<R>(&self) -> PersistenceResult<HashMap<R::Id, R>>
    where R: Resource + Persistable + Clone {
        let mut db = self.db_connection.lock().unwrap();
        let db = Db::from_connection(&mut db);
        let storage = Storage { db, memory: &mut self.memory.lock().unwrap() };
        R::list(&storage)
    }
}


// pub struct PersistentResourcesTransaction<'transaction> {
//     db_connection: Mutex<&'transaction mut PgConnection>,
//     memory: Mutex<&'transaction mut VolatileResourcesStorage>,
//     pub relayed_subscription_events: &'transaction mut RelayedSubscriptionEvents,
// }
// impl ResourcesStorageApi for PersistentResourcesTransaction<'_> {
//     fn insert<R>(&mut self, id: R::Id, resource: R) -> PersistenceResult<()>
//     where R: Resource + Persistable {
//         let mut db = self.db_connection.lock().unwrap();
//         let db = Db::from_connection(&mut db);
//         let mut storage = Storage { db, memory: &mut self.memory.lock().unwrap() };
//         resource.insert(id, &mut storage)
//     }
//
//     fn remove<R>(&mut self, id: R::Id) -> PersistenceResult<Option<R>>
//     where R: Resource + Persistable {
//         let mut db = self.db_connection.lock().unwrap();
//         let db = Db::from_connection(&mut db);
//         let mut storage = Storage { db, memory: &mut self.memory.lock().unwrap() };
//         R::remove(id, &mut storage)
//     }
//
//     fn get<R>(&self, id: R::Id) -> PersistenceResult<Option<R>>
//     where
//         R: Resource + Persistable + Clone
//     {
//         let mut db = self.db_connection.lock().unwrap();
//         let db = Db::from_connection(&mut db);
//         let storage = Storage { db, memory: &mut self.memory.lock().unwrap() };
//         R::get(id, &storage)
//     }
//
//     fn list<R>(&self) -> PersistenceResult<HashMap<R::Id, R>>
//     where
//         R: Resource + Persistable + Clone
//     {
//         let mut db = self.db_connection.lock().unwrap();
//         let db = Db::from_connection(&mut db);
//         let storage = Storage { db, memory: &mut self.memory.lock().unwrap() };
//         R::list(&storage)
//     }
// }


pub struct PersistentResourcesTransaction<'transaction> {
    db_connection: Mutex<&'transaction mut redb::WriteTransaction>,
    memory: Arc<Mutex<VolatileResourcesStorage>>,
    pub relayed_subscription_events: &'transaction mut RelayedSubscriptionEvents,
}
impl PersistentResourcesTransaction<'_> {
    pub(crate) fn insert<R>(&mut self, id: R::Id, resource: R) -> PersistenceResult<()>
    where R: Resource + Persistable {
        let mut storage = Storage2 {
            db: &mut self.db_connection.lock().unwrap(),
            memory: self.memory.clone(),
        };
        resource.insert2(id, &mut storage)
    }

    pub(crate) fn remove<R>(&mut self, id: R::Id) -> PersistenceResult<Option<R>>
    where R: Resource + Persistable {
        let mut storage = Storage2 {
            db: &mut self.db_connection.lock().unwrap(),
            memory: self.memory.clone(),
        };
        R::remove2(id, &mut storage)
    }

    pub(crate) fn get<R>(&self, id: R::Id) -> PersistenceResult<Option<R>>
    where
        R: Resource + Persistable + Clone
    {
        let mut storage = Storage2 {
            db: &mut self.db_connection.lock().unwrap(),
            memory: self.memory.clone(),
        };
        R::get2(id, &mut storage)
    }

    pub(crate) fn list<R>(&self) -> PersistenceResult<HashMap<R::Id, R>>
    where
        R: Resource + Persistable + Clone
    {
        let mut storage = Storage2 {
            db: &mut self.db_connection.lock().unwrap(),
            memory: self.memory.clone(),
        };
        R::list2(&mut storage)
    }
}

#[derive(Debug, thiserror::Error)]
enum TransactionPassthroughError {
    #[error("Error returned by Diesel while performing transaction.")]
    Diesel(#[from] diesel::result::Error),
    #[error("Error returned by the code executed within the transaction.")]
    Passthrough(Box<dyn Any + Send + Sync>),
}
