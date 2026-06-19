use crate::constants::WORKER_SERVICE_ACCOUNT_NAME;
use crate::support::components::{EsComponent, S3CredsComponent};
use crate::support::extensions::ExternalSecretExt;
use crate::{
    crds::{EchBoardNetwork, ExternalSecret},
    error::Result,
};
use ech_board_common::keys::{S3_ACCESS_KEY, S3_SECRET_KEY};
use ech_k8s::{Component, CrMeta, K8sClient, NodeState, Reconciler, ResourcesExt};
use k8s_openapi::api::{
    core::v1::{Secret, ServiceAccount},
    rbac::v1::{Role, RoleBinding, RoleRef, Subject},
};
use kube::api::ObjectMeta;
use serde::Serialize;

#[derive(Clone, Serialize)]
pub(crate) struct BootstrapReconciler;

#[async_trait::async_trait]
impl Reconciler for BootstrapReconciler {
    type Crd = EchBoardNetwork;
    type Error = crate::error::OperatorError;

    async fn reconcile(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<NodeState> {
        let network_name = network.cr_name()?;
        let ns = network.cr_ns()?;
        let sa_name = WORKER_SERVICE_ACCOUNT_NAME;
        let role_name = sa_name;

        client
            .namespaced::<ServiceAccount>(&ns)
            .apply(
                sa_name,
                &ServiceAccount {
                    metadata: ObjectMeta {
                        name: Some(sa_name.to_string()),
                        namespace: Some(ns.to_string()),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .await?;

        client
            .namespaced::<Role>(&ns)
            .apply(
                role_name,
                &Role {
                    metadata: ObjectMeta {
                        name: Some(role_name.to_string()),
                        namespace: Some(ns.to_string()),
                        ..Default::default()
                    },
                    rules: Some(vec![k8s_openapi::api::rbac::v1::PolicyRule {
                        api_groups: Some(vec!["".into()]),
                        resources: Some(vec!["secrets".into(), "configmaps".into()]),
                        verbs: vec![
                            "create".into(),
                            "get".into(),
                            "patch".into(),
                            "update".into(),
                        ],
                        ..Default::default()
                    }]),
                },
            )
            .await?;

        client
            .namespaced::<RoleBinding>(&ns)
            .apply(
                role_name,
                &RoleBinding {
                    metadata: ObjectMeta {
                        name: Some(role_name.to_string()),
                        namespace: Some(ns.to_string()),
                        ..Default::default()
                    },
                    role_ref: RoleRef {
                        api_group: "rbac.authorization.k8s.io".into(),
                        kind: "Role".into(),
                        name: role_name.to_string(),
                    },
                    subjects: Some(vec![Subject {
                        kind: "ServiceAccount".into(),
                        name: sa_name.to_string(),
                        namespace: Some(ns.to_string()),
                        ..Default::default()
                    }]),
                },
            )
            .await?;

        client
            .namespaced::<ExternalSecret>(&ns)
            .apply_external_secret(
                EsComponent(S3CredsComponent).instance_name(&network_name)?,
                EsComponent(S3CredsComponent).labels(&network_name)?,
                S3CredsComponent.instance_name(&network_name)?,
                S3CredsComponent.instance_name(&network_name)?,
                &[
                    (S3_ACCESS_KEY, Some(S3_ACCESS_KEY)),
                    (S3_SECRET_KEY, Some(S3_SECRET_KEY)),
                ],
            )
            .await?;

        Ok(NodeState::Ready)
    }

    async fn cleanup(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<()> {
        let network_name = network.cr_name()?;
        let ns = network.cr_ns()?;
        client
            .namespaced::<RoleBinding>(&ns)
            .delete_if_exists(WORKER_SERVICE_ACCOUNT_NAME)
            .await?;
        client
            .namespaced::<Role>(&ns)
            .delete_if_exists(WORKER_SERVICE_ACCOUNT_NAME)
            .await?;
        client
            .namespaced::<ServiceAccount>(&ns)
            .delete_if_exists(WORKER_SERVICE_ACCOUNT_NAME)
            .await?;
        client
            .namespaced::<ExternalSecret>(&ns)
            .delete_if_exists(EsComponent(S3CredsComponent).instance_name(&network_name)?)
            .await?;
        client
            .namespaced::<Secret>(&ns)
            .delete_if_exists(S3CredsComponent.instance_name(&network_name)?)
            .await?;
        Ok(())
    }
}
