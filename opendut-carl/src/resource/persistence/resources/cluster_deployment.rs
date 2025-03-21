use opendut_types::cluster::{ClusterDeployment, ClusterId};
use std::collections::HashMap;

use crate::resource::persistence::error::PersistenceResult;
use crate::resource::persistence::query::Filter;
use crate::resource::persistence::{query, Storage};

use super::Persistable;

impl Persistable for ClusterDeployment {
    async fn insert(self, _id: ClusterId, storage: &mut Storage<'_>) -> PersistenceResult<()> {
        let mut connection = storage.db.connection();

        query::cluster_deployment::insert(self, &mut connection)
    }

    async fn remove(cluster_id: ClusterId, storage: &mut Storage<'_>) -> PersistenceResult<Option<Self>> {
        query::cluster_deployment::remove(cluster_id, &mut storage.db.connection())
    }

    async fn get(cluster_id: ClusterId, storage: &Storage<'_>) -> PersistenceResult<Option<Self>> {
        let result = query::cluster_deployment::list(Filter::By(cluster_id), &mut storage.db.connection())?.values().next().cloned();
        Ok(result)
    }

    async fn list(storage: &Storage<'_>) -> PersistenceResult<HashMap<Self::Id, Self>> {
        query::cluster_deployment::list(Filter::Not, &mut storage.db.connection())
    }
}
