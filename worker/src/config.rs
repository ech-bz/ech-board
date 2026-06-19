use aws_credential_types::Credentials;
use aws_sdk_s3::config::{BehaviorVersion, Region, SharedCredentialsProvider};
use ech_board_common::S3Config;
use std::{fs, path::Path};

pub(crate) fn load_job_config<T: serde::de::DeserializeOwned>(path: &Path) -> anyhow::Result<T> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

pub(crate) fn build_s3_client(s3: &S3Config) -> aws_sdk_s3::Client {
    let creds = Credentials::new(&s3.access_key, &s3.secret_key, None, None, "static");
    let sdk_config = aws_sdk_s3::Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new(s3.region.clone()))
        .endpoint_url(&s3.endpoint)
        .credentials_provider(SharedCredentialsProvider::new(creds))
        .force_path_style(true)
        .build();
    aws_sdk_s3::Client::from_conf(sdk_config)
}
