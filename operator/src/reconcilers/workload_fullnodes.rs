use crate::config::OperatorSettings;
use crate::constants::{DB_PATH, GENESIS_MOUNT_DIR, NODE_CONFIG_KEY, WORKER_CONFIG_FILE_NAME};
use crate::reconcilers::bootstrap::S3CredsComponent;
use crate::reconcilers::key_node::KeyFullnodeComponent;
use crate::reconcilers::seed_peers::SeedPeersComponent;
use crate::support::components::{
    WorkerConfigComponent, WorkerDownloadDbComponent, WorkerDownloadGenesisComponent,
    WorkerOutputComponent,
};
use crate::support::extensions::{HeadlessServiceExt, ReadyReplicasExt, SingletonStatefulSetExt};
use crate::support::pod_builder::SuiNodePodBuilder;
use crate::support::yamls;
use crate::{crds::EchBoardNetwork, error::Result};
use ech_board_common::keys::{KEYS, S3_ACCESS_KEY, S3_SECRET_KEY, SEED_PEERS};
use ech_k8s::{Component, CrMeta, K8sClient, NodeState, Reconciler, ResourcesExt, StoreExt};
use k8s_openapi::api::apps::v1::StatefulSet;
use k8s_openapi::api::core::v1::{ContainerPort, PodTemplateSpec, Secret, Service};
use kube::api::ObjectMeta;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Serialize, Component)]
#[component(name = "workload-fullnode")]
pub(crate) struct WorkloadFullnodeComponent {
    pub(crate) ordinal: usize,
}

#[derive(Clone, Serialize)]
pub(crate) struct WorkloadFullnodeReconciler {
    pub(crate) ordinal: usize,
    pub(crate) operator: OperatorSettings,
}

#[async_trait::async_trait]
impl Reconciler for WorkloadFullnodeReconciler {
    type Crd = EchBoardNetwork;
    type Error = crate::error::OperatorError;

    async fn reconcile(&self, client: &K8sClient, network: &EchBoardNetwork) -> Result<NodeState> {
        let ns = network.cr_ns()?;
        let owner = network.cr_name()?;
        let instance = WorkloadFullnodeComponent {
            ordinal: self.ordinal,
        };
        let genesis_config = WorkerConfigComponent(WorkerDownloadGenesisComponent(instance));
        let db_config = WorkerConfigComponent(WorkerDownloadDbComponent(instance));
        let s3_creds = S3CredsComponent;
        let node_key = KeyFullnodeComponent {
            ordinal: self.ordinal,
        };
        let seed_peers = WorkerOutputComponent(SeedPeersComponent);

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
                    yamls::fullnode::render_read(
                        network,
                        self.ordinal,
                        &serde_json::from_str(
                            &client
                                .namespaced::<Secret>(&ns)
                                .store_load(node_key.name(&owner))
                                .await?
                                .get(KEYS)?,
                        )?,
                        &serde_json::from_str(
                            &client
                                .namespaced::<Secret>(&ns)
                                .store_load(seed_peers.name(&owner))
                                .await?
                                .get(SEED_PEERS)?,
                        )?,
                        s3_creds.get(S3_ACCESS_KEY)?,
                        s3_creds.get(S3_SECRET_KEY)?,
                    )?,
                )]),
            )
            .await?;

        let s3 = ech_board_common::S3Config {
            endpoint: network.spec.archive.endpoint.clone(),
            region: network.spec.archive.region.clone(),
            bucket: network.spec.archive.bucket.clone(),
            access_key: s3_creds.get(S3_ACCESS_KEY)?.to_string(),
            secret_key: s3_creds.get(S3_SECRET_KEY)?.to_string(),
        };

        client
            .namespaced::<Secret>(&ns)
            .store_put(
                genesis_config.name(&owner),
                genesis_config.labels(&owner),
                BTreeMap::from([(
                    WORKER_CONFIG_FILE_NAME.to_string(),
                    serde_json::to_string(&ech_board_common::GenesisDownloadConfig {
                        s3: s3.clone(),
                        genesis_dir: GENESIS_MOUNT_DIR.into(),
                    })?,
                )]),
            )
            .await?;

        client
            .namespaced::<Secret>(&ns)
            .store_put(
                db_config.name(&owner),
                db_config.labels(&owner),
                BTreeMap::from([(
                    WORKER_CONFIG_FILE_NAME.to_string(),
                    serde_json::to_string(&ech_board_common::DbSnapshotConfig {
                        s3,
                        db_path: DB_PATH.into(),
                    })?,
                )]),
            )
            .await?;

        client
            .namespaced::<Service>(&ns)
            .apply_headless_service(
                instance.name(&owner),
                instance.labels(&owner),
                "rpc",
                network.spec.fullnode.port_rpc as i32,
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
                            db_config_secret_name: Some(db_config.name(&owner)),
                            component_name: "fullnode".into(),
                            config_secret_name: instance.name(&owner),
                            ports: vec![
                                ContainerPort {
                                    name: Some("rpc".into()),
                                    container_port: network.spec.fullnode.port_rpc as i32,
                                    ..Default::default()
                                },
                                ContainerPort {
                                    name: Some("metrics".into()),
                                    container_port: network.spec.fullnode.port_metrics as i32,
                                    ..Default::default()
                                },
                                ContainerPort {
                                    name: Some("p2p".into()),
                                    container_port: network.spec.fullnode.port_p2p as i32,
                                    ..Default::default()
                                },
                                ContainerPort {
                                    name: Some("net".into()),
                                    container_port: network.spec.fullnode.port_net as i32,
                                    ..Default::default()
                                },
                                ContainerPort {
                                    name: Some("admin".into()),
                                    container_port: network.spec.fullnode.port_admin as i32,
                                    ..Default::default()
                                },
                            ],
                            cpu: network.spec.fullnode.cpu.clone(),
                            memory: network.spec.fullnode.memory.clone(),
                            network,
                        }
                        .build()?,
                    ),
                },
                &network.spec.fullnode.storage.size,
                network.spec.fullnode.storage.class_name.clone(),
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
        let instance = WorkloadFullnodeComponent {
            ordinal: self.ordinal,
        };
        let genesis_config = WorkerConfigComponent(WorkerDownloadGenesisComponent(instance));
        let db_config = WorkerConfigComponent(WorkerDownloadDbComponent(instance));

        client
            .namespaced::<Secret>(&ns)
            .delete_if_exists(instance.name(&owner))
            .await?;
        client
            .namespaced::<Secret>(&ns)
            .delete_if_exists(genesis_config.name(&owner))
            .await?;
        client
            .namespaced::<Secret>(&ns)
            .delete_if_exists(db_config.name(&owner))
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
