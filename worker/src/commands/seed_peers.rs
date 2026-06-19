use crate::config::{load_job_config, s3_auth};
use crate::{Ctx, sui_cli::tool::DumpValidatorsOutput};
use ech_board_common::keys::{GENESIS_BLOB, SEED_PEERS};
use ech_board_common::{ValidatorSeedPeer, ValidatorsConfig};
use ech_k8s::StoreExt;
use k8s_openapi::api::core::v1::ConfigMap;
use s3::Client as S3Client;
use std::collections::BTreeMap;
use std::path::Path;
use std::{fs, path::PathBuf};

const GENESIS_BLOB_PATH: &str = "/tmp/ech-board-worker-genesis.blob";

pub(crate) async fn run(ctx: &Ctx, config_path: &Path) -> anyhow::Result<()> {
    let config: ValidatorsConfig = load_job_config(config_path)?;
    let s3 = S3Client::builder(&config.s3.endpoint)?
        .region(&config.s3.region)
        .auth(s3_auth(&config.s3.creds_dir)?)
        .build()?;

    let genesis_path = download_genesis_blob(&s3, &config).await?;
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
        .namespaced::<ConfigMap>(&config.worker.namespace)
        .store_put(
            &config.config_map_name,
            BTreeMap::from([(SEED_PEERS.to_string(), serde_json::to_string(&validators)?)]),
            None,
        )
        .await?;

    tracing::info!(
        network = %config.worker.network_name,
        count = %validators.len(),
        "validator seed peers stored"
    );
    Ok(())
}

async fn download_genesis_blob(
    s3: &S3Client,
    config: &ValidatorsConfig,
) -> anyhow::Result<PathBuf> {
    let object = s3
        .objects()
        .get(&config.s3.bucket, GENESIS_BLOB)
        .send()
        .await?;
    let bytes = object.bytes().await?;
    let path = PathBuf::from(GENESIS_BLOB_PATH);
    fs::write(&path, bytes.as_ref())?;
    Ok(path)
}
