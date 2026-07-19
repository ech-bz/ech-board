use super::{CaptchaProvider, VerifyFuture};
use crate::config::TurnstileConfig;
use crate::error::RelayError;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct TurnstileRequest<'a> {
    secret: &'a str,
    response: &'a str,
    remoteip: &'a str,
}

#[derive(Deserialize)]
struct TurnstileResponse {
    success: bool,
}

pub struct TurnstileProvider {
    client: Client,
    config: TurnstileConfig,
}

impl TurnstileProvider {
    pub fn new(client: Client, config: TurnstileConfig) -> Self {
        Self { client, config }
    }
}

impl CaptchaProvider for TurnstileProvider {
    fn verify<'a>(&'a self, token: &'a str, remote_ip: &'a str) -> VerifyFuture<'a> {
        Box::pin(async move {
            let req = TurnstileRequest {
                secret: &self.config.secret,
                response: token,
                remoteip: remote_ip,
            };

            let resp = self
                .client
                .post(&self.config.verify_url)
                .form(&req)
                .send()
                .await
                .map_err(RelayError::CaptchaRequest)?;

            let parsed = resp
                .json::<TurnstileResponse>()
                .await
                .map_err(RelayError::CaptchaDecode)?;

            if parsed.success {
                Ok(())
            } else {
                Err(RelayError::CaptchaRejected)
            }
        })
    }
}
