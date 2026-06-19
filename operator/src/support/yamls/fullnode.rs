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
    authority_store_pruning_config: AuthorityStorePruningConfig,
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
struct AuthorityStorePruningConfig {
    num_latest_epoch_dbs_to_retain: u32,
    epoch_db_pruning_period_secs: u64,
    num_epochs_to_retain: u32,
    num_epochs_to_retain_for_checkpoints: u32,
    pruning_run_delay_seconds: u64,
    max_checkpoints_in_batch: u64,
    max_transactions_in_batch: u64,
    periodic_compaction_threshold_days: u64,
}

pub(crate) fn render_read(
    network: &EchBoardNetwork,
    ordinal: usize,
    keys: &ech_board_common::NodeKeypairs,
    seed_peers: &Vec<ech_board_common::ValidatorSeedPeer>,
) -> Result<String> {
    let owner = network.cr_name()?;
    render(
        network,
        keys,
        seed_peers,
        WorkloadFullnodeComponent { ordinal }.name(&owner),
    )
}

fn render(
    network: &EchBoardNetwork,
    keys: &ech_board_common::NodeKeypairs,
    seed_peers: &Vec<ech_board_common::ValidatorSeedPeer>,
    service_name: String,
) -> Result<String> {
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
        enable_index_processing: false,
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
        authority_store_pruning_config: AuthorityStorePruningConfig {
            num_latest_epoch_dbs_to_retain: 1,
            epoch_db_pruning_period_secs: 3600,
            num_epochs_to_retain: 0,
            num_epochs_to_retain_for_checkpoints: 2,
            pruning_run_delay_seconds: 60,
            max_checkpoints_in_batch: 1000,
            max_transactions_in_batch: 1000,
            periodic_compaction_threshold_days: 1,
        },
    };
    Ok(serde_saphyr::to_string(&fullnode_config)
        .map_err(|err| OperatorError::ControllerFatal(err.to_string()))?)
}
