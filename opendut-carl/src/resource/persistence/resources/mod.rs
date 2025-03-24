use crate::resource::api::Resource;
use crate::resource::persistence::error::PersistenceResult;
use crate::resource::storage::Storage;
use std::collections::HashMap;
use std::fmt::Debug;

pub mod cluster_configuration;
pub mod cluster_deployment;
pub mod old_peer_configuration;
pub mod peer_configuration;
pub mod peer_descriptor;
mod peer_connection_state;

pub(crate) trait Persistable: Send + Sync + Sized + Debug + Resource {
    async fn insert(self, id: Self::Id, storage: &mut Storage) -> PersistenceResult<()>;

    async fn remove(id: Self::Id, storage: &mut Storage) -> PersistenceResult<Option<Self>>;

    async fn get(id: Self::Id, storage: &Storage) -> PersistenceResult<Option<Self>>;

    async fn list(storage: &Storage) -> PersistenceResult<HashMap<Self::Id, Self>>;
}
