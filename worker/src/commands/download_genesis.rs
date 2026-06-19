use crate::config::{build_s3_client, load_job_config};
use ech_board_common::{GenesisDownloadConfig, keys::GENESIS_BLOB};
use std::path::Path;

pub(crate) async fn run(_ctx: &crate::Ctx, config_path: &Path) -> anyhow::Result<()> {
    let config: GenesisDownloadConfig = load_job_config(config_path)?;
    let s3 = build_s3_client(&config.s3);
    let dest = Path::new(&config.genesis_dir).join(GENESIS_BLOB);
    super::download_genesis_blob(&s3, &config.s3.bucket, &dest).await?;

    tracing::info!(dest = %dest.display(), "genesis downloaded");
    Ok(())
}
