use crate::resource::persistence::error::PersistenceResult;
use crate::resource::persistence::resources::Persistable;
use crate::resource::storage::ResourcesStorageApi;
use crate::resource::storage::{Db, Memory};
use opendut_types::peer::state::PeerConnectionState;
use opendut_types::peer::PeerId;
use std::collections::HashMap;

impl Persistable for PeerConnectionState {
    async fn insert(self, id: PeerId, memory: &mut Memory, _: &mut Db<'_>) -> PersistenceResult<()> {
        memory.insert(id, self).await
    }

    async fn remove(id: PeerId, memory: &mut Memory, _: &mut Db<'_>) -> PersistenceResult<Option<Self>> {
        memory.remove(id).await
    }

    async fn get(id: PeerId, memory: &Memory, _: &mut Db<'_>) -> PersistenceResult<Option<Self>> {
        memory.get(id).await
    }

    async fn list(memory: &Memory, _: &mut Db<'_>) -> PersistenceResult<HashMap<Self::Id, Self>> {
        memory.list().await
    }
}
