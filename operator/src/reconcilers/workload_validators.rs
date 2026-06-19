use crate::constants::NODE_CONFIG_KEY;
use crate::reconcilers::key_node::KeyValidatorComponent;
use crate::support::extensions::{HeadlessServiceExt, ReadyReplicasExt, SingletonStatefulSetExt};
use crate::support::yamls;
use crate::{crds::EchBoardNetwork, error::Result};
use ech_board_common::keys::KEYS;
use ech_k8s::{Component, CrMeta, K8sClient, NodeState, Reconciler, ResourcesExt, StoreExt};
use k8s_openapi::api::apps::v1::StatefulSet;
use k8s_openapi::api::core::v1::{ContainerPort, Secret, Service};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Serialize, Component)]
#[component(name = "workload-validator")]
pub(crate) struct WorkloadValidatorComponent {
    pub(crate) ordinal: usize,
}

#[derive(Clone, Serialize)]

pub(crate) struct WorkloadValidatorReconciler {
    pub(crate) ordinal: usize,
}

#[async_trait::async_trait]
impl Reconciler for WorkloadValidatorReconciler {
    type Crd = EchBoardNetwork;
    type Error = crate::error::OperatorError;

    async fn reconcile(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<NodeState> {
        let network_name = network.cr_name()?;
        let namespace = network.cr_ns()?;
        let instance_name = WorkloadValidatorComponent {
            ordinal: self.ordinal,
        }
        .instance_name(&network_name)?;
        let secret_name = KeyValidatorComponent {
            ordinal: self.ordinal,
        }
        .instance_name(&network_name)?;
        let labels = WorkloadValidatorComponent {
            ordinal: self.ordinal,
        }
        .labels(&network_name)?;

        client
            .namespaced::<Secret>(&namespace)
            .store_put(
                &instance_name,
                BTreeMap::from([(
                    NODE_CONFIG_KEY.to_string(),
                    yamls::validator::render(
                        network,
                        self.ordinal,
                        &serde_json::from_str(
                            &client
                                .namespaced::<Secret>(&namespace)
                                .store_load(&secret_name)
                                .await?
                                .get(KEYS)?,
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
                "p2p",
                network.spec.validator.port_p2p as i32,
            )
            .await?;

        client
            .namespaced::<StatefulSet>(&namespace)
            .apply_singleton_stateful_set(
                &instance_name,
                &labels,
                "validator",
                &instance_name,
                vec![ContainerPort {
                    name: Some("p2p".into()),
                    container_port: network.spec.validator.port_p2p as i32,
                    ..Default::default()
                }],
                &network.spec.validator.storage.size,
                network.spec.validator.storage.class_name.clone(),
                &network.spec.validator.cpu,
                &network.spec.validator.memory,
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
        let instance_name = WorkloadValidatorComponent {
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
