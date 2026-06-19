use crate::config::OperatorSettings;
use crate::constants::{WORKER_CONFIG_DIR, WORKER_CONFIG_FILE_NAME, WORKER_SERVICE_ACCOUNT_NAME};
use crate::reconcilers::fullnode_rpc::FullnodeRpcComponent;
use crate::reconcilers::key_node::KeySponsorComponent;
use crate::support::components::WorkerOutputComponent;
use crate::support::extensions::JobArtifactExt;
use crate::support::job_builder::JobBuilder;
use crate::{crds::EchBoardNetwork, error::Result};
use ech_board_common::keys::{KEYS, MOVE_REF};
use ech_board_common::{MovePublishConfig, NodeKeypairs};
use ech_k8s::{Component, CrMeta, K8sClient, NodeState, Reconciler, ResourcesExt, StoreExt};
use k8s_openapi::api::{
    batch::v1::Job,
    core::v1::{ConfigMap, Secret, SecretVolumeSource, Volume, VolumeMount},
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Serialize, Component)]
#[component(name = "move-publish")]
pub(crate) struct MovePublishComponent;

#[derive(Clone, Serialize)]
pub(crate) struct MovePublishReconciler {
    pub(crate) operator: OperatorSettings,
}

impl MovePublishReconciler {
    pub(crate) fn new(operator: OperatorSettings) -> Self {
        Self { operator }
    }
}

#[async_trait::async_trait]
impl Reconciler for MovePublishReconciler {
    type Crd = EchBoardNetwork;
    type Error = crate::error::OperatorError;

    async fn reconcile(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<NodeState> {
        let network_name = network.cr_name()?;
        let namespace = network.cr_ns()?;
        let instance_name = MovePublishComponent.instance_name(&network_name)?;
        let output_name =
            WorkerOutputComponent(MovePublishComponent).instance_name(&network_name)?;
        let labels = MovePublishComponent.labels(&network_name)?;

        let node_keys: NodeKeypairs = serde_json::from_str(
            client
                .namespaced::<Secret>(&namespace)
                .store_load(KeySponsorComponent.instance_name(&network_name)?)
                .await?
                .get(KEYS)?,
        )?;

        client
            .namespaced::<Secret>(&namespace)
            .store_put(
                &instance_name,
                BTreeMap::from([(
                    WORKER_CONFIG_FILE_NAME.to_string(),
                    serde_json::to_string(&MovePublishConfig {
                        worker: ech_board_common::WorkerConfig {
                            network_name: network_name.clone(),
                            namespace: namespace.to_string(),
                        },
                        repo: self.operator.move_repo.clone(),
                        git_ref: self.operator.move_git_ref.clone(),
                        package_path: self.operator.move_package_path.clone(),
                        publisher_key_base64: node_keys.account_keypair.private_key.clone(),
                        config_map_name: output_name.clone(),
                        rpc_url: format!(
                            "http://{}:{}",
                            FullnodeRpcComponent.instance_name(&network_name)?,
                            network.spec.fullnode.port_rpc
                        ),
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
                        "move-publish".into(),
                    ],
                    labels: labels.clone(),
                    service_account_name: WORKER_SERVICE_ACCOUNT_NAME.into(),
                    volumes: vec![Volume {
                        name: "job-config".into(),
                        secret: Some(SecretVolumeSource {
                            secret_name: Some(instance_name.clone()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }],
                    mounts: vec![VolumeMount {
                        name: "job-config".into(),
                        mount_path: WORKER_CONFIG_DIR.into(),
                        read_only: Some(true),
                        ..Default::default()
                    }],
                }
                .build(),
                || {
                    let output_name = output_name.clone();
                    let namespace = namespace.to_string();
                    let expected_ref = self.operator.move_git_ref.clone();
                    let client = client.clone();
                    Box::pin(async move {
                        let store = match client
                            .namespaced::<ConfigMap>(&namespace)
                            .store_load(&output_name)
                            .await
                        {
                            Ok(s) => s,
                            Err(kube::Error::Api(e)) if e.code == 404 => return Ok(false),
                            Err(e) => return Err(e.into()),
                        };
                        Ok(store
                            .get(MOVE_REF)
                            .map(|r| r == expected_ref.as_str())
                            .unwrap_or(false))
                    })
                },
            )
            .await
    }

    async fn cleanup(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<()> {
        let network_name = network.cr_name()?;
        let namespace = network.cr_ns()?;
        let instance_name = MovePublishComponent.instance_name(&network_name)?;
        client
            .namespaced::<Secret>(&namespace)
            .delete_if_exists(&instance_name)
            .await?;
        client
            .namespaced::<Job>(&namespace)
            .delete_if_exists(&instance_name)
            .await?;
        Ok(())
    }
}
