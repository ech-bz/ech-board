use crate::Ctx;
use std::path::Path;

#[derive(serde::Deserialize)]
struct DbSnapshotConfig {
    node_config_path: String,
    db_path: String,
}

pub(crate) async fn run(ctx: &Ctx, config_path: &Path) -> anyhow::Result<()> {
    let raw = std::fs::read(config_path)?;
    let config: DbSnapshotConfig = serde_json::from_slice(&raw)?;

    ctx.sui
        .tool()
        .download_db_snapshot(
            Path::new(&config.node_config_path),
            Path::new(&config.db_path),
        )?;

    tracing::info!(
        db_path = %config.db_path,
        "db snapshot downloaded"
    );

    Ok(())
}
