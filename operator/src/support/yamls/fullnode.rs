use serde::Serialize;

use crate::constants::{DB_PATH, GENESIS_BLOB_PATH};
use crate::reconcilers::workload_fullnodes::WorkloadFullnodeComponent;
use crate::support::yamls::KeyPairValue;
use crate::{
    crds::EchBoardNetwork,
    error::{OperatorError, Result},
};
use ech_k8s::{Component, CrMeta};

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct FullnodeYamlConfig {
    protocol_key_pair: KeyPairValue,
    worker_key_pair: KeyPairValue,
    account_key_pair: KeyPairValue,
    network_key_pair: KeyPairValue,
    db_path: String,
    network_address: String,
    json_rpc_address: String,
    rpc: FullnodeRpcConfig,
    metrics_address: String,
    admin_interface_port: u16,
    enable_index_processing: bool,
    sync_post_process_one_tx: bool,
    jsonrpc_server_type: Option<String>,
    grpc_load_shed: Option<String>,
    grpc_concurrency_limit: Option<String>,
    p2p_config: FullnodeP2pConfig,
    genesis: FullnodeGenesisConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    db_checkpoint_config: Option<DbCheckpointConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state_snapshot_write_config: Option<StateSnapshotWriteConfig>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct FullnodeRpcConfig {
    enable_indexing: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct FullnodeP2pConfig {
    listen_address: String,
    external_address: String,
    seed_peers: Vec<ech_board_common::ValidatorSeedPeer>,
    state_sync: FullnodeStateSyncConfig,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct FullnodeStateSyncConfig {
    checkpoint_content_timeout_ms: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct FullnodeGenesisConfig {
    genesis_file_location: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct DbCheckpointConfig {
    perform_db_checkpoints_at_epoch_end: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct StateSnapshotWriteConfig {
    object_store_config: ObjectStoreConfig,
    concurrency: u32,
    archive_interval_epochs: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct ObjectStoreConfig {
    object_store: String,
    bucket: String,
    aws_endpoint: String,
    aws_region: String,
    aws_virtual_hosted_style_request: bool,
}

pub(crate) fn render(
    network: &EchBoardNetwork,
    ordinal: usize,
    keys: &ech_board_common::NodeKeypairs,
    seed_peers: &Vec<ech_board_common::ValidatorSeedPeer>,
) -> Result<String> {
    let network_name = network.cr_name()?;
    let service_name = WorkloadFullnodeComponent { ordinal }.instance_name(&network_name)?;
    let pod_dns = format!("{service_name}-0.{service_name}");
    let fullnode_config = FullnodeYamlConfig {
        protocol_key_pair: KeyPairValue {
            value: keys.protocol_keypair.private_key.clone(),
        },
        worker_key_pair: KeyPairValue {
            value: keys.worker_keypair.private_key.clone(),
        },
        account_key_pair: KeyPairValue {
            value: keys.account_keypair.private_key.clone(),
        },
        network_key_pair: KeyPairValue {
            value: keys.network_keypair.private_key.clone(),
        },
        db_path: DB_PATH.into(),
        network_address: format!(
            "/dns/{pod_dns}/tcp/{}/https",
            network.spec.fullnode.port_net
        ),
        json_rpc_address: format!("0.0.0.0:{}", network.spec.fullnode.port_rpc),
        rpc: FullnodeRpcConfig {
            enable_indexing: true,
        },
        metrics_address: format!("0.0.0.0:{}", network.spec.fullnode.port_metrics),
        admin_interface_port: network.spec.fullnode.port_admin as u16,
        enable_index_processing: true,
        sync_post_process_one_tx: false,
        jsonrpc_server_type: None,
        grpc_load_shed: None,
        grpc_concurrency_limit: None,
        p2p_config: FullnodeP2pConfig {
            listen_address: format!("0.0.0.0:{}", network.spec.fullnode.port_p2p),
            external_address: format!("/dns/{pod_dns}/udp/{}", network.spec.fullnode.port_p2p),
            seed_peers: seed_peers.to_vec(),
            state_sync: FullnodeStateSyncConfig {
                checkpoint_content_timeout_ms: 10_000,
            },
        },
        genesis: FullnodeGenesisConfig {
            genesis_file_location: GENESIS_BLOB_PATH.into(),
        },
        db_checkpoint_config: None,
        state_snapshot_write_config: None,
    };
    Ok(serde_saphyr::to_string(&fullnode_config)
        .map_err(|err| OperatorError::ControllerFatal(err.to_string()))?)
}
