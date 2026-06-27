use serde::Serialize;

use crate::constants::{DB_PATH, GENESIS_BLOB_PATH, S3_CREDS_DIR};
use crate::reconcilers::{
    workload_archive::WorkloadArchiveComponent, workload_fullnodes::WorkloadFullnodeComponent,
};
use crate::support::yamls::KeyPairValue;
use crate::{
    crds::EchBoardNetwork,
    error::{OperatorError, Result},
};
use ech_board_common::keys::{S3_ACCESS_KEY, S3_SECRET_KEY};
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
    authority_store_pruning_config: Option<AuthorityStorePruningConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    db_checkpoint_config: Option<DbCheckpointConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state_archive_read_config: Option<Vec<StateArchiveReadConfig>>,
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
struct AuthorityStorePruningConfig {
    num_latest_epoch_dbs_to_retain: u32,
    epoch_db_pruning_period_secs: u64,
    num_epochs_to_retain: u32,
    num_epochs_to_retain_for_checkpoints: u32,
    max_checkpoints_in_batch: u64,
    max_transactions_in_batch: u64,
    periodic_compaction_threshold_days: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct StateArchiveReadConfig {
    ingestion_url: String,
    concurrency: u32,
    remote_store_options: Vec<(String, String)>,
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

struct FullnodeMode {
    service_name: String,
    writes_archive: bool,
}

pub(crate) fn render_read(
    network: &EchBoardNetwork,
    ordinal: usize,
    keys: &ech_board_common::NodeKeypairs,
    seed_peers: &Vec<ech_board_common::ValidatorSeedPeer>,
) -> Result<String> {
    let network_name = network.cr_name()?;
    let service_name = WorkloadFullnodeComponent { ordinal }.instance_name(&network_name)?;
    render(network, keys, seed_peers, FullnodeMode { service_name, writes_archive: false })
}

pub(crate) fn render_archive(
    network: &EchBoardNetwork,
    keys: &ech_board_common::NodeKeypairs,
    seed_peers: &Vec<ech_board_common::ValidatorSeedPeer>,
) -> Result<String> {
    let network_name = network.cr_name()?;
    let service_name = WorkloadArchiveComponent.instance_name(&network_name)?;
    render(network, keys, seed_peers, FullnodeMode { service_name, writes_archive: true })
}

fn render(
    network: &EchBoardNetwork,
    keys: &ech_board_common::NodeKeypairs,
    seed_peers: &Vec<ech_board_common::ValidatorSeedPeer>,
    mode: FullnodeMode,
) -> Result<String> {
    let pod_dns = format!("{0}-0.{0}", mode.service_name);
    let archive_endpoint = network.spec.archive.endpoint.trim_end_matches('/');
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
        enable_index_processing: !mode.writes_archive,
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
        authority_store_pruning_config: if mode.writes_archive {
            Some(AuthorityStorePruningConfig {
                num_latest_epoch_dbs_to_retain: 3,
                epoch_db_pruning_period_secs: 3600,
                num_epochs_to_retain: 0,
                num_epochs_to_retain_for_checkpoints: 0,
                max_checkpoints_in_batch: 1000,
                max_transactions_in_batch: 1000,
                periodic_compaction_threshold_days: 1,
            })
        } else {
            Some(AuthorityStorePruningConfig {
                num_latest_epoch_dbs_to_retain: 3,
                epoch_db_pruning_period_secs: 3600,
                num_epochs_to_retain: 1,
                num_epochs_to_retain_for_checkpoints: 2,
                max_checkpoints_in_batch: 1000,
                max_transactions_in_batch: 1000,
                periodic_compaction_threshold_days: 1,
            })
        },
        db_checkpoint_config: mode.writes_archive.then(|| DbCheckpointConfig {
            perform_db_checkpoints_at_epoch_end: true,
        }),
        state_archive_read_config: Some(vec![StateArchiveReadConfig {
            ingestion_url: format!("{archive_endpoint}/{}", network.spec.archive.bucket),
            concurrency: 5,
            remote_store_options: vec![
                (
                    "aws_access_key_id".into(),
                    format!("{S3_CREDS_DIR}/{S3_ACCESS_KEY}"),
                ),
                (
                    "aws_secret_access_key".into(),
                    format!("{S3_CREDS_DIR}/{S3_SECRET_KEY}"),
                ),
            ],
        }]),
        state_snapshot_write_config: mode
            .writes_archive
            .then(|| StateSnapshotWriteConfig {
                object_store_config: ObjectStoreConfig {
                    object_store: "S3".into(),
                    bucket: network.spec.archive.bucket.clone(),
                    aws_endpoint: network.spec.archive.endpoint.clone(),
                    aws_region: network.spec.archive.region.clone(),
                    aws_virtual_hosted_style_request: false,
                },
                concurrency: 5,
                archive_interval_epochs: network.spec.archive.interval_epochs as u64,
            }),
    };
    Ok(serde_saphyr::to_string(&fullnode_config)
        .map_err(|err| OperatorError::ControllerFatal(err.to_string()))?)
}
