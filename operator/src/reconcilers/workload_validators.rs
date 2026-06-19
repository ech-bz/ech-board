use crate::config::OperatorSettings;
use crate::constants::{GENESIS_MOUNT_DIR, NODE_CONFIG_KEY, WORKER_CONFIG_FILE_NAME};
use crate::reconcilers::bootstrap::S3CredsComponent;
use crate::reconcilers::key_node::KeyValidatorComponent;
use crate::support::components::WorkerConfigComponent;
use crate::support::extensions::{HeadlessServiceExt, ReadyReplicasExt, SingletonStatefulSetExt};
use crate::support::pod_builder::SuiNodePodBuilder;
use crate::support::yamls;
use crate::{crds::EchBoardNetwork, error::Result};
use ech_board_common::keys::{KEYS, S3_ACCESS_KEY, S3_SECRET_KEY};
use ech_k8s::{Component, CrMeta, K8sClient, NodeState, Reconciler, ResourcesExt, StoreExt};
use k8s_openapi::api::apps::v1::StatefulSet;
use k8s_openapi::api::core::v1::{ContainerPort, PodTemplateSpec, Secret, Service};
use kube::api::ObjectMeta;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Serialize, Component)]
#[component(name = "workload-validator")]
pub(crate) struct WorkloadValidatorComponent {
    pub(crate) ordinal: usize,
}

#[derive(Clone, Serialize)]

pub(crate) struct WorkloadValidatorReconciler {
    pub(crate) ordinal: usize,
    pub(crate) operator: OperatorSettings,
}

#[async_trait::async_trait]
impl Reconciler for WorkloadValidatorReconciler {
    type Crd = EchBoardNetwork;
    type Error = crate::error::OperatorError;

    async fn reconcile(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<NodeState> {
        let ns = network.cr_ns()?;
        let owner = network.cr_name()?;
        let instance = WorkloadValidatorComponent {
            ordinal: self.ordinal,
        };
        let genesis_config = WorkerConfigComponent(instance);
        let s3_creds = S3CredsComponent;
        let node_key = KeyValidatorComponent {
            ordinal: self.ordinal,
        };

        let s3_creds = client
            .namespaced::<Secret>(&ns)
            .store_load(s3_creds.name(&owner))
            .await?;

        client
            .namespaced::<Secret>(&ns)
            .store_put(
                instance.name(&owner),
                instance.labels(&owner),
                BTreeMap::from([(
                    NODE_CONFIG_KEY.to_string(),
                    yamls::validator::render(
                        network,
                        self.ordinal,
                        &serde_json::from_str(
                            &client
                                .namespaced::<Secret>(&ns)
                                .store_load(node_key.name(&owner))
                                .await?
                                .get(KEYS)?,
                        )?,
                        s3_creds.get(S3_ACCESS_KEY)?,
                        s3_creds.get(S3_SECRET_KEY)?,
                    )?,
                )]),
            )
            .await?;

        client
            .namespaced::<Secret>(&ns)
            .store_put(
                genesis_config.name(&owner),
                genesis_config.labels(&owner),
                BTreeMap::from([(
                    WORKER_CONFIG_FILE_NAME.to_string(),
                    serde_json::to_string(&ech_board_common::GenesisDownloadConfig {
                        s3: ech_board_common::S3Config {
                            endpoint: network.spec.archive.endpoint.clone(),
                            region: network.spec.archive.region.clone(),
                            bucket: network.spec.archive.bucket.clone(),
                            access_key: s3_creds.get(S3_ACCESS_KEY)?.to_string(),
                            secret_key: s3_creds.get(S3_SECRET_KEY)?.to_string(),
                        },
                        genesis_dir: GENESIS_MOUNT_DIR.into(),
                    })?,
                )]),
            )
            .await?;

        client
            .namespaced::<Service>(&ns)
            .apply_headless_service(
                instance.name(&owner),
                instance.labels(&owner),
                "p2p",
                network.spec.validator.port_p2p as i32,
            )
            .await?;

        client
            .namespaced::<StatefulSet>(&ns)
            .apply_singleton_stateful_set(
                instance.name(&owner),
                instance.labels(&owner),
                PodTemplateSpec {
                    metadata: Some(ObjectMeta {
                        labels: Some(instance.labels(&owner)),
                        ..Default::default()
                    }),
                    spec: Some(
                        SuiNodePodBuilder {
                            image: self.operator.sui_node_image.clone(),
                            worker_image: Some(self.operator.worker_image.clone()),
                            genesis_config_secret_name: genesis_config.name(&owner),
                            db_config_secret_name: None,
                            component_name: "validator".into(),
                            config_secret_name: instance.name(&owner),
                            ports: vec![ContainerPort {
                                name: Some("p2p".into()),
                                container_port: network.spec.validator.port_p2p as i32,
                                ..Default::default()
                            }],
                            cpu: network.spec.validator.cpu.clone(),
                            memory: network.spec.validator.memory.clone(),
                            network,
                        }
                        .build()?,
                    ),
                },
                &network.spec.validator.storage.size,
                network.spec.validator.storage.class_name.clone(),
            )
            .await?;

        if client
            .namespaced::<StatefulSet>(&ns)
            .ready_replicas(instance.name(&owner))
            .await?
            >= 1
        {
            Ok(NodeState::Ready)
        } else {
            Ok(NodeState::Pending)
        }
    }

    async fn cleanup(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<()> {
        let ns = network.cr_ns()?;
        let owner = network.cr_name()?;
        let instance = WorkloadValidatorComponent {
            ordinal: self.ordinal,
        };
        let genesis_config = WorkerConfigComponent(instance);
        client
            .namespaced::<Secret>(&ns)
            .delete_if_exists(instance.name(&owner))
            .await?;
        client
            .namespaced::<Secret>(&ns)
            .delete_if_exists(genesis_config.name(&owner))
            .await?;
        client
            .namespaced::<Service>(&ns)
            .delete_if_exists(instance.name(&owner))
            .await?;
        client
            .namespaced::<StatefulSet>(&ns)
            .delete_if_exists(instance.name(&owner))
            .await?;
        Ok(())
    }
}
