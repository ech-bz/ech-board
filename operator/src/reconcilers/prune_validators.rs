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
        let network_name = network.cr_name()?;
        let namespace = network.cr_ns()?;
        let workload_selector = WorkloadValidatorComponent::selector(&network_name);
        let desired_instances: BTreeSet<String> = (0..network.spec.validator.replicas as usize)
            .map(|ordinal| WorkloadValidatorComponent { ordinal }.instance_name(&network_name))
            .collect::<std::result::Result<_, _>>()?;

        client
            .namespaced::<Secret>(&namespace)
            .prune(&workload_selector, &desired_instances)
            .await?;
        client
            .namespaced::<Service>(&namespace)
            .prune(&workload_selector, &desired_instances)
            .await?;
        client
            .namespaced::<StatefulSet>(&namespace)
            .prune(&workload_selector, &desired_instances)
            .await?;

        Ok(NodeState::Ready)
    }
}
