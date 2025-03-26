use super::Persistable;
use crate::resource::persistence::error::PersistenceResult;
use crate::resource::persistence::query::Filter;
use crate::resource::persistence::{query, Storage, Storage2};
use opendut_types::cluster::{ClusterConfiguration, ClusterId};
use std::collections::HashMap;
use redb::{ReadableTable, TableDefinition};
use uuid::Uuid;

impl Persistable for ClusterConfiguration {
    fn insert(self, _id: ClusterId, storage: &mut Storage) -> PersistenceResult<()> {
        let mut connection = storage.db.connection();

        query::cluster_configuration::insert(self, &mut connection)
    }

    fn remove(cluster_id: ClusterId, storage: &mut Storage) -> PersistenceResult<Option<Self>> {
        query::cluster_configuration::remove(cluster_id, &mut storage.db.connection())
    }

    fn get(cluster_id: ClusterId, storage: &Storage) -> PersistenceResult<Option<Self>> {
        let result = query::cluster_configuration::list(Filter::By(cluster_id), &mut storage.db.connection())?.values().next().cloned();
        Ok(result)
    }

    fn list(storage: &Storage) -> PersistenceResult<HashMap<Self::Id, Self>> {
        query::cluster_configuration::list(Filter::Not, &mut storage.db.connection())
    }


    fn insert2(self, cluster_id: ClusterId, storage: &mut Storage2) -> PersistenceResult<()> {
        let mut table = storage.db.open_table(CLUSTER_CONFIGURATION_TABLE).unwrap(); //TODO don't unwrap

        let key = cluster_id.0.as_bytes().as_slice();

        // let value = { //TODO
        //     let PeerDescriptor { id, name, location, network, topology, executors } = self;
        //     SerializablePeerDescriptor {
        //         id, name, location, network, topology, executors,
        //     }
        // };
        let value = self;
        let value = serde_json::to_string(&value).unwrap(); //TODO don't unwrap

        table.insert(key, value).unwrap(); //TODO don't unwrap

        Ok(())
    }

    fn remove2(cluster_id: ClusterId, storage: &mut Storage2) -> PersistenceResult<Option<Self>> {
        let mut table = storage.db.open_table(CLUSTER_CONFIGURATION_TABLE).unwrap(); //TODO don't unwrap

        let key = cluster_id.0.as_bytes().as_slice();

        let value = table.remove(key).unwrap() //TODO don't unwrap
            .map(|value| {
                let value = serde_json::from_str::<ClusterConfiguration>(&value.value()).unwrap(); //TODO don't unwrap

                // let value = {
                //     let SerializablePeerDescriptor { id, name, location, network, topology, executors } = peer_descriptor;
                //     PeerDescriptor { id, name, location, network, topology, executors }
                // };
                value
            });

        Ok(value)
    }

    fn get2(cluster_id: ClusterId, storage: &Storage2) -> PersistenceResult<Option<Self>> {
        let table = storage.db.open_table(CLUSTER_CONFIGURATION_TABLE).unwrap(); //TODO don't unwrap

        let key = cluster_id.0.as_bytes().as_slice();

        let value = table.get(key).unwrap() //TODO don't unwrap
            .map(|value| {
                let value = serde_json::from_str::<ClusterConfiguration>(&value.value()).unwrap(); //TODO don't unwrap

                // let value = {
                //     let SerializablePeerDescriptor { id, name, location, network, topology, executors } = value;
                //     PeerDescriptor { id, name, location, network, topology, executors }
                // };
                value
            });

        Ok(value)
    }

    fn list2(storage: &Storage2) -> PersistenceResult<HashMap<Self::Id, Self>> {
        let table = storage.db.open_table(CLUSTER_CONFIGURATION_TABLE).unwrap(); //TODO don't unwrap

        let value = table.iter().unwrap() //TODO don't unwrap
            .map(|value| {
                let (key, value) = value.unwrap(); //TODO don't unwrap
                let id = ClusterId::from(Uuid::from_slice(key.value()).unwrap()); //TODO don't unwrap

                let value = serde_json::from_str::<ClusterConfiguration>(&value.value()).unwrap(); //TODO don't unwrap
                // let value = {
                //     let SerializablePeerDescriptor { id, name, location, network, topology, executors } = value;
                //     PeerDescriptor { id, name, location, network, topology, executors }
                // };

                (id, value)
            })
            .collect();

        Ok(value)
    }
}

const CLUSTER_CONFIGURATION_TABLE: TableDefinition<&[u8], String> = TableDefinition::new("cluster_configuration");

//TODO SerializableClusterConfiguration + Version-field
