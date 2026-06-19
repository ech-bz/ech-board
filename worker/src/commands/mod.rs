pub(crate) mod download_db_snapshot;
pub(crate) mod download_genesis;
pub(crate) mod genesis;
pub(crate) mod keys;
pub(crate) mod move_publish;
pub(crate) mod seed_peers;

use aws_sdk_s3::Client as S3Client;
use ech_board_common::keys::GENESIS_BLOB;
use std::path::Path;

pub(crate) async fn download_genesis_blob(
    s3: &S3Client,
    bucket: &str,
    dest: &Path,
) -> anyhow::Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = s3
        .get_object()
        .bucket(bucket)
        .key(GENESIS_BLOB)
        .send()
        .await?
        .body
        .collect()
        .await?
        .into_bytes();
    std::fs::write(dest, bytes)?;
    Ok(())
}
