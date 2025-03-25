use opendut_types::resources::Id;
use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::resource::api::id::ResourceId;
use crate::resource::api::resources::RelayedSubscriptionEvents;
use crate::resource::api::Resource;
use crate::resource::persistence::error::PersistenceResult;
use crate::resource::persistence::resources::Persistable;
use crate::resource::storage::{Memory, ResourcesStorageApi};
use crate::resource::subscription::Subscribable;

#[derive(Default)]
pub struct VolatileResourcesStorageHandle {
    memory: Memory,
}
impl VolatileResourcesStorageHandle {
    pub async fn resources<T, F>(&mut self, code: F) -> T
    where
        F: AsyncFnOnce(VolatileResourcesTransaction) -> T,
    {
        let mut relayed_subscription_events = RelayedSubscriptionEvents::default();

        let transaction = VolatileResourcesTransaction {
            memory: &mut self.memory,
            relayed_subscription_events: &mut relayed_subscription_events,
        };
        let result = code(transaction).await;

        debug_assert!(relayed_subscription_events.is_empty(), "Read-only storage operations should not trigger any subscription events.");

        result
    }

    pub async fn resources_mut<T, E, F>(&mut self, code: F) -> PersistenceResult<(Result<T, E>, RelayedSubscriptionEvents)>
    where
        F: AsyncFnOnce(VolatileResourcesTransaction) -> Result<T, E>,
        E: Send + Sync + 'static,
    {
        let mut relayed_subscription_events = RelayedSubscriptionEvents::default();

        let transaction = VolatileResourcesTransaction {
            memory: &mut self.memory,
            relayed_subscription_events: &mut relayed_subscription_events,
        };
        let result = code(transaction).await;
        Ok((result, relayed_subscription_events))
    }
}

#[derive(Default)]
pub struct VolatileResourcesStorage {
    storage: HashMap<TypeId, HashMap<Id, Box<dyn Any + Send + Sync>>>,
}

impl ResourcesStorageApi for Memory {

    async fn insert<R>(&mut self, id: R::Id, resource: R) -> PersistenceResult<()>
    where R: Resource {
        let mut memory = self.lock().unwrap();

        let id = id.into_id();
        let column = memory.storage
            .entry(TypeId::of::<R>())
            .or_default();
        column.insert(id, Box::new(resource));
        Ok(())
    }

    async fn remove<R>(&mut self, id: R::Id) -> PersistenceResult<Option<R>>
    where R: Resource {
        let mut memory = self.lock().unwrap();

        let id = id.into_id();
        let type_id = TypeId::of::<R>();
        match memory.storage.get_mut(&TypeId::of::<R>()) {
            None => Ok(None),
            Some(column) => {
                let result = column.remove(&id)
                    .and_then(|old_value| old_value
                        .downcast()
                        .map(|value| *value)
                        .ok()
                    );
                if column.is_empty() {
                    memory.storage.remove(&type_id);
                }
                Ok(result)
            }
        }
    }

    async fn get<R>(&self, id: R::Id) -> PersistenceResult<Option<R>>
    where R: Resource + Clone {
        let memory = self.lock().unwrap();

        let id = id.into_id();

        let result = memory.storage.get(&TypeId::of::<R>())
            .and_then(|column| column.get(&id))
            .and_then(|resource| resource.downcast_ref().cloned());
        Ok(result)
    }

    async fn list<R>(&self) -> PersistenceResult<HashMap<R::Id, R>>
    where R: Resource {
        let memory = self.lock().unwrap();

        let result: HashMap<R::Id, R> = match memory.storage.get(&TypeId::of::<R>()) {
            Some(column) => {
                column.iter().map(|(value_id, value)| {
                    let resource = value
                        .downcast_ref::<R>()
                        .cloned()
                        .expect("It should always be possible to cast the stored data back to its own type while building an iterator.");
                    let resource_id = R::Id::from_id(*value_id);
                    (resource_id, resource)
                })
                .collect::<HashMap<_, _>>()
            }
            None => HashMap::new()
        };
        Ok(result)
    }
}

#[cfg(test)]
impl VolatileResourcesStorageHandle {
    pub async fn contains<R>(&self, id: R::Id) -> bool
    where R: Resource {
        let memory = self.memory.lock().unwrap();

        let id = id.into_id();
        if let Some(column) = memory.storage.get(&TypeId::of::<R>()) {
            column.contains_key(&id)
        }
        else {
            false
        }
    }

    pub fn is_empty(&self) -> bool {
        let memory = self.memory.lock().unwrap();
        memory.storage.is_empty()
    }
}

pub struct VolatileResourcesTransaction<'transaction> {
    memory: &'transaction mut Memory,
    pub relayed_subscription_events: &'transaction mut RelayedSubscriptionEvents,
}
impl ResourcesStorageApi for VolatileResourcesTransaction<'_> {
    async fn insert<R>(&mut self, id: R::Id, resource: R) -> PersistenceResult<()>
    where R: Resource + Persistable + Subscribable {
        self.memory.insert(id, resource).await
    }

    async fn remove<R>(&mut self, id: R::Id) -> PersistenceResult<Option<R>>
    where R: Resource + Persistable {
        self.memory.remove(id).await
    }

    async fn get<R>(&self, id: R::Id) -> PersistenceResult<Option<R>>
    where R: Resource + Persistable + Clone {
        self.memory.get(id).await
    }

    async fn list<R>(&self) -> PersistenceResult<HashMap<R::Id, R>>
    where R: Resource + Persistable + Clone {
        self.memory.list().await
    }
}
