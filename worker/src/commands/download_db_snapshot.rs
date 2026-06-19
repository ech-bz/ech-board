use crate::config::{build_s3_client, load_job_config};
use ech_board_common::DbSnapshotConfig;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct Manifest {
    available_epochs: Vec<u64>,
}

pub(crate) async fn run(_ctx: &crate::Ctx, config_path: &Path) -> anyhow::Result<()> {
    let config: DbSnapshotConfig = load_job_config(config_path)?;
    let s3 = build_s3_client(&config.s3);

    let manifest = s3
        .get_object()
        .bucket(&config.s3.bucket)
        .key("MANIFEST")
        .send()
        .await;
    let manifest: Manifest = match manifest {
        Ok(resp) => serde_json::from_slice(&resp.body.collect().await?.into_bytes())?,
        Err(_) => {
            tracing::info!("no MANIFEST found, skipping");
            return Ok(());
        }
    };
    let latest = manifest.available_epochs.iter().max();
    let Some(&epoch) = latest else {
        tracing::info!("no epochs in MANIFEST, skipping");
        return Ok(());
    };
    let epoch_prefix = format!("epoch_{}/", epoch);

    let db_path = PathBuf::from(&config.db_path).join("live");
    std::fs::create_dir_all(&db_path)?;

    let resp = s3
        .list_objects_v2()
        .bucket(&config.s3.bucket)
        .prefix(&epoch_prefix)
        .send()
        .await?;

    for object in resp.contents() {
        let key = object.key().unwrap_or_default();
        let relative = key.strip_prefix(&epoch_prefix).unwrap_or(key);
        if relative.is_empty() {
            continue;
        }
        let dest = db_path.join(relative);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = s3
            .get_object()
            .bucket(&config.s3.bucket)
            .key(key)
            .send()
            .await?
            .body
            .collect()
            .await?
            .into_bytes();
        std::fs::write(&dest, bytes)?;
    }

    tracing::info!(epoch, db_path = %config.db_path, "db snapshot downloaded");
    Ok(())
}
