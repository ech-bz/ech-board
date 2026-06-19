use crate::reconcilers::workload_fullnodes::WorkloadFullnodeComponent;
use crate::{crds::EchBoardNetwork, error::Result};
use ech_k8s::{Component, CrMeta, K8sClient, NodeState, Reconciler, ResourcesExt};
use k8s_openapi::api::{
    apps::v1::StatefulSet,
    core::v1::{Secret, Service},
};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Serialize)]
pub(crate) struct PruneFullnodesReconciler;

#[async_trait::async_trait]
impl Reconciler for PruneFullnodesReconciler {
    type Crd = EchBoardNetwork;
    type Error = crate::error::OperatorError;

    async fn reconcile(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<NodeState> {
        let owner = network.cr_name()?;
        let ns = network.cr_ns()?;
        let selector = WorkloadFullnodeComponent::selector(&owner);
        let desired_instances: BTreeSet<String> = (0..network.spec.fullnode.replicas as usize)
            .map(|ordinal| WorkloadFullnodeComponent { ordinal }.name(&owner))
            .collect();

        client
            .namespaced::<Secret>(&ns)
            .prune(&selector, &desired_instances)
            .await?;
        client
            .namespaced::<Service>(&ns)
            .prune(&selector, &desired_instances)
            .await?;
        client
            .namespaced::<StatefulSet>(&ns)
            .prune(&selector, &desired_instances)
            .await?;

        Ok(NodeState::Ready)
    }
}
