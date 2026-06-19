use crate::config::OperatorSettings;
use crate::reconcilers::bootstrap::S3CredsComponent;
use crate::support::components::{WorkerConfigComponent, WorkerOutputComponent};
use crate::support::extensions::WorkerExt;
use crate::{crds::EchBoardNetwork, error::Result};
use ech_board_common::ValidatorsConfig;
use ech_board_common::keys::{S3_ACCESS_KEY, S3_SECRET_KEY};
use ech_k8s::{Component, CrMeta, K8sClient, NodeState, Reconciler, ResourcesExt, StoreExt};
use k8s_openapi::api::{batch::v1::Job, core::v1::Secret};
use serde::Serialize;

#[derive(Clone, Copy, Serialize, Component)]
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
        let ns = network.cr_ns()?;
        let owner = network.cr_name()?;

        let s3_creds = client
            .namespaced::<Secret>(&ns)
            .store_load(S3CredsComponent.name(&owner))
            .await?;

        client
            .run_worker(
                &self.operator,
                network,
                "seed-peers",
                SeedPeersComponent,
                WorkerOutputComponent(SeedPeersComponent),
                ValidatorsConfig {
                    worker: ech_board_common::WorkerConfig {
                        network_name: owner.clone(),
                        namespace: ns.to_string(),
                    },
                    s3: ech_board_common::S3Config {
                        endpoint: network.spec.archive.endpoint.clone(),
                        region: network.spec.archive.region.clone(),
                        bucket: network.spec.archive.bucket.clone(),
                        access_key: s3_creds.get(S3_ACCESS_KEY)?.to_string(),
                        secret_key: s3_creds.get(S3_SECRET_KEY)?.to_string(),
                    },
                    output_name: WorkerOutputComponent(SeedPeersComponent).name(&owner),
                },
            )
            .await
    }

    async fn cleanup(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<()> {
        let ns = network.cr_ns()?;
        let owner = network.cr_name()?;
        let instance = SeedPeersComponent;
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
        Ok(())
    }
}
