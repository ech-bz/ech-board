use crate::reconcilers::workload_validators::WorkloadValidatorComponent;
use crate::{crds::EchBoardNetwork, error::Result};
use ech_k8s::{Component, CrMeta, K8sClient, NodeState, Reconciler, ResourcesExt};
use k8s_openapi::api::{
    apps::v1::StatefulSet,
    core::v1::{Secret, Service},
};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Serialize)]
pub(crate) struct PruneValidatorsReconciler;

#[async_trait::async_trait]
impl Reconciler for PruneValidatorsReconciler {
    type Crd = EchBoardNetwork;
    type Error = crate::error::OperatorError;

    async fn reconcile(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<NodeState> {
        let ns = network.cr_ns()?;
        let owner = network.cr_name()?;
        let workload_selector = WorkloadValidatorComponent::selector(&owner);
        let desired_instances: BTreeSet<String> = (0..network.spec.validator.replicas as usize)
            .map(|ordinal| WorkloadValidatorComponent { ordinal }.name(&owner))
            .collect();

        client
            .namespaced::<Secret>(&ns)
            .prune(&workload_selector, &desired_instances)
            .await?;
        client
            .namespaced::<Service>(&ns)
            .prune(&workload_selector, &desired_instances)
            .await?;
        client
            .namespaced::<StatefulSet>(&ns)
            .prune(&workload_selector, &desired_instances)
            .await?;

        Ok(NodeState::Ready)
    }
}
