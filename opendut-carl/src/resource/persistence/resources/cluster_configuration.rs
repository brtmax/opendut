use super::Persistable;
use crate::resource::persistence::error::PersistenceResult;
use crate::resource::persistence::{Memory, Db, DbMut};
use opendut_types::cluster::{ClusterConfiguration, ClusterId};
use std::collections::HashMap;
use redb::{ReadableTable, TableDefinition};
use uuid::Uuid;

impl Persistable for ClusterConfiguration {
    fn insert(self, cluster_id: ClusterId, _: &mut Memory, db: DbMut) -> PersistenceResult<()> {

        let key = cluster_id.0.as_bytes().as_slice();

        let value = self;
        let value = serde_json::to_string(&value).unwrap(); //TODO don't unwrap

        let mut table = db.open_table(CLUSTER_CONFIGURATION_TABLE).unwrap(); //TODO don't unwrap
        table.insert(key, value).unwrap(); //TODO don't unwrap

        Ok(())
    }

    fn remove(cluster_id: ClusterId, _: &mut Memory, db: DbMut) -> PersistenceResult<Option<Self>> {
        let key = cluster_id.0.as_bytes().as_slice();

        let mut table = db.open_table(CLUSTER_CONFIGURATION_TABLE).unwrap(); //TODO don't unwrap

        let value = table.remove(key).unwrap() //TODO don't unwrap
            .map(|value| {
                serde_json::from_str::<ClusterConfiguration>(&value.value()).unwrap() //TODO don't unwrap
            });

        Ok(value)
    }

    fn get(cluster_id: ClusterId, _: &Memory, db: Db) -> PersistenceResult<Option<Self>> {
        let key = cluster_id.0.as_bytes().as_slice();

        let table = db.open_table(CLUSTER_CONFIGURATION_TABLE).unwrap(); //TODO don't unwrap

        let value = table.get(key).unwrap() //TODO don't unwrap
            .map(|value| {
                serde_json::from_str::<ClusterConfiguration>(&value.value()).unwrap() //TODO don't unwrap
            });

        Ok(value)
    }
    fn get_mut(cluster_id: ClusterId, _: &Memory, db: DbMut) -> PersistenceResult<Option<Self>> {
        let key = cluster_id.0.as_bytes().as_slice();

        let table = db.open_table(CLUSTER_CONFIGURATION_TABLE).unwrap(); //TODO don't unwrap

        let value = table.get(key).unwrap() //TODO don't unwrap
            .map(|value| {
                serde_json::from_str::<ClusterConfiguration>(&value.value()).unwrap() //TODO don't unwrap
            });

        Ok(value)
    }

    fn list(_: &Memory, db: Db) -> PersistenceResult<HashMap<Self::Id, Self>> {
        let table = db.open_table(CLUSTER_CONFIGURATION_TABLE).unwrap(); //TODO don't unwrap

        let value = table.iter().unwrap() //TODO don't unwrap
            .map(|value| {
                let (key, value) = value.unwrap(); //TODO don't unwrap
                let id = ClusterId::from(Uuid::from_slice(key.value()).unwrap()); //TODO don't unwrap

                let value = serde_json::from_str::<ClusterConfiguration>(&value.value()).unwrap(); //TODO don't unwrap

                (id, value)
            })
            .collect();

        Ok(value)
    }
    fn list_mut(_: &Memory, db: DbMut) -> PersistenceResult<HashMap<Self::Id, Self>> {
        let table = db.open_table(CLUSTER_CONFIGURATION_TABLE).unwrap(); //TODO don't unwrap

        let value = table.iter().unwrap() //TODO don't unwrap
            .map(|value| {
                let (key, value) = value.unwrap(); //TODO don't unwrap
                let id = ClusterId::from(Uuid::from_slice(key.value()).unwrap()); //TODO don't unwrap

                let value = serde_json::from_str::<ClusterConfiguration>(&value.value()).unwrap(); //TODO don't unwrap

                (id, value)
            })
            .collect();

        Ok(value)
    }
}

fn list_impl<K: redb::Key, V: redb::Value>(table: impl ReadableTable<K, V>) -> PersistenceResult<HashMap<Self::Id, Self>> {

}

const CLUSTER_CONFIGURATION_TABLE: TableDefinition<&[u8], String> = TableDefinition::new("cluster_configuration");

//TODO SerializableClusterConfiguration + Version-field
