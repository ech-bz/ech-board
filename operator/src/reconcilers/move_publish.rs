use crate::config::OperatorSettings;
use crate::crds::{EchBoardNetwork, ExternalSecret, PushSecret};
use crate::error::{OperatorError, Result};
use crate::reconcilers::fullnode_rpc::FullnodeRpcComponent;
use crate::reconcilers::key_node::KeySponsorComponent;
use crate::support::components::{WorkerConfigComponent, WorkerOutputComponent};
use crate::support::extensions::{ExternalSecretExt, PushSecretExt, WorkerExt};
use ech_board_common::keys::{FORUM_REGISTRY, KEYS, MOVE_ORIGINAL_ID};
use ech_board_common::{MovePublishConfig, NodeKeypairs};
use ech_k8s::{Component, CrMeta, K8sClient, NodeState, Reconciler, ResourcesExt, StoreExt};
use k8s_openapi::api::{batch::v1::Job, core::v1::Secret};
use serde::Serialize;

#[derive(Clone, Copy, Serialize, Component)]
#[component(name = "move-package")]
pub(crate) struct MovePackageComponent;

#[derive(Clone, Copy, Serialize, Component)]
#[component(name = "move-publish")]
pub(crate) struct MovePublishComponent(u32);

impl MovePublishComponent {
    pub(crate) fn new(git_ref: &str) -> Result<Self> {
        Ok(Self(u32::from_str_radix(&git_ref[0..8], 16).map_err(
            |_| OperatorError::Config(format!("Invalid move_git_ref \"{git_ref}\"")),
        )?))
    }
}

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
        let ns = network.cr_ns()?;
        let owner = network.cr_name()?;
        let instance = MovePublishComponent::new(&self.operator.move_git_ref)?;

        client
            .namespaced::<PushSecret>(&ns)
            .apply_push_secret(
                &owner,
                WorkerOutputComponent(instance),
                MovePackageComponent,
                &[MOVE_ORIGINAL_ID, FORUM_REGISTRY],
            )
            .await?;

        client
            .namespaced::<ExternalSecret>(&ns)
            .apply_external_secret(
                &owner,
                MovePackageComponent,
                MovePackageComponent,
                &[
                    (MOVE_ORIGINAL_ID, Some(MOVE_ORIGINAL_ID)),
                    (FORUM_REGISTRY, Some(FORUM_REGISTRY)),
                ],
            )
            .await?;

        let node_keys: NodeKeypairs = serde_json::from_str(
            client
                .namespaced::<Secret>(&ns)
                .store_load(KeySponsorComponent.name(&owner))
                .await?
                .get(KEYS)?,
        )?;

        client
            .run_worker(
                &self.operator,
                network,
                "move-publish",
                instance,
                WorkerOutputComponent(instance),
                MovePublishConfig {
                    worker: ech_board_common::WorkerConfig {
                        network_name: owner.clone(),
                        namespace: ns.to_string(),
                    },
                    repo: self.operator.move_repo.clone(),
                    git_ref: self.operator.move_git_ref.clone(),
                    package_path: self.operator.move_package_path.clone(),
                    publisher_key_base64: node_keys.account_keypair.private_key.clone(),
                    rpc_url: format!(
                        "http://{}:{}",
                        FullnodeRpcComponent.name(&owner),
                        network.spec.fullnode.port_rpc
                    ),
                    original_id: client
                        .namespaced::<Secret>(&ns)
                        .api()
                        .get_opt(&MovePackageComponent.name(&owner))
                        .await?
                        .and_then(|s| s.string_data)
                        .and_then(|d| d.get(MOVE_ORIGINAL_ID).cloned()),
                    output_name: WorkerOutputComponent(instance).name(&owner),
                },
            )
            .await
    }

    async fn cleanup(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<()> {
        let ns = network.cr_ns()?;
        let owner = network.cr_name()?;
        let instance = MovePublishComponent::new(&self.operator.move_git_ref)?;
        let config = WorkerConfigComponent(instance);
        let output = WorkerOutputComponent(instance);

        client
            .namespaced::<Job>(&ns)
            .delete_if_exists(instance.name(&owner))
            .await?;
        client
            .namespaced::<Secret>(&ns)
            .delete_if_exists(config.name(&owner))
            .await?;
        client
            .namespaced::<Secret>(&ns)
            .delete_if_exists(output.name(&owner))
            .await?;
        client
            .namespaced::<ExternalSecret>(&ns)
            .delete_if_exists(instance.name(&owner))
            .await?;
        client
            .namespaced::<PushSecret>(&ns)
            .delete_if_exists(instance.name(&owner))
            .await?;
        Ok(())
    }
}
