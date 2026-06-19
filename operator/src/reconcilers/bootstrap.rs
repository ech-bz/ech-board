use crate::constants::WORKER_SERVICE_ACCOUNT_NAME;
use crate::crds::{EchBoardNetwork, ExternalSecret};
use crate::error::Result;
use crate::support::extensions::ExternalSecretExt;
use ech_board_common::keys::{S3_ACCESS_KEY, S3_SECRET_KEY};
use ech_k8s::{Component, CrMeta, K8sClient, NodeState, Reconciler, ResourcesExt};
use k8s_openapi::api::{
    core::v1::{Secret, ServiceAccount},
    rbac::v1::{Role, RoleBinding, RoleRef, Subject},
};
use kube::api::ObjectMeta;
use serde::Serialize;

#[derive(Clone, Copy, Serialize, Component)]
#[component(name = "s3-creds")]
pub(crate) struct S3CredsComponent;

#[derive(Clone, Copy, Serialize)]
pub(crate) struct BootstrapReconciler;

#[async_trait::async_trait]
impl Reconciler for BootstrapReconciler {
    type Crd = EchBoardNetwork;
    type Error = crate::error::OperatorError;

    async fn reconcile(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<NodeState> {
        let ns = network.cr_ns()?;
        let owner = network.cr_name()?;

        client
            .namespaced::<ServiceAccount>(&ns)
            .apply(
                WORKER_SERVICE_ACCOUNT_NAME,
                &ServiceAccount {
                    metadata: ObjectMeta {
                        name: Some(WORKER_SERVICE_ACCOUNT_NAME.to_string()),
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
                WORKER_SERVICE_ACCOUNT_NAME,
                &Role {
                    metadata: ObjectMeta {
                        name: Some(WORKER_SERVICE_ACCOUNT_NAME.to_string()),
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
                WORKER_SERVICE_ACCOUNT_NAME,
                &RoleBinding {
                    metadata: ObjectMeta {
                        name: Some(WORKER_SERVICE_ACCOUNT_NAME.to_string()),
                        namespace: Some(ns.to_string()),
                        ..Default::default()
                    },
                    role_ref: RoleRef {
                        api_group: "rbac.authorization.k8s.io".into(),
                        kind: "Role".into(),
                        name: WORKER_SERVICE_ACCOUNT_NAME.to_string(),
                    },
                    subjects: Some(vec![Subject {
                        kind: "ServiceAccount".into(),
                        name: WORKER_SERVICE_ACCOUNT_NAME.to_string(),
                        namespace: Some(ns.to_string()),
                        ..Default::default()
                    }]),
                },
            )
            .await?;

        client
            .namespaced::<ExternalSecret>(&ns)
            .apply_external_secret(
                &owner,
                S3CredsComponent,
                S3CredsComponent,
                &[
                    (S3_ACCESS_KEY, Some(S3_ACCESS_KEY)),
                    (S3_SECRET_KEY, Some(S3_SECRET_KEY)),
                ],
            )
            .await?;

        Ok(NodeState::Ready)
    }

    async fn cleanup(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<()> {
        let ns = network.cr_ns()?;
        let owner = network.cr_name()?;
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
            .delete_if_exists(S3CredsComponent.name(&owner))
            .await?;
        client
            .namespaced::<Secret>(&ns)
            .delete_if_exists(S3CredsComponent.name(&owner))
            .await?;
        Ok(())
    }
}
