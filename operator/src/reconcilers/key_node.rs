use crate::config::OperatorSettings;
use crate::crds::{EchBoardNetwork, ExternalSecret, PushSecret};
use crate::error::Result;
use crate::support::components::{WorkerConfigComponent, WorkerOutputComponent};
use crate::support::extensions::{ExternalSecretExt, PushSecretExt, WorkerExt};
use ech_board_common::{KeysConfig, keys::KEYS};
use ech_k8s::{Component, CrMeta, K8sClient, NodeState, Reconciler, ResourcesExt};
use k8s_openapi::api::batch::v1::Job;
use k8s_openapi::api::core::v1::Secret;
use serde::Serialize;

#[derive(Clone, Copy, Serialize, Component)]
#[component(name = "key-validator")]
pub(crate) struct KeyValidatorComponent {
    pub(crate) ordinal: usize,
}

#[derive(Clone, Copy, Serialize, Component)]
#[component(name = "key-fullnode")]
pub(crate) struct KeyFullnodeComponent {
    pub(crate) ordinal: usize,
}

#[derive(Clone, Copy, Serialize, Component)]
#[component(name = "key-archive")]
pub(crate) struct KeyArchiveComponent;

#[derive(Clone, Copy, Serialize, Component)]
#[component(name = "key-sponsor")]
pub(crate) struct KeySponsorComponent;

#[derive(Clone, Serialize)]
pub(crate) struct KeyNodeReconciler<C: Component> {
    pub(crate) component: C,
    pub(crate) operator: OperatorSettings,
}

impl<C: Component> KeyNodeReconciler<C> {
    pub(crate) fn new(component: C, operator: OperatorSettings) -> Self {
        Self {
            component,
            operator,
        }
    }
}

#[async_trait::async_trait]
impl<C: Component> Reconciler for KeyNodeReconciler<C> {
    type Crd = EchBoardNetwork;
    type Error = crate::error::OperatorError;

    async fn reconcile(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<NodeState> {
        let ns = network.cr_ns()?;
        let owner = network.cr_name()?;

        client
            .namespaced::<PushSecret>(&ns)
            .apply_push_secret(
                &owner,
                WorkerOutputComponent(self.component),
                self.component,
                &[KEYS],
            )
            .await?;

        client
            .namespaced::<ExternalSecret>(&ns)
            .apply_external_secret(
                &owner,
                self.component,
                self.component,
                &[(KEYS, None::<&str>)],
            )
            .await?;

        client
            .run_worker(
                &self.operator,
                network,
                "keys",
                self.component,
                self.component,
                KeysConfig {
                    worker: ech_board_common::WorkerConfig {
                        network_name: owner.clone(),
                        namespace: ns.to_string(),
                    },
                    output_name: WorkerOutputComponent(self.component).name(&owner),
                },
            )
            .await
    }

    async fn cleanup(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<()> {
        let ns = network.cr_ns()?;
        let owner = network.cr_name()?;
        let instance = self.component;
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
