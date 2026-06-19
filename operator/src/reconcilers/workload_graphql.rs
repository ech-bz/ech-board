use crate::config::OperatorSettings;
use crate::crds::EchBoardNetwork;
use crate::error::Result;
use crate::support::extensions::{DeploymentExt, ReadyReplicasExt};
use ech_k8s::{Component, CrMeta, K8sClient, NodeState, Reconciler, ResourcesExt, StoreExt};
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{
        Container, ContainerPort, EnvVar, PodSpec, PodTemplateSpec, ResourceRequirements, Secret,
        SecretVolumeSource, Service, ServicePort, ServiceSpec, Volume, VolumeMount,
    },
};
use k8s_openapi::apimachinery::pkg::{
    api::resource::Quantity, apis::meta::v1::ObjectMeta, util::intstr::IntOrString,
};
use serde::Serialize;
use std::collections::BTreeMap;

const GRAPHQL_CONFIG_TOML: &str = r#"[pipeline]
cp_sequence_numbers = {}
kv_checkpoints = {}
kv_epoch_starts = {}
kv_epoch_ends = {}
kv_feature_flags = {}
kv_transactions = {}
kv_objects = {}
kv_packages = {}
kv_protocol_configs = {}
tx_digests = {}
tx_calls = {}
tx_kinds = {}
obj_versions = {}
tx_affected_objects = {}
tx_affected_addresses = {}
tx_balance_changes = {}
ev_emit_mod = {}
ev_struct_inst = {}
"#;

#[derive(Clone, Copy, Serialize, Component)]
#[component(name = "workload-graphql")]
pub(crate) struct WorkloadGraphqlComponent;

#[derive(Clone, serde::Serialize)]
pub(crate) struct WorkloadGraphqlReconciler {
    pub(crate) operator: OperatorSettings,
}

impl WorkloadGraphqlReconciler {
    pub(crate) fn new(operator: OperatorSettings) -> Self {
        Self { operator }
    }
}

#[async_trait::async_trait]
impl Reconciler for WorkloadGraphqlReconciler {
    type Crd = EchBoardNetwork;
    type Error = crate::error::OperatorError;

    async fn reconcile(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<NodeState> {
        let ns = network.cr_ns()?;
        let indexer = &network.spec.indexer;
        let spec = &indexer.graphql;
        let owner = network.cr_name()?;
        let instance = WorkloadGraphqlComponent;

        let db_secret: Secret = client
            .namespaced::<Secret>(&ns)
            .api()
            .get(&indexer.database_secret_ref.name)
            .await
            .map_err(|e| {
                crate::error::OperatorError::ControllerFatal(format!(
                    "failed to load secret '{}': {e}",
                    indexer.database_secret_ref.name
                ))
            })?;

        let database_url = String::from_utf8(
            db_secret
                .data
                .as_ref()
                .and_then(|d| d.get(&indexer.database_secret_ref.key))
                .map(|b| b.0.clone())
                .ok_or_else(|| {
                    crate::error::OperatorError::ControllerFatal(format!(
                        "key '{}' not found in secret '{}'",
                        indexer.database_secret_ref.key, indexer.database_secret_ref.name
                    ))
                })?,
        )
        .map_err(|e| {
            crate::error::OperatorError::ControllerFatal(format!(
                "invalid UTF-8 in secret '{}' key '{}': {e}",
                indexer.database_secret_ref.name, indexer.database_secret_ref.key
            ))
        })?;

        let labels = instance.labels(&owner);
        let name = instance.name(&owner);

        client
            .namespaced::<Secret>(&ns)
            .store_put(
                &name,
                labels.clone(),
                BTreeMap::from([("indexer.toml".to_string(), GRAPHQL_CONFIG_TOML.to_string())]),
            )
            .await?;

        let service = Service {
            metadata: ObjectMeta {
                name: Some(name.clone()),
                labels: Some(labels.clone()),
                ..Default::default()
            },
            spec: Some(ServiceSpec {
                selector: Some(labels.clone()),
                ports: Some(vec![ServicePort {
                    name: Some("graphql".into()),
                    port: spec.port as i32,
                    target_port: Some(IntOrString::Int(spec.port as i32)),
                    ..Default::default()
                }]),
                ..Default::default()
            }),
            ..Default::default()
        };
        client
            .namespaced::<Service>(&ns)
            .apply(&name, &service)
            .await?;

        client
            .namespaced::<Deployment>(&ns)
            .apply_deployment(
                &name,
                labels,
                1,
                PodTemplateSpec {
                    metadata: Some(ObjectMeta {
                        labels: Some(instance.labels(&owner)),
                        ..Default::default()
                    }),
                    spec: Some(PodSpec {
                        containers: vec![Container {
                            name: "graphql".into(),
                            image: Some(self.operator.graphql_image.clone()),
                            image_pull_policy: Some("IfNotPresent".into()),
                            command: Some(vec!["sui-indexer-alt-graphql".into()]),
                            resources: Some(ResourceRequirements {
                                requests: Some(BTreeMap::from([
                                    ("cpu".into(), Quantity(spec.cpu.clone())),
                                    ("memory".into(), Quantity(spec.memory.clone())),
                                ])),
                                ..Default::default()
                            }),
                            args: Some(vec![
                                "rpc".into(),
                                "--database-url".into(),
                                database_url,
                                "--rpc-listen-address".into(),
                                format!("0.0.0.0:{}", spec.port),
                                "--indexer-config".into(),
                                "/opt/ech-board/config/indexer.toml".into(),
                            ]),
                            env: Some(vec![EnvVar {
                                name: "RUST_LOG".into(),
                                value: Some("info".into()),
                                ..Default::default()
                            }]),
                            ports: Some(vec![ContainerPort {
                                name: Some("graphql".into()),
                                container_port: spec.port as i32,
                                ..Default::default()
                            }]),
                            volume_mounts: Some(vec![VolumeMount {
                                name: "config".into(),
                                mount_path: "/opt/ech-board/config".into(),
                                read_only: Some(true),
                                ..Default::default()
                            }]),
                            ..Default::default()
                        }],
                        termination_grace_period_seconds: Some(30),
                        volumes: Some(vec![Volume {
                            name: "config".into(),
                            secret: Some(SecretVolumeSource {
                                secret_name: Some(name.clone()),
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
            .ready_replicas(&name)
            .await?
            >= 1
        {
            Ok(NodeState::Ready)
        } else {
            Ok(NodeState::Pending)
        }
    }
}
