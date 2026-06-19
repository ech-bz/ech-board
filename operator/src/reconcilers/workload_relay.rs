use crate::config::OperatorSettings;
use crate::reconcilers::fullnode_rpc::FullnodeRpcComponent;
use crate::reconcilers::key_node::KeySponsorComponent;
use crate::reconcilers::move_publish::MovePackageComponent;
use crate::support::extensions::{DeploymentExt, HeadlessServiceExt, ReadyReplicasExt};
use crate::{crds::EchBoardNetwork, error::Result};
use ech_board_common::NodeKeypairs;
use ech_board_common::keys::{FORUM_REGISTRY, KEYS, MOVE_ORIGINAL_ID};
use ech_k8s::{Component, CrMeta, K8sClient, NodeState, Reconciler, ResourcesExt, StoreExt};
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{
        Container, ContainerPort, PodSpec, PodTemplateSpec, ResourceRequirements, Secret,
        SecretVolumeSource, Service, Volume, VolumeMount,
    },
};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, LabelSelectorRequirement};
use serde::Serialize;
use std::collections::BTreeMap;

const RELAY_CONFIG_FILE_NAME: &str = "config.json";
const RELAY_CONFIG_DIR: &str = "/opt/ech-board/config";

#[derive(Clone, Copy, Serialize, Component)]
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
        let ns = network.cr_ns()?;
        let owner = network.cr_name()?;
        let instance = WorkloadRelayComponent;
        let sponsor_key = KeySponsorComponent;
        let rpc = FullnodeRpcComponent;

        let key_store = client
            .namespaced::<Secret>(&ns)
            .store_load(sponsor_key.name(&owner))
            .await?;
        let node_keys: NodeKeypairs = serde_json::from_str(key_store.get(KEYS)?).map_err(|e| {
            crate::error::OperatorError::ControllerFatal(format!(
                "failed to parse sponsor keys: {e}"
            ))
        })?;

        client
            .namespaced::<Service>(&ns)
            .apply_headless_service(
                instance.name(&owner),
                instance.labels(&owner),
                "relay",
                network.spec.relay.port as i32,
            )
            .await?;

        let gas_budget = &network.spec.relay.sponsor.gas_budget;
        let gas_price = &network.spec.relay.sponsor.gas_price;

        let package_meta = client
            .namespaced::<Secret>(&ns)
            .store_load(MovePackageComponent.name(&owner))
            .await?;

        let forum_package_id = package_meta
            .get(MOVE_ORIGINAL_ID)
            .map_err(|e| {
                crate::error::OperatorError::ControllerFatal(format!(
                    "MOVE_ORIGINAL_ID not found: {e}"
                ))
            })?
            .to_string();

        let forum_registry = package_meta
            .get(FORUM_REGISTRY)
            .map_err(|e| {
                crate::error::OperatorError::ControllerFatal(format!(
                    "FORUM_REGISTRY not found: {e}"
                ))
            })?
            .to_string();

        let sec_value: toml::Value = toml::from_str(
            client
                .namespaced::<Secret>(&ns)
                .store_load(format!("{}-security-config", owner))
                .await
                .map_err(|e| {
                    crate::error::OperatorError::ControllerFatal(format!(
                        "seaweedfs security-config secret not found: {e}"
                    ))
                })?
                .get("security.toml")
                .map_err(|e| {
                    crate::error::OperatorError::ControllerFatal(format!(
                        "seaweedfs security.toml key not found in secret: {e}"
                    ))
                })?,
        )
        .map_err(|e| {
            crate::error::OperatorError::ControllerFatal(format!(
                "failed to parse seaweedfs security.toml: {e}"
            ))
        })?;
        let seaweed_jwt_signing_key = sec_value["jwt"]["filer_signing"]["key"]
            .as_str()
            .ok_or_else(|| {
                crate::error::OperatorError::ControllerFatal(
                    "jwt.filer_signing.key not found in security.toml".into(),
                )
            })?
            .to_string();

        client
            .namespaced::<Secret>(&ns)
            .store_put(
                instance.name(&owner),
                instance.labels(&owner),
                BTreeMap::from([(
                    RELAY_CONFIG_FILE_NAME.to_string(),
                    serde_json::to_string(&ech_board_common::RelayConfig {
                        server: ech_board_common::RelayServerConfig {
                            bind: format!("0.0.0.0:{}", network.spec.relay.port),
                            admin_bind: format!("0.0.0.0:{}", network.spec.relay.admin_port),
                        },
                        captcha: ech_board_common::RelayCaptchaConfig::Disabled,
                        upstream: ech_board_common::RelayUpstreamConfig {
                            submit_url: format!(
                                "http://{}:{}",
                                rpc.name(&owner),
                                network.spec.fullnode.port_rpc
                            ),
                            request_timeout_ms: 5000,
                        },
                        sponsor: ech_board_common::RelaySponsorConfig {
                            private_key_base64: node_keys.account_keypair.private_key.clone(),
                            gas_budget: gas_budget.parse().map_err(|e| {
                                crate::error::OperatorError::ControllerFatal(format!(
                                    "invalid gas_budget: {e}"
                                ))
                            })?,
                            gas_price: gas_price.parse().map_err(|e| {
                                crate::error::OperatorError::ControllerFatal(format!(
                                    "invalid gas_price: {e}"
                                ))
                            })?,
                        },
                        seaweed_filer_url: network.spec.relay.seaweed_filer_url.clone(),
                        seaweed_jwt_signing_key,
                        forum_package_id,
                        forum_registry,
                    })
                    .map_err(|e| {
                        crate::error::OperatorError::ControllerFatal(format!(
                            "failed to serialize relay config: {e}"
                        ))
                    })?,
                )]),
            )
            .await?;

        client
            .namespaced::<Deployment>(&ns)
            .apply_deployment(
                instance.name(&owner),
                instance.labels(&owner),
                network.spec.relay.replicas as i64,
                PodTemplateSpec {
                    metadata: Some(ObjectMeta {
                        labels: Some(instance.labels(&owner)),
                        ..Default::default()
                    }),
                    spec: Some(PodSpec {
                        containers: vec![Container {
                            name: "relay".into(),
                            image: Some(self.operator.relay_image.clone()),
                            image_pull_policy: Some("IfNotPresent".into()),
                            args: Some(vec![
                                "--config".into(),
                                format!("{RELAY_CONFIG_DIR}/{RELAY_CONFIG_FILE_NAME}"),
                            ]),
                            resources: Some(ResourceRequirements {
                                requests: Some(BTreeMap::from([
                                    ("cpu".into(), Quantity(network.spec.relay.cpu.clone())),
                                    ("memory".into(), Quantity(network.spec.relay.memory.clone())),
                                ])),
                                ..Default::default()
                            }),
                            volume_mounts: Some(vec![VolumeMount {
                                name: "config".into(),
                                mount_path: RELAY_CONFIG_DIR.into(),
                                read_only: Some(true),
                                ..Default::default()
                            }]),
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
                        volumes: Some(vec![Volume {
                            name: "config".into(),
                            secret: Some(SecretVolumeSource {
                                secret_name: Some(instance.name(&owner)),
                                optional: Some(false),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }]),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        if client
            .namespaced::<Deployment>(&ns)
            .ready_replicas(instance.name(&owner))
            .await?
            >= network.spec.relay.replicas as i64
        {
            Ok(NodeState::Ready)
        } else {
            Ok(NodeState::Pending)
        }
    }

    async fn cleanup(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<()> {
        let ns = network.cr_ns()?;
        let owner = network.cr_name()?;
        let instance = WorkloadRelayComponent;
        client
            .namespaced::<Service>(&ns)
            .delete_if_exists(instance.name(&owner))
            .await?;
        client
            .namespaced::<Secret>(&ns)
            .delete_if_exists(instance.name(&owner))
            .await?;
        client
            .namespaced::<Deployment>(&ns)
            .delete_if_exists(instance.name(&owner))
            .await?;
        Ok(())
    }
}
