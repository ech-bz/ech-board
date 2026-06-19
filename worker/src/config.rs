use ech_board_common::keys::{S3_ACCESS_KEY, S3_SECRET_KEY};
use s3::{Auth, Credentials};
use std::{fs, path::Path};

pub(crate) fn load_job_config<T: serde::de::DeserializeOwned>(path: &Path) -> anyhow::Result<T> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

pub(crate) fn s3_auth(creds_dir: &str) -> anyhow::Result<Auth> {
    Ok(Auth::Static(Credentials::new(
        fs::read_to_string(Path::new(creds_dir).join(S3_ACCESS_KEY))?.to_string(),
        fs::read_to_string(Path::new(creds_dir).join(S3_SECRET_KEY))?.to_string(),
    )?))
}
