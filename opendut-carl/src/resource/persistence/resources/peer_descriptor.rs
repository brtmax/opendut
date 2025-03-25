use opendut_types::peer::{PeerDescriptor, PeerId};
use std::collections::HashMap;

use super::Persistable;
use crate::resource::persistence::error::PersistenceResult;
use crate::resource::persistence::query;
use crate::resource::persistence::query::Filter;
use crate::resource::storage::{Db, Memory};

impl Persistable for PeerDescriptor {
    async fn insert(self, _peer_id: PeerId, _: &mut Memory, db: &mut Db<'_>) -> PersistenceResult<()> {

        query::peer_descriptor::insert(self, db.connection).await
    }

    async fn remove(peer_id: PeerId, _: &mut Memory, db: &mut Db<'_>) -> PersistenceResult<Option<Self>> {
        query::peer_descriptor::remove(peer_id, db.connection).await
    }

    async fn get(peer_id: PeerId, _: &Memory, db: &mut Db<'_>) -> PersistenceResult<Option<Self>> {
        let result = query::peer_descriptor::list(Filter::By(peer_id), db.connection).await?.values().next().cloned();
        Ok(result)
    }

    async fn list(_: &Memory, db: &mut Db<'_>) -> PersistenceResult<HashMap<Self::Id, Self>> {
        query::peer_descriptor::list(Filter::Not, db.connection).await
    }
}
