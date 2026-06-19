use crate::constants::NODE_CONFIG_KEY;
use crate::reconcilers::key_node::KeyFullnodeComponent;
use crate::reconcilers::seed_peers::SeedPeersComponent;
use crate::support::components::WorkerOutputComponent;
use crate::support::extensions::{HeadlessServiceExt, ReadyReplicasExt, SingletonStatefulSetExt};
use crate::support::yamls;
use crate::{crds::EchBoardNetwork, error::Result};
use ech_board_common::keys::{KEYS, SEED_PEERS};
use ech_k8s::{Component, CrMeta, K8sClient, NodeState, Reconciler, ResourcesExt, StoreExt};
use k8s_openapi::api::apps::v1::StatefulSet;
use k8s_openapi::api::core::v1::{ConfigMap, ContainerPort, Secret, Service};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Serialize, Component)]
#[component(name = "workload-fullnode")]
pub(crate) struct WorkloadFullnodeComponent {
    pub(crate) ordinal: usize,
}

#[derive(Clone, Serialize)]
pub(crate) struct WorkloadFullnodeReconciler {
    pub(crate) ordinal: usize,
}

#[async_trait::async_trait]
impl Reconciler for WorkloadFullnodeReconciler {
    type Crd = EchBoardNetwork;
    type Error = crate::error::OperatorError;

    async fn reconcile(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<NodeState> {
        let network_name = network.cr_name()?;
        let namespace = network.cr_ns()?;
        let instance_name = WorkloadFullnodeComponent {
            ordinal: self.ordinal,
        }
        .instance_name(&network_name)?;
        let mut labels = WorkloadFullnodeComponent {
            ordinal: self.ordinal,
        }
        .labels(&network_name)?;
        labels.insert("ech.bz/sui-role".into(), "fullnode".into());

        client
            .namespaced::<Secret>(&namespace)
            .store_put(
                &instance_name,
                BTreeMap::from([(
                    NODE_CONFIG_KEY.to_string(),
                    yamls::fullnode::render(
                        network,
                        self.ordinal,
                        &serde_json::from_str(
                            &client
                                .namespaced::<Secret>(&namespace)
                                .store_load(
                                    KeyFullnodeComponent {
                                        ordinal: self.ordinal,
                                    }
                                    .instance_name(&network_name)?,
                                )
                                .await?
                                .get(KEYS)?,
                        )?,
                        &serde_json::from_str(
                            &client
                                .namespaced::<ConfigMap>(&namespace)
                                .store_load(
                                    WorkerOutputComponent(SeedPeersComponent)
                                        .instance_name(&network_name)?,
                                )
                                .await?
                                .get(SEED_PEERS)?,
                        )?,
                    )?,
                )]),
                Some(labels.clone()),
            )
            .await?;

        client
            .namespaced::<Service>(&namespace)
            .apply_headless_service(
                &instance_name,
                &labels,
                "rpc",
                network.spec.fullnode.port_rpc as i32,
            )
            .await?;

        client
            .namespaced::<StatefulSet>(&namespace)
            .apply_singleton_stateful_set(
                &instance_name,
                &labels,
                "fullnode",
                &instance_name,
                vec![
                    ContainerPort {
                        name: Some("rpc".into()),
                        container_port: network.spec.fullnode.port_rpc as i32,
                        ..Default::default()
                    },
                    ContainerPort {
                        name: Some("metrics".into()),
                        container_port: network.spec.fullnode.port_metrics as i32,
                        ..Default::default()
                    },
                    ContainerPort {
                        name: Some("p2p".into()),
                        container_port: network.spec.fullnode.port_p2p as i32,
                        ..Default::default()
                    },
                    ContainerPort {
                        name: Some("net".into()),
                        container_port: network.spec.fullnode.port_net as i32,
                        ..Default::default()
                    },
                    ContainerPort {
                        name: Some("admin".into()),
                        container_port: network.spec.fullnode.port_admin as i32,
                        ..Default::default()
                    },
                ],
                &network.spec.fullnode.storage.size,
                network.spec.fullnode.storage.class_name.clone(),
                &network.spec.fullnode.cpu,
                &network.spec.fullnode.memory,
                network,
            )
            .await?;

        if client
            .namespaced::<StatefulSet>(&namespace)
            .ready_replicas(&instance_name)
            .await?
            >= 1
        {
            Ok(NodeState::Ready)
        } else {
            Ok(NodeState::Pending)
        }
    }

    async fn cleanup(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<()> {
        let network_name = network.cr_name()?;
        let namespace = network.cr_ns()?;
        let instance_name = WorkloadFullnodeComponent {
            ordinal: self.ordinal,
        }
        .instance_name(&network_name)?;
        client
            .namespaced::<Secret>(&namespace)
            .delete_if_exists(&instance_name)
            .await?;
        client
            .namespaced::<Service>(&namespace)
            .delete_if_exists(&instance_name)
            .await?;
        client
            .namespaced::<StatefulSet>(&namespace)
            .delete_if_exists(&instance_name)
            .await?;
        Ok(())
    }
}
