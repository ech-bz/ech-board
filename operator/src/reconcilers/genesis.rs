use crate::config::OperatorSettings;
use crate::constants::{
    S3_CREDS_DIR, WORKER_CONFIG_DIR, WORKER_CONFIG_FILE_NAME, WORKER_SERVICE_ACCOUNT_NAME,
};
use crate::reconcilers::key_node::{KeySponsorComponent, KeyValidatorComponent};
use crate::reconcilers::workload_validators::WorkloadValidatorComponent;
use crate::support::components::S3CredsComponent;
use crate::support::extensions::JobArtifactExt;
use crate::support::job_builder::JobBuilder;
use crate::{crds::EchBoardNetwork, error::Result};
use ech_board_common::GenesisConfig;
use ech_board_common::keys::{GENESIS_BLOB, KEYS, S3_ACCESS_KEY, S3_SECRET_KEY};
use ech_k8s::{Component, CrMeta, K8sClient, NodeState, Reconciler, ResourcesExt, StoreExt};
use k8s_openapi::api::{
    batch::v1::Job,
    core::v1::{
        ConfigMap, ConfigMapVolumeSource, EmptyDirVolumeSource, Secret, SecretVolumeSource, Volume,
        VolumeMount,
    },
};
use s3::{AddressingStyle, Auth, Client as S3Client, Credentials, Error as S3Error};
use serde::Serialize;
use std::collections::BTreeMap;

const GENESIS_WORK_DIR: &str = "/tmp/ech-board-genesis";
const VALIDATOR_KEYS_ROOT_DIR: &str = "/opt/ech-board/validator-keys";
const SPONSOR_KEY_DIR: &str = "/opt/ech-board/sponsor-key";

#[derive(Clone, Serialize, Component)]
#[component(name = "genesis")]
pub(crate) struct GenesisComponent;

#[derive(Clone, Serialize)]
pub(crate) struct GenesisReconciler {
    pub(crate) operator: OperatorSettings,
}

impl GenesisReconciler {
    pub(crate) fn new(operator: OperatorSettings) -> Self {
        Self { operator }
    }
}

#[async_trait::async_trait]
impl Reconciler for GenesisReconciler {
    type Crd = EchBoardNetwork;
    type Error = crate::error::OperatorError;

    async fn reconcile(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<NodeState> {
        let namespace = network.cr_ns()?;
        let network_name = network.cr_name()?;
        let instance_name = GenesisComponent.instance_name(&network_name)?;
        let labels = GenesisComponent.labels(&network_name)?;
        let validator_replicas = network.spec.validator.replicas as usize;

        client
            .namespaced::<ConfigMap>(&namespace)
            .store_put(
                &instance_name,
                BTreeMap::from([(
                    WORKER_CONFIG_FILE_NAME.to_string(),
                    serde_json::to_string(&GenesisConfig {
                        worker: ech_board_common::WorkerConfig {
                            network_name: network_name.clone(),
                            namespace: namespace.to_string(),
                        },
                        s3: ech_board_common::S3Config {
                            endpoint: network.spec.archive.endpoint.clone(),
                            region: network.spec.archive.region.clone(),
                            bucket: network.spec.archive.bucket.clone(),
                            creds_dir: S3_CREDS_DIR.into(),
                        },
                        validator_key_paths: (0..validator_replicas)
                            .map(|ordinal| format!("{VALIDATOR_KEYS_ROOT_DIR}/{ordinal}/{KEYS}"))
                            .collect(),
                        validator_service_names: (0..validator_replicas)
                            .map(|ordinal| {
                                WorkloadValidatorComponent { ordinal }.instance_name(&network_name)
                            })
                            .collect::<std::result::Result<Vec<_>, _>>()?,
                        sponsor_key_path: format!("{SPONSOR_KEY_DIR}/{KEYS}"),
                        validator_port_p2p: network.spec.validator.port_p2p as u32,
                        sponsor_gas_object_count: network.spec.relay.sponsor.gas_object_count
                            as usize,
                        work_dir: GENESIS_WORK_DIR.into(),
                    })?,
                )]),
                Some(labels.clone()),
            )
            .await?;

        client
            .namespaced::<Job>(&namespace)
            .reconcile_job_artifact(
                JobBuilder {
                    name: instance_name.clone(),
                    namespace: namespace.to_string(),
                    image: self.operator.worker_image.clone(),
                    args: vec![
                        "--config".into(),
                        format!("{WORKER_CONFIG_DIR}/{WORKER_CONFIG_FILE_NAME}"),
                        "genesis".into(),
                    ],
                    labels: GenesisComponent.labels(&network_name)?,
                    service_account_name: WORKER_SERVICE_ACCOUNT_NAME.into(),
                    volumes: std::iter::empty()
                        .chain([Volume {
                            name: "job-config".into(),
                            config_map: Some(ConfigMapVolumeSource {
                                name: instance_name.clone(),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }])
                        .chain([Volume {
                            name: "s3-creds".into(),
                            secret: Some(SecretVolumeSource {
                                secret_name: Some(S3CredsComponent.instance_name(&network_name)?),
                                optional: Some(false),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }])
                        .chain([Volume {
                            name: "genesis-work".into(),
                            empty_dir: Some(EmptyDirVolumeSource::default()),
                            ..Default::default()
                        }])
                        .chain([Volume {
                            name: "sponsor-key".into(),
                            secret: Some(SecretVolumeSource {
                                secret_name: Some(
                                    KeySponsorComponent.instance_name(&network_name)?,
                                ),
                                optional: Some(false),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }])
                        .chain((0..validator_replicas).map(|ordinal| {
                            Volume {
                                name: format!("validator-keys-{ordinal}"),
                                secret: Some(SecretVolumeSource {
                                    secret_name: Some(
                                        KeyValidatorComponent { ordinal }
                                            .instance_name(&network_name)
                                            .unwrap(),
                                    ),
                                    optional: Some(false),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            }
                        }))
                        .collect(),
                    mounts: std::iter::empty()
                        .chain([VolumeMount {
                            name: "job-config".into(),
                            mount_path: WORKER_CONFIG_DIR.into(),
                            read_only: Some(true),
                            ..Default::default()
                        }])
                        .chain([VolumeMount {
                            name: "s3-creds".into(),
                            mount_path: S3_CREDS_DIR.into(),
                            read_only: Some(true),
                            ..Default::default()
                        }])
                        .chain([VolumeMount {
                            name: "genesis-work".into(),
                            mount_path: GENESIS_WORK_DIR.into(),
                            read_only: Some(false),
                            ..Default::default()
                        }])
                        .chain([VolumeMount {
                            name: "sponsor-key".into(),
                            mount_path: format!("{SPONSOR_KEY_DIR}/{KEYS}"),
                            sub_path: Some(KEYS.into()),
                            read_only: Some(true),
                            ..Default::default()
                        }])
                        .chain((0..validator_replicas).map(|ordinal| VolumeMount {
                            name: format!("validator-keys-{ordinal}"),
                            mount_path: format!("{VALIDATOR_KEYS_ROOT_DIR}/{ordinal}"),
                            read_only: Some(true),
                            ..Default::default()
                        }))
                        .collect(),
                }
                .build(),
                || {
                    let namespace = namespace.clone();
                    let network = network.clone();
                    let client = client.clone();
                    let network_name = network_name.clone();
                    Box::pin(async move {
                        let store = client
                            .namespaced::<Secret>(namespace)
                            .store_load(&S3CredsComponent.instance_name(&network_name)?)
                            .await?;
                        match S3Client::builder(&network.spec.archive.endpoint)
                            .map_err(|err| {
                                crate::error::OperatorError::ControllerFatal(err.to_string())
                            })?
                            .region(network.spec.archive.region.clone())
                            .auth(Auth::Static(
                                Credentials::new(
                                    store.get(S3_ACCESS_KEY)?,
                                    store.get(S3_SECRET_KEY)?,
                                )
                                .map_err(|err| {
                                    crate::error::OperatorError::ControllerFatal(err.to_string())
                                })?,
                            ))
                            .addressing_style(AddressingStyle::Path)
                            .build()
                            .map_err(|err| {
                                crate::error::OperatorError::ControllerFatal(err.to_string())
                            })?
                            .objects()
                            .head(&network.spec.archive.bucket, GENESIS_BLOB)
                            .send()
                            .await
                        {
                            Ok(_) => Ok(true),
                            Err(S3Error::Api { status, .. }) if status.as_u16() == 404 => Ok(false),
                            Err(err) => Err(crate::error::OperatorError::ControllerFatal(
                                err.to_string(),
                            )),
                        }
                    })
                },
            )
            .await
    }

    async fn cleanup(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<()> {
        let network_name = network.cr_name()?;
        let namespace = network.cr_ns()?;
        let instance_name = GenesisComponent.instance_name(&network_name)?;
        client
            .namespaced::<ConfigMap>(&namespace)
            .delete_if_exists(&instance_name)
            .await?;
        client
            .namespaced::<Job>(&namespace)
            .delete_if_exists(&instance_name)
            .await?;
        Ok(())
    }
}
