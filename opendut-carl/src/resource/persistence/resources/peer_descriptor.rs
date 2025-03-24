use opendut_types::peer::{PeerDescriptor, PeerId};
use std::collections::HashMap;

use super::Persistable;
use crate::resource::persistence::error::PersistenceResult;
use crate::resource::persistence::query::Filter;
use crate::resource::persistence::{query, Storage};

impl Persistable for PeerDescriptor {
    async fn insert(self, _peer_id: PeerId, storage: &mut Storage<'_>) -> PersistenceResult<()> {
        let mut connection = storage.db.connection();

        query::peer_descriptor::insert(self, &mut connection).await
    }

    async fn remove(peer_id: PeerId, storage: &mut Storage<'_>) -> PersistenceResult<Option<Self>> {
        query::peer_descriptor::remove(peer_id, &mut storage.db.connection()).await
    }

    async fn get(peer_id: PeerId, storage: &Storage<'_>) -> PersistenceResult<Option<Self>> {
        let result = query::peer_descriptor::list(Filter::By(peer_id), &mut storage.db.connection()).await?.values().next().cloned();
        Ok(result)
    }

    async fn list(storage: &Storage<'_>) -> PersistenceResult<HashMap<Self::Id, Self>> {
        query::peer_descriptor::list(Filter::Not, &mut storage.db.connection()).await
    }
}
