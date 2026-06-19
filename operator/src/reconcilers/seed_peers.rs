use crate::config::OperatorSettings;
use crate::constants::{
    S3_CREDS_DIR, WORKER_CONFIG_DIR, WORKER_CONFIG_FILE_NAME, WORKER_SERVICE_ACCOUNT_NAME,
};
use crate::support::components::{S3CredsComponent, WorkerOutputComponent};
use crate::support::extensions::JobArtifactExt;
use crate::support::job_builder::JobBuilder;
use crate::{crds::EchBoardNetwork, error::Result};
use ech_board_common::ValidatorsConfig;
use ech_k8s::{Component, CrMeta, K8sClient, NodeState, Reconciler, ResourcesExt, StoreExt};
use k8s_openapi::api::{
    batch::v1::Job,
    core::v1::{ConfigMap, ConfigMapVolumeSource, SecretVolumeSource, Volume, VolumeMount},
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Serialize, Component)]
#[component(name = "seed-peers")]
pub(crate) struct SeedPeersComponent;

#[derive(Clone, Serialize)]
pub(crate) struct SeedPeersReconciler {
    pub(crate) operator: OperatorSettings,
}

impl SeedPeersReconciler {
    pub(crate) fn new(operator: OperatorSettings) -> Self {
        Self { operator }
    }
}

#[async_trait::async_trait]
impl Reconciler for SeedPeersReconciler {
    type Crd = EchBoardNetwork;
    type Error = crate::error::OperatorError;

    async fn reconcile(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<NodeState> {
        let network_name = network.cr_name()?;
        let namespace = network.cr_ns()?;
        let instance_name = SeedPeersComponent.instance_name(&network_name)?;
        let output_name = WorkerOutputComponent(SeedPeersComponent).instance_name(&network_name)?;
        let labels = SeedPeersComponent.labels(&network_name)?;

        client
            .namespaced::<ConfigMap>(&namespace)
            .store_put(
                &instance_name,
                BTreeMap::from([(
                    WORKER_CONFIG_FILE_NAME.to_string(),
                    serde_json::to_string(&ValidatorsConfig {
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
                        config_map_name: output_name.clone(),
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
                        "seed-peers".into(),
                    ],
                    labels: labels.clone(),
                    service_account_name: WORKER_SERVICE_ACCOUNT_NAME.into(),
                    volumes: vec![
                        Volume {
                            name: "job-config".into(),
                            config_map: Some(ConfigMapVolumeSource {
                                name: instance_name.clone(),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                        Volume {
                            name: "s3-creds".into(),
                            secret: Some(SecretVolumeSource {
                                secret_name: Some(S3CredsComponent.instance_name(&network_name)?),
                                optional: Some(false),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                    ],
                    mounts: vec![
                        VolumeMount {
                            name: "job-config".into(),
                            mount_path: WORKER_CONFIG_DIR.into(),
                            read_only: Some(true),
                            ..Default::default()
                        },
                        VolumeMount {
                            name: "s3-creds".into(),
                            mount_path: S3_CREDS_DIR.into(),
                            read_only: Some(true),
                            ..Default::default()
                        },
                    ],
                }
                .build(),
                || {
                    let namespace = namespace.to_string();
                    let output_name = output_name.clone();
                    let client = client.clone();
                    Box::pin(async move {
                        Ok(client
                            .namespaced::<ConfigMap>(&namespace)
                            .api()
                            .get_opt(&output_name)
                            .await?
                            .is_some())
                    })
                },
            )
            .await
    }

    async fn cleanup(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<()> {
        let network_name = network.cr_name()?;
        let namespace = network.cr_ns()?;
        let instance_name = SeedPeersComponent.instance_name(&network_name)?;
        let output_name = WorkerOutputComponent(SeedPeersComponent).instance_name(&network_name)?;
        client
            .namespaced::<ConfigMap>(&namespace)
            .delete_if_exists(&instance_name)
            .await?;
        client
            .namespaced::<ConfigMap>(&namespace)
            .delete_if_exists(&output_name)
            .await?;
        client
            .namespaced::<Job>(&namespace)
            .delete_if_exists(&instance_name)
            .await?;
        Ok(())
    }
}
