use crate::reconcilers::key_node::{
    KeyArchiveComponent, KeyFullnodeComponent, KeySponsorComponent, KeyValidatorComponent,
};
use crate::{crds::EchBoardNetwork, error::Result};
use ech_k8s::{Component, CrMeta, K8sClient, NodeState, Reconciler, ResourcesExt};
use k8s_openapi::api::core::v1::Secret;
use serde::Serialize;

#[derive(Clone, Serialize)]
pub(crate) struct PruneKeysReconciler;

#[async_trait::async_trait]
impl Reconciler for PruneKeysReconciler {
    type Crd = EchBoardNetwork;
    type Error = crate::error::OperatorError;

    async fn reconcile(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<NodeState> {
        let ns = network.cr_ns()?;
        let owner = network.cr_name()?;

        client
            .namespaced::<Secret>(&ns)
            .prune(
                KeyValidatorComponent::selector(&owner),
                &(0..network.spec.validator.replicas as usize)
                    .map(|ordinal| KeyValidatorComponent { ordinal }.name(&owner))
                    .collect(),
            )
            .await?;

        client
            .namespaced::<Secret>(&ns)
            .prune(
                KeyFullnodeComponent::selector(&owner),
                &(0..network.spec.fullnode.replicas as usize)
                    .map(|ordinal| KeyFullnodeComponent { ordinal }.name(&owner))
                    .collect(),
            )
            .await?;

        client
            .namespaced::<Secret>(&ns)
            .prune(
                KeyArchiveComponent::selector(&owner),
                &std::iter::once(KeyArchiveComponent.name(&owner)).collect(),
            )
            .await?;

        client
            .namespaced::<Secret>(&ns)
            .prune(
                KeySponsorComponent::selector(&owner),
                &std::iter::once(KeySponsorComponent.name(&owner)).collect(),
            )
            .await?;

        Ok(NodeState::Ready)
    }
}
