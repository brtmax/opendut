use crate::resource::api::Resource;
use crate::resource::persistence::error::PersistenceResult;
use crate::resource::persistence::{Memory, Db, DbMut};
use std::collections::HashMap;
use std::fmt::Debug;

pub mod cluster_configuration;
pub mod cluster_deployment;
pub mod old_peer_configuration;
pub mod peer_configuration;
pub mod peer_descriptor;
mod peer_connection_state;

pub trait Persistable: Send + Sync + Sized + Debug + Resource {
    fn insert(self, id: Self::Id, memory: &mut Memory, db: DbMut) -> PersistenceResult<()>;

    fn remove(id: Self::Id, memory: &mut Memory, db: DbMut) -> PersistenceResult<Option<Self>>;

    fn get(id: Self::Id, memory: &Memory, db: Db) -> PersistenceResult<Option<Self>>;
    fn get_mut(id: Self::Id, memory: &Memory, db: DbMut) -> PersistenceResult<Option<Self>>;

    fn list(memory: &Memory, db: Db) -> PersistenceResult<HashMap<Self::Id, Self>>;
    fn list_mut(memory: &Memory, db: DbMut) -> PersistenceResult<HashMap<Self::Id, Self>>;
}
