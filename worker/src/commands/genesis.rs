use crate::Ctx;
use crate::config::{load_job_config, s3_auth};
use anyhow::Context;
use base64::Engine;
use ech_board_common::keys::GENESIS_BLOB;
use ech_board_common::{GenesisConfig, NodeKeypairs};
use s3::{Client as S3Client, Error as S3Error};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

const GENESIS_OUTPUT_FILE: &str = "genesis.yaml";

fn strip_scheme_byte(key_b64: &str) -> anyhow::Result<String> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(key_b64)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&bytes[1..]))
}

pub(crate) async fn run(ctx: &Ctx, config_path: &Path) -> anyhow::Result<()> {
    let config: GenesisConfig = load_job_config(config_path)?;
    let s3 = S3Client::builder(&config.s3.endpoint)?
        .region(&config.s3.region)
        .auth(s3_auth(&config.s3.creds_dir)?)
        .build()?;

    ensure_genesis_blob_absent(&s3, &config).await?;

    let validator_keypairs = load_validator_keypairs(&config.validator_key_paths)?;
    let sponsor_keypair = load_sponsor_keypair(&config.sponsor_key_path)?;
    let genesis_yaml = render_genesis_yaml(&config, &validator_keypairs, &sponsor_keypair)?;
    let yaml_path = PathBuf::from(&config.work_dir).join(GENESIS_OUTPUT_FILE);
    let work_path = PathBuf::from(&config.work_dir).join("work");
    fs::create_dir_all(&work_path)?;
    fs::write(&yaml_path, genesis_yaml)?;

    upload_genesis_blob(
        &s3,
        &config,
        &ctx.sui.genesis_from_file(yaml_path, work_path, true)?.blob,
    )
    .await?;

    tracing::info!(
        network = %config.worker.network_name,
        bucket = %config.s3.bucket,
        "genesis blob uploaded"
    );

    Ok(())
}

fn load_validator_keypairs(paths: &[String]) -> anyhow::Result<Vec<NodeKeypairs>> {
    paths
        .iter()
        .map(|path| {
            let bytes = fs::read(path)?;
            Ok(serde_json::from_slice(&bytes)?)
        })
        .collect()
}

fn load_sponsor_keypair(path: &str) -> anyhow::Result<NodeKeypairs> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

async fn upload_genesis_blob(
    s3: &S3Client,
    config: &GenesisConfig,
    genesis_blob: &[u8],
) -> anyhow::Result<()> {
    s3.objects()
        .put(&config.s3.bucket, GENESIS_BLOB)
        .content_type("application/octet-stream")
        .body_bytes(genesis_blob.to_vec())
        .send()
        .await?;
    Ok(())
}

async fn ensure_genesis_blob_absent(s3: &S3Client, config: &GenesisConfig) -> anyhow::Result<()> {
    match s3
        .objects()
        .head(&config.s3.bucket, GENESIS_BLOB)
        .send()
        .await
    {
        Ok(_) => Err(anyhow::anyhow!("genesis.blob already exists in S3")),
        Err(S3Error::Api { status, .. }) if status.as_u16() == 404 => Ok(()),
        Err(err) => Err(err.into()),
    }
}

fn render_genesis_yaml(
    config: &GenesisConfig,
    validator_keypairs: &[NodeKeypairs],
    sponsor_keypair: &NodeKeypairs,
) -> anyhow::Result<String> {
    let namespace = &config.worker.namespace;
    let base_port = config.validator_port_p2p;
    let network_port = base_port
        .checked_sub(1)
        .ok_or_else(|| anyhow::anyhow!("validator p2p port must be greater than 0"))?;
    let validators = validator_keypairs
        .iter()
        .enumerate()
        .map(|(ordinal, keys)| {
            let service_name = &config.validator_service_names[ordinal];
            let pod_dns = format!("{service_name}-0.{service_name}.{namespace}.svc.cluster.local");
            Ok(GenesisValidatorConfig {
                key_pair: keys.protocol_keypair.private_key.clone(),
                worker_key_pair: strip_scheme_byte(&keys.worker_keypair.private_key)?,
                account_key_pair: keys.account_keypair.private_key.clone(),
                network_key_pair: strip_scheme_byte(&keys.network_keypair.private_key)?,
                network_address: format!("/dns/{pod_dns}/tcp/{network_port}/https"),
                p2p_address: format!("/dns/{pod_dns}/udp/{base_port}/https"),
                p2p_listen_address: format!("0.0.0.0:{base_port}"),
                metrics_address: format!("0.0.0.0:{}", base_port + 1),
                narwhal_metrics_address: format!("/ip4/0.0.0.0/tcp/{}/https", base_port + 2),
                narwhal_primary_address: format!("/dns/{pod_dns}/udp/{}/https", base_port + 3),
                narwhal_worker_address: format!("/dns/{pod_dns}/udp/{}/https", base_port + 4),
                consensus_address: format!("/dns/{pod_dns}/tcp/{}/https", base_port + 5),
                gas_price: GAS_PRICE,
                commission_rate: COMMISSION_RATE,
                stake: STAKE,
                name: None,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let validator_count = validators.len() as u64;
    let gas_pool = TOTAL_MIST
        .checked_sub(validator_count * (STAKE + DEFAULT_GAS_AMOUNT))
        .context("total pool insufficient for validators")?;
    let base = gas_pool / config.sponsor_gas_object_count as u64;
    let remainder = (gas_pool % config.sponsor_gas_object_count as u64) as usize;

    let config_yaml = GenesisYamlConfig {
        ssfn_config_info: None::<()>,
        validator_config_info: validators,
        parameters: GenesisParameters {
            chain_start_timestamp_ms: 0,
            protocol_version: 124,
            allow_insertion_of_extra_objects: true,
            epoch_duration_ms: 3_600_000,
            stake_subsidy_start_epoch: 0,
            stake_subsidy_initial_distribution_amount: 0,
            stake_subsidy_period_length: 10,
            stake_subsidy_decrease_rate: 1_000,
        },
        accounts: vec![GenesisAccount {
            address: sponsor_keypair.account_keypair.address.clone(),
            gas_amounts: (0..config.sponsor_gas_object_count)
                .map(|i| base + u64::from(i < remainder))
                .collect(),
        }],
    };
    serde_saphyr::to_string(&config_yaml).map_err(Into::into)
}

const TOTAL_MIST: u64 = 10_000_000_000_000_000_000; // 10B SUI
const STAKE: u64 = 1_000_000_000; // 1 SUI per validator
const DEFAULT_GAS_AMOUNT: u64 = 30_000_000_000_000_000; // 30M SUI per validator
const GAS_PRICE: u64 = 1;
const COMMISSION_RATE: u64 = 200;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct GenesisYamlConfig<T = ()> {
    ssfn_config_info: Option<T>,
    validator_config_info: Vec<GenesisValidatorConfig>,
    parameters: GenesisParameters,
    accounts: Vec<GenesisAccount>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct GenesisValidatorConfig {
    key_pair: String,
    worker_key_pair: String,
    account_key_pair: String,
    network_key_pair: String,
    network_address: String,
    p2p_address: String,
    p2p_listen_address: String,
    metrics_address: String,
    narwhal_metrics_address: String,
    gas_price: u64,
    commission_rate: u64,
    narwhal_primary_address: String,
    narwhal_worker_address: String,
    consensus_address: String,
    stake: u64,
    name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct GenesisParameters {
    chain_start_timestamp_ms: u64,
    protocol_version: u64,
    allow_insertion_of_extra_objects: bool,
    epoch_duration_ms: u64,
    stake_subsidy_start_epoch: u64,
    stake_subsidy_initial_distribution_amount: u64,
    stake_subsidy_period_length: u64,
    stake_subsidy_decrease_rate: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct GenesisAccount {
    address: String,
    gas_amounts: Vec<u64>,
}
