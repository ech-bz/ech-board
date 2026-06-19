use crate::{crds::EchBoardNetwork, error::Result};
use ech_k8s::{Component, CrMeta, K8sClient, NodeState, Reconciler, ResourcesExt};
use k8s_openapi::api::core::v1::{Service, ServicePort, ServiceSpec};
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Serialize, Component)]
#[component(name = "fullnode-rpc")]
pub(crate) struct FullnodeRpcComponent;

#[derive(Clone, Serialize)]
pub(crate) struct FullnodeRpcReconciler;

#[async_trait::async_trait]
impl Reconciler for FullnodeRpcReconciler {
    type Crd = EchBoardNetwork;
    type Error = crate::error::OperatorError;

    async fn reconcile(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<NodeState> {
        let network_name = network.cr_name()?;
        let instance_name = FullnodeRpcComponent.instance_name(&network_name)?;

        client
            .namespaced::<Service>(&network.cr_ns()?)
            .apply(
                &instance_name,
                &Service {
                    metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                        name: Some(instance_name.clone()),
                        labels: Some(FullnodeRpcComponent.labels(&network_name)?),
                        ..Default::default()
                    },
                    spec: Some(ServiceSpec {
                        selector: Some(BTreeMap::from([
                            ("ech.bz/sui-role".into(), "fullnode".into()),
                            ("ech.bz/owner".into(), network_name.clone()),
                        ])),
                        ports: Some(vec![ServicePort {
                            name: Some("rpc".into()),
                            port: network.spec.fullnode.port_rpc as i32,
                            target_port: Some(IntOrString::Int(
                                network.spec.fullnode.port_rpc as i32,
                            )),
                            ..Default::default()
                        }]),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .await?;

        Ok(NodeState::Ready)
    }

    async fn cleanup(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<()> {
        client
            .namespaced::<Service>(&network.cr_ns()?)
            .delete_if_exists(&FullnodeRpcComponent.instance_name(&network.cr_name()?)?)
            .await?;
        Ok(())
    }
}
