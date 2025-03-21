use crate::resource::persistence::error::PersistenceResult;
use crate::resource::persistence::resources::Persistable;
use crate::resource::persistence::Storage;
use crate::resource::storage::ResourcesStorageApi;
use opendut_types::peer::state::PeerConnectionState;
use opendut_types::peer::PeerId;
use std::collections::HashMap;

impl Persistable for PeerConnectionState {
    async fn insert(self, id: PeerId, storage: &mut Storage<'_>) -> PersistenceResult<()> {
        storage.memory.insert(id, self).await
    }

    async fn remove(id: PeerId, storage: &mut Storage<'_>) -> PersistenceResult<Option<Self>> {
        storage.memory.remove(id).await
    }

    async fn get(id: PeerId, storage: &Storage<'_>) -> PersistenceResult<Option<Self>> {
        storage.memory.get(id).await
    }

    async fn list(storage: &Storage<'_>) -> PersistenceResult<HashMap<Self::Id, Self>> {
        storage.memory.list().await
    }
}
