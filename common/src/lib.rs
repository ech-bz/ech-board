use serde::{Deserialize, Serialize};

pub mod keys {
    pub const SEED_PEERS: &str = "seed-peers.json";
    pub const KEYS: &str = "keys.json";
    pub const S3_ACCESS_KEY: &str = "S3_ACCESS_KEY";
    pub const S3_SECRET_KEY: &str = "S3_SECRET_KEY";
    pub const GENESIS_BLOB: &str = "genesis.blob";
    pub const MOVE_REF: &str = "move_ref";
    pub const MOVE_ORIGINAL_ID: &str = "move_original_id";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiKeypair {
    pub address: String,
    pub private_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeKeypairs {
    pub protocol_keypair: SuiKeypair,
    pub worker_keypair: SuiKeypair,
    pub account_keypair: SuiKeypair,
    pub network_keypair: SuiKeypair,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorSeedPeer {
    pub address: String,
    pub peer_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    pub network_name: String,
    pub namespace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub creds_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysConfig {
    pub worker: WorkerConfig,
    pub secret_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisConfig {
    pub worker: WorkerConfig,
    pub s3: S3Config,
    pub validator_key_paths: Vec<String>,
    pub validator_service_names: Vec<String>,
    pub sponsor_key_path: String,
    pub validator_port_p2p: u32,
    pub sponsor_gas_object_count: usize,
    pub work_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorsConfig {
    pub worker: WorkerConfig,
    pub s3: S3Config,
    pub config_map_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovePublishConfig {
    pub worker: WorkerConfig,
    pub repo: String,
    pub git_ref: String,
    pub package_path: String,
    pub publisher_key_base64: String,
    pub config_map_name: String,
    pub rpc_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbSnapshotConfig {
    pub node_config_path: String,
    pub db_path: String,
}
