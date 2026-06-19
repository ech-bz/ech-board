use serde::Serialize;

use crate::reconcilers::workload_validators::WorkloadValidatorComponent;
use crate::support::yamls::{Empty, KeyPairValue};
use crate::{
    crds::EchBoardNetwork,
    error::{OperatorError, Result},
};
use ech_k8s::{Component, CrMeta};

use crate::constants::{CONSENSUS_DB_PATH, DB_PATH, GENESIS_BLOB_PATH};

pub(crate) fn render(
    network: &EchBoardNetwork,
    ordinal: usize,
    keys: &ech_board_common::NodeKeypairs,
    s3_access_key: &str,
    s3_secret_key: &str,
) -> Result<String> {
    let owner = network.cr_name()?;
    let service_name = WorkloadValidatorComponent { ordinal }.name(&owner);
    let pod_dns = format!("{service_name}-0.{service_name}");
    let p2p_port = network.spec.validator.port_p2p;

    let config = ValidatorConfig {
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
        network_address: format!("/dns/{pod_dns}/tcp/{}/https", p2p_port - 1),
        json_rpc_address: "127.0.0.1:43747".into(),
        rpc: Empty {},
        metrics_address: format!("0.0.0.0:{}", p2p_port + 1),
        admin_interface_port: 37279,
        consensus_config: ConsensusConfig {
            db_path: CONSENSUS_DB_PATH.into(),
            db_retention_epochs: None,
            db_pruner_period_secs: None,
            max_pending_transactions: None,
            parameters: None,
        },
        enable_index_processing: false,
        authority_store_pruning_config: AuthorityStorePruningConfig {
            num_latest_epoch_dbs_to_retain: 1,
            epoch_db_pruning_period_secs: 3600,
            num_epochs_to_retain: 0,
            num_epochs_to_retain_for_checkpoints: 2,
            max_checkpoints_in_batch: 1000,
            max_transactions_in_batch: 1000,
            pruning_run_delay_seconds: 60,
        },
        sync_post_process_one_tx: false,
        jsonrpc_server_type: None::<String>,
        grpc_load_shed: None::<String>,
        grpc_concurrency_limit: 20_000_000_000u64,
        p2p_config: P2pConfig {
            listen_address: format!("0.0.0.0:{p2p_port}"),
            external_address: format!("/dns/{pod_dns}/udp/{p2p_port}/https"),
            state_sync: StateSyncConfig {
                checkpoint_content_timeout_ms: 10_000,
            },
        },
        genesis: GenesisConfig {
            genesis_file_location: GENESIS_BLOB_PATH.into(),
        },
        db_checkpoint_config: DbCheckpointConfig {
            perform_db_checkpoints_at_epoch_end: true,
            perform_index_db_checkpoints_at_epoch_end: true,
            object_store_config: ObjectStoreConfig {
                object_store: "S3".into(),
                bucket: network.spec.archive.bucket.clone(),
                aws_endpoint: network.spec.archive.endpoint.clone(),
                aws_region: network.spec.archive.region.clone(),
                aws_virtual_hosted_style_request: false,
                aws_access_key_id: Some(s3_access_key.to_string()),
                aws_secret_access_key: Some(s3_secret_key.to_string()),
                object_store_connection_limit: 20,
            },
        },
    };

    Ok(serde_saphyr::to_string(&config)
        .map_err(|err| OperatorError::ControllerFatal(err.to_string()))?)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct ValidatorConfig {
    protocol_key_pair: KeyPairValue,
    worker_key_pair: KeyPairValue,
    account_key_pair: KeyPairValue,
    network_key_pair: KeyPairValue,

    db_path: String,
    network_address: String,
    json_rpc_address: String,
    rpc: Empty,
    metrics_address: String,
    admin_interface_port: u16,

    consensus_config: ConsensusConfig,
    enable_index_processing: bool,
    authority_store_pruning_config: AuthorityStorePruningConfig,
    sync_post_process_one_tx: bool,
    jsonrpc_server_type: Option<String>,
    grpc_load_shed: Option<String>,
    grpc_concurrency_limit: u64,

    p2p_config: P2pConfig,
    genesis: GenesisConfig,
    db_checkpoint_config: DbCheckpointConfig,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct ConsensusConfig {
    db_path: String,
    db_retention_epochs: Option<u64>,
    db_pruner_period_secs: Option<u64>,
    max_pending_transactions: Option<u64>,
    parameters: Option<()>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct P2pConfig {
    listen_address: String,
    external_address: String,
    state_sync: StateSyncConfig,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct StateSyncConfig {
    checkpoint_content_timeout_ms: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct GenesisConfig {
    genesis_file_location: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct DbCheckpointConfig {
    perform_db_checkpoints_at_epoch_end: bool,
    perform_index_db_checkpoints_at_epoch_end: bool,
    object_store_config: ObjectStoreConfig,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct ObjectStoreConfig {
    object_store: String,
    bucket: String,
    aws_endpoint: String,
    aws_region: String,
    aws_virtual_hosted_style_request: bool,
    aws_access_key_id: Option<String>,
    aws_secret_access_key: Option<String>,
    object_store_connection_limit: u64,
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
    pruning_run_delay_seconds: u64,
}
