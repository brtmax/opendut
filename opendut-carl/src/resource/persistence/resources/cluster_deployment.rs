use opendut_types::cluster::{ClusterDeployment, ClusterId};
use std::collections::HashMap;

use super::Persistable;
use crate::resource::persistence::error::PersistenceResult;
use crate::resource::persistence::query;
use crate::resource::persistence::query::Filter;
use crate::resource::storage::{Db, Memory};

impl Persistable for ClusterDeployment {
    async fn insert(self, _id: ClusterId, _: &mut Memory, db: &mut Db<'_>) -> PersistenceResult<()> {
        query::cluster_deployment::insert(self, db.connection).await
    }

    async fn remove(cluster_id: ClusterId, _: &mut Memory, db: &mut Db<'_>) -> PersistenceResult<Option<Self>> {
        query::cluster_deployment::remove(cluster_id, db.connection).await
    }

    async fn get(cluster_id: ClusterId, _: &Memory, db: &mut Db<'_>) -> PersistenceResult<Option<Self>> {
        let result = query::cluster_deployment::list(Filter::By(cluster_id), db.connection).await?.values().next().cloned();
        Ok(result)
    }

    async fn list(_: &Memory, db: &mut Db<'_>) -> PersistenceResult<HashMap<Self::Id, Self>> {
        query::cluster_deployment::list(Filter::Not, db.connection).await
    }
}
