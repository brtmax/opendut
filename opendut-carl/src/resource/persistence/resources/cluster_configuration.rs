use super::Persistable;
use crate::resource::persistence::error::PersistenceResult;
use crate::resource::persistence::query;
use crate::resource::persistence::query::Filter;
use crate::resource::storage::{Db, Memory};
use opendut_types::cluster::{ClusterConfiguration, ClusterId};
use std::collections::HashMap;

impl Persistable for ClusterConfiguration {
    async fn insert(self, _id: ClusterId, _: &mut Memory, db: &mut Db<'_>) -> PersistenceResult<()> {
        query::cluster_configuration::insert(self, db.connection).await
    }

    async fn remove(cluster_id: ClusterId, _: &mut Memory, db: &mut Db<'_>) -> PersistenceResult<Option<Self>> {
        query::cluster_configuration::remove(cluster_id, db.connection).await
    }

    async fn get(cluster_id: ClusterId, _: &Memory, db: &mut Db<'_>) -> PersistenceResult<Option<Self>> {
        let result = query::cluster_configuration::list(Filter::By(cluster_id), db.connection).await?
            .values().next().cloned();
        Ok(result)
    }

    async fn list(_: &Memory, db: &mut Db<'_>) -> PersistenceResult<HashMap<Self::Id, Self>> {
        query::cluster_configuration::list(Filter::Not, db.connection).await
    }
}
