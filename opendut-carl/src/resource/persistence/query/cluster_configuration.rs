use crate::resource::persistence::database::schema;
use crate::resource::persistence::error::{PersistenceError, PersistenceResult};
use crate::resource::persistence::query;
use crate::resource::persistence::query::cluster_device::PersistableClusterDevice;
use crate::resource::persistence::query::Filter;
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use opendut_types::cluster::{ClusterConfiguration, ClusterId, ClusterName};
use opendut_types::peer::PeerId;
use opendut_types::topology::DeviceId;
use std::collections::{HashMap, HashSet};
use std::ops::Not;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

pub async fn insert(cluster_configuration: ClusterConfiguration, connection: &mut AsyncPgConnection) -> PersistenceResult<()> {
    let ClusterConfiguration { id, name, leader, devices } = cluster_configuration;

    insert_persistable(PersistableClusterConfiguration {
        cluster_id: id.0,
        name: name.value(),
        leader_id: leader.uuid,
    }, connection).await?;

    {
        let previous_cluster_devices = query::cluster_device::list_filtered_by_cluster_id(id, connection).await?;

        for previous_device in previous_cluster_devices {
            if devices.contains(&previous_device.device_id.into()).not() {
                query::cluster_device::remove(previous_device, connection).await?;
            }
        }

        for device in devices {
            query::cluster_device::insert(PersistableClusterDevice {
                cluster_id: id.0,
                device_id: device.0,
            }, connection).await?
        }
    }

    Ok(())
}

#[derive(Clone, Debug, PartialEq, diesel::Queryable, diesel::Selectable, diesel::Insertable, diesel::AsChangeset)]
#[diesel(table_name = schema::cluster_configuration)]
#[diesel(belongs_to(PersistablePeerDescriptor, foreign_key = leader_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct PersistableClusterConfiguration {
    pub cluster_id: Uuid,
    pub name: String,
    pub leader_id: Uuid,
}
async fn insert_persistable(persistable: PersistableClusterConfiguration, connection: &mut AsyncPgConnection) -> PersistenceResult<()> {
    diesel::insert_into(schema::cluster_configuration::table)
        .values(&persistable)
        .on_conflict(schema::cluster_configuration::cluster_id)
        .do_update()
        .set(&persistable)
        .execute(connection).await
        .map_err(|cause| PersistenceError::insert::<ClusterConfiguration>(persistable.cluster_id, cause))?;
    Ok(())
}

pub async fn remove(cluster_id: ClusterId, connection: &mut AsyncPgConnection) -> PersistenceResult<Option<ClusterConfiguration>> {
    let result = list(Filter::By(cluster_id), connection).await?.values().next().cloned();

    diesel::delete(
        schema::cluster_configuration::table
            .filter(schema::cluster_configuration::cluster_id.eq(cluster_id.0))
    )
    .execute(connection).await
    .map_err(|cause| PersistenceError::remove::<ClusterConfiguration>(cluster_id.0, cause))?;

    Ok(result)
}

pub async fn list(filter_by_cluster_id: Filter<ClusterId>, connection: &mut AsyncPgConnection) -> PersistenceResult<HashMap<ClusterId, ClusterConfiguration>> {
    let persistable_cluster_configurations: Vec<PersistableClusterConfiguration> = {
        let mut query = schema::cluster_configuration::table.into_boxed();

        if let Filter::By(cluster_id) = filter_by_cluster_id {
            query = query.filter(schema::cluster_configuration::cluster_id.eq(cluster_id.0));
        }

        query
            .select(PersistableClusterConfiguration::as_select())
            .get_results(connection).await
            .map_err(PersistenceError::list::<ClusterConfiguration>)?
    };


    let mut result = HashMap::new();

    for persistable in persistable_cluster_configurations {
        let PersistableClusterConfiguration { cluster_id, name, leader_id } = persistable;

        let cluster_id = ClusterId::from(cluster_id);

        let name = ClusterName::try_from(name)
            .map_err(|cause|
                PersistenceError::get::<ClusterConfiguration>(cluster_id.0, cause)
                    .context("Listing ClusterConfigurations from persistence.")
            )?;

        let leader_id = PeerId::from(leader_id);

        let devices = query::cluster_device::list_filtered_by_cluster_id(cluster_id, connection).await
            .map_err(|cause| cause.context("Listing ClusterConfigurations from persistence."))?
            .into_iter()
            .map(|cluster_device| DeviceId::from(cluster_device.device_id))
            .collect::<HashSet<_>>();

        result.insert(cluster_id, ClusterConfiguration {
            id: cluster_id,
            name,
            leader: leader_id,
            devices,
        });
    }

    Ok(result)
}
