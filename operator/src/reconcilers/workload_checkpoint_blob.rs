use crate::config::OperatorSettings;
use crate::reconcilers::bootstrap::S3CredsComponent;
use crate::reconcilers::workload_archive::WorkloadArchiveComponent;
use crate::support::components::WorkerConfigComponent;
use crate::support::extensions::{DeploymentExt, HeadlessServiceExt, ReadyReplicasExt};
use crate::{crds::EchBoardNetwork, error::Result};
use ech_board_common::keys::{S3_ACCESS_KEY, S3_SECRET_KEY};
use ech_k8s::{Component, CrMeta, K8sClient, NodeState, Reconciler, StoreExt};
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{
        Container, ContainerPort, EnvVar, PodSpec, PodTemplateSpec, ResourceRequirements, Secret,
        SecretVolumeSource, Service, Volume, VolumeMount,
    },
};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use serde::Serialize;
use std::collections::BTreeMap;

const CHECKPOINT_BLOB_CONFIG_KEY: &str = "indexer.toml";

#[derive(Clone, Copy, Serialize, Component)]
#[component(name = "workload-checkpoint-blob")]
pub(crate) struct WorkloadCheckpointBlobComponent;

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum ConcurrencyConfig {
    Fixed { value: usize },
}

#[derive(Clone, Serialize)]
pub(crate) struct CheckpointBlobIndexerConfig {
    pub(crate) ingestion: CheckpointBlobIngestionConfig,
    pub(crate) committer: CheckpointBlobCommitterConfig,
    pub(crate) pipeline: CheckpointBlobPipelineConfig,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct CheckpointBlobIngestionConfig {
    pub(crate) ingest_concurrency: ConcurrencyConfig,
    pub(crate) retry_interval_ms: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct CheckpointBlobCommitterConfig {
    pub(crate) write_concurrency: usize,
    pub(crate) collect_interval_ms: u64,
    pub(crate) watermark_interval_ms: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct CheckpointBlobPipelineConfig {
    pub(crate) checkpoint_blob: CheckpointBlobPipelineLayer,
    pub(crate) epochs: CheckpointBlobPipelineLayer,
    pub(crate) checkpoint_bcs: CheckpointBlobPipelineLayer,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct CheckpointBlobPipelineLayer {
    pub(crate) ingestion: CheckpointBlobPipelineIngestionConfig,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct CheckpointBlobPipelineIngestionConfig {
    pub(crate) subscriber_channel_size: usize,
}

#[derive(Clone, Serialize)]
pub(crate) struct WorkloadCheckpointBlobReconciler {
    pub(crate) operator: OperatorSettings,
}

impl WorkloadCheckpointBlobReconciler {
    pub(crate) fn new(operator: OperatorSettings) -> Self {
        Self { operator }
    }
}

#[async_trait::async_trait]
impl Reconciler for WorkloadCheckpointBlobReconciler {
    type Crd = EchBoardNetwork;
    type Error = crate::error::OperatorError;

    async fn reconcile(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<NodeState> {
        let ns = network.cr_ns()?;
        let owner = network.cr_name()?;
        let instance = WorkloadCheckpointBlobComponent;
        let config_comp = WorkerConfigComponent(instance);
        let s3_creds = S3CredsComponent;
        let rpc = WorkloadArchiveComponent;

        let s3_creds = client
            .namespaced::<Secret>(&ns)
            .store_load(s3_creds.name(&owner))
            .await?;

        client
            .namespaced::<Secret>(&ns)
            .store_put(
                config_comp.name(&owner),
                config_comp.labels(&owner),
                BTreeMap::from([(
                    CHECKPOINT_BLOB_CONFIG_KEY.to_string(),
                    toml::to_string_pretty(&CheckpointBlobIndexerConfig {
                        ingestion: CheckpointBlobIngestionConfig {
                            ingest_concurrency: ConcurrencyConfig::Fixed { value: 10 },
                            retry_interval_ms: 1000,
                        },
                        committer: CheckpointBlobCommitterConfig {
                            write_concurrency: 10,
                            collect_interval_ms: 100,
                            watermark_interval_ms: 100,
                        },
                        pipeline: CheckpointBlobPipelineConfig {
                            checkpoint_blob: CheckpointBlobPipelineLayer {
                                ingestion: CheckpointBlobPipelineIngestionConfig {
                                    subscriber_channel_size: 1000,
                                },
                            },
                            epochs: CheckpointBlobPipelineLayer {
                                ingestion: CheckpointBlobPipelineIngestionConfig {
                                    subscriber_channel_size: 1000,
                                },
                            },
                            checkpoint_bcs: CheckpointBlobPipelineLayer {
                                ingestion: CheckpointBlobPipelineIngestionConfig {
                                    subscriber_channel_size: 1000,
                                },
                            },
                        },
                    })
                    .map_err(|e| {
                        crate::error::OperatorError::ControllerFatal(format!(
                            "failed to serialize checkpoint blob config: {e}"
                        ))
                    })?,
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

        let blob_spec = &network.spec.checkpoint_blob;

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
                            name: "checkpoint-blob-indexer".into(),
                            image: Some(self.operator.checkpoint_blob_indexer_image.clone()),
                            image_pull_policy: Some("IfNotPresent".into()),
                            command: Some(vec!["/opt/sui/bin/sui-checkpoint-blob-indexer".into()]),
                            resources: Some(ResourceRequirements {
                                requests: Some(BTreeMap::from([
                                    ("cpu".into(), Quantity(blob_spec.cpu.clone())),
                                    ("memory".into(), Quantity(blob_spec.memory.clone())),
                                ])),
                                ..Default::default()
                            }),
                            args: Some(vec![
                                "--config".into(),
                                "/opt/ech-board/config/indexer.toml".into(),
                                "--s3".into(),
                                network.spec.archive.bucket.clone(),
                                "--rpc-api-url".into(),
                                format!(
                                    "http://{}:{}",
                                    rpc.name(&owner),
                                    network.spec.fullnode.port_rpc
                                ),
                                "--metrics-address".into(),
                                "0.0.0.0:9184".into(),
                                "--compression-level".into(),
                                blob_spec.compression_level.to_string(),
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
                                    secret_name: Some(config_comp.name(&owner)),
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
