use opendut_types::cluster::{ClusterDeployment, ClusterId};
use std::collections::HashMap;

use crate::resource::persistence::error::PersistenceResult;
use crate::resource::persistence::query::Filter;
use crate::resource::persistence::{query, Storage, Storage2};

use super::Persistable;

impl Persistable for ClusterDeployment {
    fn insert(self, _id: ClusterId, storage: &mut Storage) -> PersistenceResult<()> {
        let mut connection = storage.db.connection();

        query::cluster_deployment::insert(self, &mut connection)
    }

    fn remove(cluster_id: ClusterId, storage: &mut Storage) -> PersistenceResult<Option<Self>> {
        query::cluster_deployment::remove(cluster_id, &mut storage.db.connection())
    }

    fn get(cluster_id: ClusterId, storage: &Storage) -> PersistenceResult<Option<Self>> {
        let result = query::cluster_deployment::list(Filter::By(cluster_id), &mut storage.db.connection())?.values().next().cloned();
        Ok(result)
    }

    fn list(storage: &Storage) -> PersistenceResult<HashMap<Self::Id, Self>> {
        query::cluster_deployment::list(Filter::Not, &mut storage.db.connection())
    }

    fn insert2(self, id: Self::Id, storage: &mut Storage2) -> PersistenceResult<()> {
        todo!()
    }

    fn remove2(id: Self::Id, storage: &mut Storage2) -> PersistenceResult<Option<Self>> {
        todo!()
    }

    fn get2(id: Self::Id, storage: &Storage2) -> PersistenceResult<Option<Self>> {
        todo!()
    }

    fn list2(storage: &Storage2) -> PersistenceResult<HashMap<Self::Id, Self>> {
        todo!()
    }
}
