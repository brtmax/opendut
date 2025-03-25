use crate::resource::persistence::error::{PersistenceError};
use opendut_types::cluster::{ClusterConfiguration, ClusterDeployment};
use std::collections::{HashSet};

pub mod internal {
    use crate::resource::api::resources::Resources;
    use super::*;

    pub(crate) async fn list_deployed_clusters(resources: &mut Resources<'_>) -> Result<Vec<ClusterConfiguration>, PersistenceError> {
        let cluster_deployments = resources.list::<ClusterDeployment>().await?
            .into_values()
            .map(|cluster_deployment| cluster_deployment.id)
            .collect::<HashSet<_>>();

        let cluster_configurations = resources.list::<ClusterConfiguration>().await?;
        let deployed_cluster_configurations = cluster_configurations.into_values()
            .filter(|cluster_configuration| {
                cluster_deployments.contains(&cluster_configuration.id)
            }).collect::<Vec<_>>();
        Ok(deployed_cluster_configurations)
    }
}
