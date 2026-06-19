use crate::config::load_job_config;
use crate::{Ctx, sui_cli::KeyScheme};
use ech_board_common::keys::KEYS;
use ech_board_common::{KeysConfig, NodeKeypairs};
use ech_k8s::StoreExt;
use k8s_openapi::api::core::v1::Secret;
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) async fn run(ctx: &Ctx, config_path: &Path) -> anyhow::Result<()> {
    let config: KeysConfig = load_job_config(config_path)?;

    let keytool = ctx.sui.keytool();
    let node_keypairs = NodeKeypairs {
        protocol_keypair: keytool.generate(KeyScheme::Bls12381)?,
        worker_keypair: keytool.generate(KeyScheme::Ed25519)?,
        account_keypair: keytool.generate(KeyScheme::Ed25519)?,
        network_keypair: keytool.generate(KeyScheme::Ed25519)?,
    };

    ctx.k8s
        .namespaced::<Secret>(&config.worker.namespace)
        .store_put(
            &config.output_name,
            BTreeMap::new(),
            BTreeMap::from([(KEYS.to_string(), serde_json::to_string(&node_keypairs)?)]),
        )
        .await?;

    tracing::info!(
        network = %config.worker.network_name,
        secret = %config.output_name,
        "node key secret created"
    );

    Ok(())
}
