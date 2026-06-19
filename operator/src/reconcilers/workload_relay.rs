use crate::config::OperatorSettings;
use crate::reconcilers::fullnode_rpc::FullnodeRpcComponent;
use crate::reconcilers::key_node::KeySponsorComponent;
use crate::reconcilers::move_publish::MovePublishComponent;
use crate::support::components::WorkerOutputComponent;
use crate::support::extensions::{DeploymentExt, HeadlessServiceExt, ReadyReplicasExt};
use crate::{crds::EchBoardNetwork, error::Result};
use ech_board_common::NodeKeypairs;
use ech_board_common::keys::{KEYS, MOVE_ORIGINAL_ID};
use ech_k8s::{Component, CrMeta, K8sClient, NodeState, Reconciler, ResourcesExt, StoreExt};
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{
        ConfigMap, Container, ContainerPort, EnvVar, PodSpec, PodTemplateSpec,
        ResourceRequirements, Secret, Service,
    },
};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, LabelSelectorRequirement};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Serialize, Component)]
#[component(name = "workload-relay")]
pub(crate) struct WorkloadRelayComponent;

#[derive(Clone, Serialize)]
pub(crate) struct WorkloadRelayReconciler {
    pub(crate) operator: OperatorSettings,
}

impl WorkloadRelayReconciler {
    pub(crate) fn new(operator: OperatorSettings) -> Self {
        Self { operator }
    }
}

#[async_trait::async_trait]
impl Reconciler for WorkloadRelayReconciler {
    type Crd = EchBoardNetwork;
    type Error = crate::error::OperatorError;

    async fn reconcile(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<NodeState> {
        let network_name = network.cr_name()?;
        let namespace = network.cr_ns()?;
        let instance_name = WorkloadRelayComponent.instance_name(&network_name)?;
        let labels = WorkloadRelayComponent.labels(&network_name)?;

        let key_store = client
            .namespaced::<Secret>(&namespace)
            .store_load(KeySponsorComponent.instance_name(&network_name)?)
            .await?;
        let node_keys: NodeKeypairs = serde_json::from_str(key_store.get(KEYS)?).map_err(|e| {
            crate::error::OperatorError::ControllerFatal(format!(
                "failed to parse sponsor keys: {e}"
            ))
        })?;

        client
            .namespaced::<Service>(&namespace)
            .apply_headless_service(
                &instance_name,
                &labels,
                "relay",
                network.spec.relay.port as i32,
            )
            .await?;

        let relay_port = network.spec.relay.port.to_string();
        let gas_budget = &network.spec.relay.sponsor.gas_budget;
        let gas_price = &network.spec.relay.sponsor.gas_price;

        let move_store = client
            .namespaced::<ConfigMap>(&namespace)
            .store_load(WorkerOutputComponent(MovePublishComponent).instance_name(&network_name)?)
            .await?;
        let forum_package_id = move_store
            .get(MOVE_ORIGINAL_ID)
            .map_err(|e| {
                crate::error::OperatorError::ControllerFatal(format!(
                    "MOVE_ORIGINAL_ID not found: {e}"
                ))
            })?
            .to_string();

        client
            .namespaced::<Deployment>(&namespace)
            .apply_deployment(
                &instance_name,
                &labels,
                network.spec.relay.replicas as i64,
                PodTemplateSpec {
                    metadata: Some(ObjectMeta {
                        labels: Some(labels.clone()),
                        ..Default::default()
                    }),
                    spec: Some(PodSpec {
                        containers: vec![Container {
                            name: "relay".into(),
                            image: Some(self.operator.relay_image.clone()),
                            image_pull_policy: Some("IfNotPresent".into()),
                            resources: Some(ResourceRequirements {
                                requests: Some(BTreeMap::from([
                                    ("cpu".into(), Quantity(network.spec.relay.cpu.clone())),
                                    ("memory".into(), Quantity(network.spec.relay.memory.clone())),
                                ])),
                                ..Default::default()
                            }),
                            env: Some(vec![
                                EnvVar {
                                    name: "RELAY_SERVER_BIND".into(),
                                    value: Some(format!("0.0.0.0:{relay_port}")),
                                    ..Default::default()
                                },
                                EnvVar {
                                    name: "RELAY_UPSTREAM_SUBMIT_URL".into(),
                                    value: Some(format!(
                                        "http://{}:{}",
                                        FullnodeRpcComponent.instance_name(&network_name)?,
                                        network.spec.fullnode.port_rpc
                                    )),
                                    ..Default::default()
                                },
                                EnvVar {
                                    name: "RELAY_UPSTREAM_REQUEST_TIMEOUT_MS".into(),
                                    value: Some("5000".into()),
                                    ..Default::default()
                                },
                                EnvVar {
                                    name: "RELAY_SPONSOR_PRIVATE_KEY_BASE64".into(),
                                    value: Some(node_keys.account_keypair.private_key.clone()),
                                    ..Default::default()
                                },
                                EnvVar {
                                    name: "RELAY_SPONSOR_GAS_BUDGET".into(),
                                    value: Some(gas_budget.clone()),
                                    ..Default::default()
                                },
                                EnvVar {
                                    name: "RELAY_SPONSOR_GAS_PRICE".into(),
                                    value: Some(gas_price.clone()),
                                    ..Default::default()
                                },
                                EnvVar {
                                    name: "RELAY_CAPTCHA_PROVIDER".into(),
                                    value: Some("disabled".into()),
                                    ..Default::default()
                                },
                                EnvVar {
                                    name: "RELAY_FORUM_PACKAGE_ID".into(),
                                    value: Some(forum_package_id),
                                    ..Default::default()
                                },
                            ]),
                            ports: Some(vec![ContainerPort {
                                name: Some("relay".into()),
                                container_port: network.spec.relay.port as i32,
                                ..Default::default()
                            }]),
                            ..Default::default()
                        }],
                        affinity: Some(k8s_openapi::api::core::v1::Affinity {
                            pod_anti_affinity: Some(k8s_openapi::api::core::v1::PodAntiAffinity {
                                preferred_during_scheduling_ignored_during_execution: Some(vec![
                                    k8s_openapi::api::core::v1::WeightedPodAffinityTerm {
                                        weight: 100,
                                        pod_affinity_term:
                                            k8s_openapi::api::core::v1::PodAffinityTerm {
                                                label_selector: Some(LabelSelector {
                                                    match_expressions: Some(vec![
                                                        LabelSelectorRequirement {
                                                            key: "ech.bz/owner".into(),
                                                            operator: "Exists".into(),
                                                            values: None,
                                                        },
                                                    ]),
                                                    ..Default::default()
                                                }),
                                                topology_key: "kubernetes.io/hostname".into(),
                                                ..Default::default()
                                            },
                                    },
                                ]),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }),
                        termination_grace_period_seconds: Some(30),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        if client
            .namespaced::<Deployment>(&namespace)
            .ready_replicas(&instance_name)
            .await?
            >= network.spec.relay.replicas as i64
        {
            Ok(NodeState::Ready)
        } else {
            Ok(NodeState::Pending)
        }
    }

    async fn cleanup(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<()> {
        let network_name = network.cr_name()?;
        let namespace = network.cr_ns()?;
        let instance_name = WorkloadRelayComponent.instance_name(&network_name)?;
        client
            .namespaced::<Deployment>(&namespace)
            .delete_if_exists(&instance_name)
            .await?;
        client
            .namespaced::<Service>(&namespace)
            .delete_if_exists(&instance_name)
            .await?;
        Ok(())
    }
}
