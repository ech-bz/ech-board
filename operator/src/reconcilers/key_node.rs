use crate::config::OperatorSettings;
use crate::constants::{WORKER_CONFIG_DIR, WORKER_CONFIG_FILE_NAME, WORKER_SERVICE_ACCOUNT_NAME};
use crate::support::components::{EsComponent, PsComponent};
use crate::support::extensions::{ExternalSecretExt, JobArtifactExt, PushSecretExt};
use crate::support::job_builder::JobBuilder;
use crate::{
    crds::{EchBoardNetwork, ExternalSecret, PushSecret},
    error::Result,
};
use ech_board_common::{KeysConfig, keys::KEYS};
use ech_k8s::{Component, CrMeta, K8sClient, NodeState, Reconciler, ResourcesExt, StoreExt};
use k8s_openapi::api::{
    batch::v1::Job,
    core::v1::{ConfigMap, ConfigMapVolumeSource, Secret, Volume, VolumeMount},
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Serialize, Component)]
#[component(name = "key-validator")]
pub(crate) struct KeyValidatorComponent {
    pub(crate) ordinal: usize,
}

#[derive(Clone, Serialize, Component)]
#[component(name = "key-fullnode")]
pub(crate) struct KeyFullnodeComponent {
    pub(crate) ordinal: usize,
}

#[derive(Clone, Serialize, Component)]
#[component(name = "key-archive")]
pub(crate) struct KeyArchiveComponent;

#[derive(Clone, Serialize, Component)]
#[component(name = "key-sponsor")]
pub(crate) struct KeySponsorComponent;

#[derive(Clone, Serialize)]
pub(crate) struct KeyNodeReconciler<C: Component> {
    pub(crate) component: C,
    pub(crate) operator: OperatorSettings,
}

impl<C: Component> KeyNodeReconciler<C> {
    pub(crate) fn new(component: C, operator: OperatorSettings) -> Self {
        Self {
            component,
            operator,
        }
    }
}

#[async_trait::async_trait]
impl<C: Component> Reconciler for KeyNodeReconciler<C> {
    type Crd = EchBoardNetwork;
    type Error = crate::error::OperatorError;

    async fn reconcile(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<NodeState> {
        let network_name = network.cr_name()?;
        let namespace = network.cr_ns()?;
        let instance_name = self.component.instance_name(&network_name)?;
        let labels = self.component.labels(&network_name)?;
        let es_name = EsComponent(self.component.clone()).instance_name(&network_name)?;
        let ps_name = PsComponent(self.component.clone()).instance_name(&network_name)?;

        client
            .namespaced::<ExternalSecret>(&namespace)
            .apply_external_secret(
                &es_name,
                EsComponent(self.component.clone()).labels(&network_name)?,
                &instance_name,
                &es_name,
                &[(KEYS, None::<&str>)],
            )
            .await?;

        client
            .namespaced::<PushSecret>(&namespace)
            .apply_push_secret(
                &ps_name,
                PsComponent(self.component.clone()).labels(&network_name)?,
                &ps_name,
                &es_name,
                &[KEYS],
            )
            .await?;

        client
            .namespaced::<ConfigMap>(&namespace)
            .store_put(
                &instance_name,
                BTreeMap::from([(
                    WORKER_CONFIG_FILE_NAME.to_string(),
                    serde_json::to_string(&KeysConfig {
                        worker: ech_board_common::WorkerConfig {
                            network_name: network_name.clone(),
                            namespace: namespace.to_string(),
                        },
                        secret_name: ps_name.clone(),
                    })?,
                )]),
                Some(labels.clone()),
            )
            .await?;

        client
            .namespaced::<Job>(&namespace)
            .reconcile_job_artifact(
                JobBuilder {
                    name: instance_name.clone(),
                    namespace: namespace.to_string(),
                    image: self.operator.worker_image.clone(),
                    args: vec![
                        "--config".into(),
                        format!("{WORKER_CONFIG_DIR}/{WORKER_CONFIG_FILE_NAME}"),
                        "keys".into(),
                    ],
                    labels: labels.clone(),
                    service_account_name: WORKER_SERVICE_ACCOUNT_NAME.into(),
                    volumes: vec![Volume {
                        name: "job-config".into(),
                        config_map: Some(ConfigMapVolumeSource {
                            name: instance_name.clone(),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }],
                    mounts: vec![VolumeMount {
                        name: "job-config".into(),
                        mount_path: WORKER_CONFIG_DIR.into(),
                        read_only: Some(true),
                        ..Default::default()
                    }],
                }
                .build(),
                || {
                    let instance_name = instance_name.clone();
                    let namespace = namespace.to_string();
                    let client = client.clone();
                    Box::pin(async move {
                        Ok(client
                            .namespaced::<Secret>(&namespace)
                            .api()
                            .get_opt(&instance_name)
                            .await?
                            .is_some())
                    })
                },
            )
            .await
    }

    async fn cleanup(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<()> {
        let network_name = network.cr_name()?;
        let namespace = network.cr_ns()?;
        let instance_name = self.component.instance_name(&network_name)?;
        client
            .namespaced::<ConfigMap>(&namespace)
            .delete_if_exists(&instance_name)
            .await?;
        client
            .namespaced::<Secret>(&namespace)
            .delete_if_exists(&instance_name)
            .await?;
        client
            .namespaced::<ExternalSecret>(&namespace)
            .delete_if_exists(EsComponent(self.component.clone()).instance_name(&network_name)?)
            .await?;
        client
            .namespaced::<PushSecret>(&namespace)
            .delete_if_exists(PsComponent(self.component.clone()).instance_name(&network_name)?)
            .await?;
        Ok(())
    }
}
