use super::Persistable;
use crate::resource::persistence::error::PersistenceResult;
use crate::resource::persistence::query::Filter;
use crate::resource::persistence::{query, Storage};
use opendut_types::cluster::{ClusterConfiguration, ClusterId};
use std::collections::HashMap;

impl Persistable for ClusterConfiguration {
    async fn insert(self, _id: ClusterId, storage: &mut Storage<'_>) -> PersistenceResult<()> {
        let mut connection = storage.db.connection();

        query::cluster_configuration::insert(self, &mut connection).await
    }

    async fn remove(cluster_id: ClusterId, storage: &mut Storage<'_>) -> PersistenceResult<Option<Self>> {
        query::cluster_configuration::remove(cluster_id, &mut storage.db.connection()).await
    }

    async fn get(cluster_id: ClusterId, storage: &Storage<'_>) -> PersistenceResult<Option<Self>> {
        let result = query::cluster_configuration::list(Filter::By(cluster_id), &mut storage.db.connection()).await?
            .values().next().cloned();
        Ok(result)
    }

    async fn list(storage: &Storage<'_>) -> PersistenceResult<HashMap<Self::Id, Self>> {
        query::cluster_configuration::list(Filter::Not, &mut storage.db.connection()).await
    }
}
