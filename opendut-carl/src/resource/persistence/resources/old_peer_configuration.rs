use opendut_types::peer::configuration::OldPeerConfiguration;
use opendut_types::peer::PeerId;
use std::collections::HashMap;

use crate::resource::persistence::error::PersistenceResult;
use crate::resource::persistence::resources::Persistable;
use crate::resource::persistence::{Storage, Storage2};
use crate::resource::storage::ResourcesStorageApi;

impl Persistable for OldPeerConfiguration {
    fn insert(self, id: PeerId, storage: &mut Storage) -> PersistenceResult<()> {
        storage.memory.insert(id, self)
    }

    fn remove(id: PeerId, storage: &mut Storage) -> PersistenceResult<Option<Self>> {
        storage.memory.remove(id)
    }

    fn get(id: PeerId, storage: &Storage) -> PersistenceResult<Option<Self>> {
        storage.memory.get(id)
    }
    
    fn list(storage: &Storage) -> PersistenceResult<HashMap<Self::Id, Self>> {
        storage.memory.list()
    }

    fn insert2(self, id: Self::Id, storage: &mut Storage2) -> PersistenceResult<()> {
        todo!()
    }

    fn remove2(id: Self::Id, storage: &mut Storage2) -> PersistenceResult<Option<Self>> {
        todo!()
    }

    fn get2(id: Self::Id, storage: &Storage2) -> PersistenceResult<Option<Self>> {
        todo!()
    }

    fn list2(storage: &Storage2) -> PersistenceResult<HashMap<Self::Id, Self>> {
        todo!()
    }
}
