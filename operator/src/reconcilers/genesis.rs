use crate::config::OperatorSettings;
use crate::crds::EchBoardNetwork;
use crate::error::Result;
use crate::reconcilers::bootstrap::S3CredsComponent;
use crate::reconcilers::key_node::{KeySponsorComponent, KeyValidatorComponent};
use crate::reconcilers::workload_validators::WorkloadValidatorComponent;
use crate::support::components::{WorkerConfigComponent, WorkerOutputComponent};
use crate::support::extensions::WorkerExt;
use ech_board_common::GenesisConfig;
use ech_board_common::keys::{KEYS, S3_ACCESS_KEY, S3_SECRET_KEY};
use ech_k8s::{Component, CrMeta, K8sClient, NodeState, Reconciler, ResourcesExt, StoreExt};
use k8s_openapi::api::{batch::v1::Job, core::v1::Secret};
use serde::Serialize;

#[derive(Clone, Copy, Serialize, Component)]
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
        let ns = network.cr_ns()?;
        let owner = network.cr_name()?;
        let job = GenesisComponent;
        let output = WorkerOutputComponent(job);
        let validator_replicas = network.spec.validator.replicas as usize;

        let s3_creds = client
            .namespaced::<Secret>(&ns)
            .store_load(S3CredsComponent.name(&owner))
            .await?;

        let mut validator_keys = vec![];
        for ordinal in 0..validator_replicas {
            validator_keys.push(serde_json::from_str(
                client
                    .namespaced::<Secret>(&ns)
                    .store_load(KeyValidatorComponent { ordinal }.name(&owner))
                    .await?
                    .get(KEYS)?,
            )?);
        }

        let sponsor_key = serde_json::from_str(
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
                "genesis",
                job,
                output,
                GenesisConfig {
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
                    validator_keys,
                    validator_service_names: (0..validator_replicas)
                        .map(|ordinal| WorkloadValidatorComponent { ordinal }.name(&owner))
                        .collect(),
                    sponsor_key,
                    validator_port_p2p: network.spec.validator.port_p2p as u32,
                    sponsor_gas_object_count: network.spec.relay.sponsor.gas_object_count as usize,
                    output_name: output.name(&owner),
                },
            )
            .await
    }

    async fn cleanup(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<()> {
        let ns = network.cr_ns()?;
        let owner = network.cr_name()?;
        client
            .namespaced::<Job>(&ns)
            .delete_if_exists(GenesisComponent.name(&owner))
            .await?;
        client
            .namespaced::<Secret>(&ns)
            .delete_if_exists(WorkerConfigComponent(GenesisComponent).name(&owner))
            .await?;
        client
            .namespaced::<Secret>(&ns)
            .delete_if_exists(WorkerOutputComponent(GenesisComponent).name(&owner))
            .await?;
        Ok(())
    }
}
