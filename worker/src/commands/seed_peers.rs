use crate::config::{build_s3_client, load_job_config};
use crate::{Ctx, sui_cli::tool::DumpValidatorsOutput};
use ech_board_common::keys::SEED_PEERS;
use ech_board_common::{ValidatorSeedPeer, ValidatorsConfig};
use ech_k8s::StoreExt;
use k8s_openapi::api::core::v1::Secret;
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) async fn run(ctx: &Ctx, config_path: &Path) -> anyhow::Result<()> {
    let config: ValidatorsConfig = load_job_config(config_path)?;
    let s3 = build_s3_client(&config.s3);

    let genesis_path = std::path::PathBuf::from("/tmp/ech-board-worker-genesis.blob");
    super::download_genesis_blob(&s3, &config.s3.bucket, &genesis_path).await?;
    let validators = ctx
        .sui
        .tool()
        .dump_validators(&genesis_path)?
        .into_iter()
        .map(|peer: DumpValidatorsOutput| ValidatorSeedPeer {
            address: peer.address,
            peer_id: peer.peer_id,
        })
        .collect::<Vec<_>>();
    ctx.k8s
        .namespaced::<Secret>(&config.worker.namespace)
        .store_put(
            &config.output_name,
            BTreeMap::new(),
            BTreeMap::from([(SEED_PEERS.to_string(), serde_json::to_string(&validators)?)]),
        )
        .await?;

    tracing::info!(
        network = %config.worker.network_name,
        count = %validators.len(),
        "validator seed peers stored"
    );
    Ok(())
}
