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
        let network_name = network.cr_name()?;
        let namespace = network.cr_ns()?;

        client
            .namespaced::<Secret>(&namespace)
            .prune(
                KeyValidatorComponent::selector(&network_name),
                &(0..network.spec.validator.replicas as usize)
                    .map(|ordinal| KeyValidatorComponent { ordinal }.instance_name(&network_name))
                    .collect::<std::result::Result<_, _>>()?,
            )
            .await?;

        client
            .namespaced::<Secret>(&namespace)
            .prune(
                KeyFullnodeComponent::selector(&network_name),
                &(0..network.spec.fullnode.replicas as usize)
                    .map(|ordinal| KeyFullnodeComponent { ordinal }.instance_name(&network_name))
                    .collect::<std::result::Result<_, _>>()?,
            )
            .await?;

        client
            .namespaced::<Secret>(&namespace)
            .prune(
                KeyArchiveComponent::selector(&network_name),
                &std::iter::once(KeyArchiveComponent.instance_name(&network_name))
                    .collect::<std::result::Result<_, _>>()?,
            )
            .await?;

        client
            .namespaced::<Secret>(&namespace)
            .prune(
                KeySponsorComponent::selector(&network_name),
                &std::iter::once(KeySponsorComponent.instance_name(&network_name))
                    .collect::<std::result::Result<_, _>>()?,
            )
            .await?;

        Ok(NodeState::Ready)
    }
}
