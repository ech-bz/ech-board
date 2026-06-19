use crate::config::OperatorSettings;
use crate::crds::EchBoardNetwork;
use crate::error::Result;
use crate::reconcilers::bootstrap::S3CredsComponent;
use crate::support::extensions::{DeploymentExt, HeadlessServiceExt, ReadyReplicasExt};
use ech_board_common::keys::{S3_ACCESS_KEY, S3_SECRET_KEY};
use ech_k8s::{Component, CrMeta, K8sClient, NodeState, Reconciler, StoreExt};
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{
        Container, ContainerPort, EnvVar, PodSpec, PodTemplateSpec, ResourceRequirements, Secret,
        SecretVolumeSource, Service, Volume, VolumeMount,
    },
};
use k8s_openapi::apimachinery::pkg::{api::resource::Quantity, apis::meta::v1::ObjectMeta};
use serde::Serialize;
use std::collections::BTreeMap;

const INDEXER_ALT_CONFIG_KEY: &str = "indexer.toml";

const INDEXER_ALT_CONFIG_TOML: &str = r#"[ingestion]
ingest-concurrency = { kind = "fixed", value = 10 }
retry-interval-ms = 1000

[committer]
write-concurrency = 10
collect-interval-ms = 100
watermark-interval-ms = 100

[pipeline.cp_sequence_numbers]
[pipeline.kv_checkpoints]
[pipeline.kv_epoch_starts]
[pipeline.kv_epoch_ends]
[pipeline.kv_feature_flags]
[pipeline.kv_transactions]
[pipeline.kv_objects]
[pipeline.kv_packages]
[pipeline.kv_protocol_configs]
[pipeline.tx_digests]
[pipeline.tx_calls]
[pipeline.tx_kinds]
[pipeline.obj_versions]
[pipeline.tx_affected_objects]
[pipeline.tx_affected_addresses]
[pipeline.tx_balance_changes]
[pipeline.ev_emit_mod]
[pipeline.ev_struct_inst]
"#;

#[derive(Clone, Copy, Serialize, Component)]
#[component(name = "workload-indexer-alt")]
pub(crate) struct WorkloadIndexerAltComponent;

#[derive(Clone, serde::Serialize)]
pub(crate) struct WorkloadIndexerAltReconciler {
    pub(crate) operator: OperatorSettings,
}

impl WorkloadIndexerAltReconciler {
    pub(crate) fn new(operator: OperatorSettings) -> Self {
        Self { operator }
    }
}

#[async_trait::async_trait]
impl Reconciler for WorkloadIndexerAltReconciler {
    type Crd = EchBoardNetwork;
    type Error = crate::error::OperatorError;

    async fn reconcile(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<NodeState> {
        let ns = network.cr_ns()?;
        let indexer = &network.spec.indexer;
        let spec = &indexer.alt;
        let owner = network.cr_name()?;
        let instance = WorkloadIndexerAltComponent;
        let s3_creds_comp = S3CredsComponent;

        let s3_creds = client
            .namespaced::<Secret>(&ns)
            .store_load(s3_creds_comp.name(&owner))
            .await?;

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

        client
            .namespaced::<Secret>(&ns)
            .store_put(
                instance.name(&owner),
                instance.labels(&owner),
                BTreeMap::from([(
                    INDEXER_ALT_CONFIG_KEY.to_string(),
                    INDEXER_ALT_CONFIG_TOML.to_string(),
                )]),
            )
            .await?;

        client
            .namespaced::<Service>(&ns)
            .apply_headless_service(
                instance.name(&owner),
                instance.labels(&owner),
                "metrics",
                9184,
            )
            .await?;

        client
            .namespaced::<Deployment>(&ns)
            .apply_deployment(
                instance.name(&owner),
                instance.labels(&owner),
                1,
                PodTemplateSpec {
                    metadata: Some(ObjectMeta {
                        labels: Some(instance.labels(&owner)),
                        ..Default::default()
                    }),
                    spec: Some(PodSpec {
                        containers: vec![Container {
                            name: "indexer-alt".into(),
                            image: Some(self.operator.indexer_alt_image.clone()),
                            image_pull_policy: Some("IfNotPresent".into()),
                            command: Some(vec!["sui-indexer-alt".into()]),
                            resources: Some(ResourceRequirements {
                                requests: Some(BTreeMap::from([
                                    ("cpu".into(), Quantity(spec.cpu.clone())),
                                    ("memory".into(), Quantity(spec.memory.clone())),
                                ])),
                                ..Default::default()
                            }),
                            args: Some(vec![
                                "indexer".into(),
                                "--database-url".into(),
                                database_url,
                                "--remote-store-s3".into(),
                                network.spec.archive.bucket.clone(),
                                "--config".into(),
                                format!("/opt/ech-board/config/{INDEXER_ALT_CONFIG_KEY}"),
                                "--metrics-address".into(),
                                "0.0.0.0:9184".into(),
                            ]),
                            env: Some(vec![
                                EnvVar {
                                    name: "AWS_ENDPOINT".into(),
                                    value: Some(network.spec.archive.endpoint.clone()),
                                    ..Default::default()
                                },
                                EnvVar {
                                    name: "AWS_DEFAULT_REGION".into(),
                                    value: Some(network.spec.archive.region.clone()),
                                    ..Default::default()
                                },
                                EnvVar {
                                    name: "AWS_ACCESS_KEY_ID".into(),
                                    value: Some(s3_creds.get(S3_ACCESS_KEY)?.to_string()),
                                    ..Default::default()
                                },
                                EnvVar {
                                    name: "AWS_SECRET_ACCESS_KEY".into(),
                                    value: Some(s3_creds.get(S3_SECRET_KEY)?.to_string()),
                                    ..Default::default()
                                },
                                EnvVar {
                                    name: "SSL_CERT_FILE".into(),
                                    value: Some("/certs/tls.crt".into()),
                                    ..Default::default()
                                },
                            ]),
                            ports: Some(vec![ContainerPort {
                                name: Some("metrics".into()),
                                container_port: 9184,
                                ..Default::default()
                            }]),
                            volume_mounts: Some(vec![
                                VolumeMount {
                                    name: "config".into(),
                                    mount_path: "/opt/ech-board/config".into(),
                                    read_only: Some(true),
                                    ..Default::default()
                                },
                                VolumeMount {
                                    name: "s3proxy-ca".into(),
                                    mount_path: "/certs".into(),
                                    read_only: Some(true),
                                    ..Default::default()
                                },
                            ]),
                            ..Default::default()
                        }],
                        termination_grace_period_seconds: Some(30),
                        volumes: Some(vec![
                            Volume {
                                name: "config".into(),
                                secret: Some(SecretVolumeSource {
                                    secret_name: Some(instance.name(&owner)),
                                    optional: Some(false),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            },
                            Volume {
                                name: "s3proxy-ca".into(),
                                secret: Some(SecretVolumeSource {
                                    secret_name: Some("s3proxy-ca-secret".into()),
                                    optional: Some(false),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            },
                        ]),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        if client
            .namespaced::<Deployment>(&ns)
            .ready_replicas(instance.name(&owner))
            .await?
            >= 1
        {
            Ok(NodeState::Ready)
        } else {
            Ok(NodeState::Pending)
        }
    }
}
