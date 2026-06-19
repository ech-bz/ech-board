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
#[serde(rename_all = "kebab-case")]
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
    pub access_key: String,
    pub secret_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysConfig {
    pub worker: WorkerConfig,
    pub output_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisConfig {
    pub worker: WorkerConfig,
    pub s3: S3Config,
    pub validator_keys: Vec<NodeKeypairs>,
    pub validator_service_names: Vec<String>,
    pub sponsor_key: NodeKeypairs,
    pub validator_port_p2p: u32,
    pub sponsor_gas_object_count: usize,
    pub output_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorsConfig {
    pub worker: WorkerConfig,
    pub s3: S3Config,
    pub output_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovePublishConfig {
    pub worker: WorkerConfig,
    pub repo: String,
    pub git_ref: String,
    pub package_path: String,
    pub publisher_key_base64: String,
    pub rpc_url: String,
    pub graphql_url: String,
    pub original_id: Option<String>,
    pub output_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbSnapshotConfig {
    pub s3: S3Config,
    pub db_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisDownloadConfig {
    pub s3: S3Config,
    pub genesis_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayServerConfig {
    pub bind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum RelayCaptchaConfig {
    Disabled,
    Turnstile(RelayTurnstileConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayTurnstileConfig {
    pub verify_url: String,
    pub secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayUpstreamConfig {
    pub submit_url: String,
    #[serde(default = "default_relay_timeout")]
    pub request_timeout_ms: u64,
}

fn default_relay_timeout() -> u64 {
    5000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelaySponsorConfig {
    pub private_key_base64: String,
    pub gas_budget: u64,
    pub gas_price: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayConfig {
    pub server: RelayServerConfig,
    pub captcha: RelayCaptchaConfig,
    pub upstream: RelayUpstreamConfig,
    pub sponsor: RelaySponsorConfig,
    pub forum_package_id: String,
    pub graphql_url: String,
}
