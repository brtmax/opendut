use crate::resource::api::Resource;
use crate::resource::persistence::error::PersistenceResult;
use crate::resource::storage::{Db, Memory};
use std::collections::HashMap;
use std::fmt::Debug;

pub mod cluster_configuration;
pub mod cluster_deployment;
pub mod old_peer_configuration;
pub mod peer_configuration;
pub mod peer_descriptor;
mod peer_connection_state;

pub(crate) trait Persistable: Send + Sync + Sized + Debug + Resource {
    async fn insert(self, id: Self::Id, memory: &mut Memory, db: &mut Db) -> PersistenceResult<()>;

    async fn remove(id: Self::Id, memory: &mut Memory, db: &mut Db) -> PersistenceResult<Option<Self>>;

    async fn get(id: Self::Id, memory: &Memory, db: &mut Db) -> PersistenceResult<Option<Self>>;

    async fn list(memory: &Memory, db: &mut Db) -> PersistenceResult<HashMap<Self::Id, Self>>;
}
