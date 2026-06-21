mod turnstile;

use crate::config::CaptchaConfig;
use crate::error::RelayError;
use reqwest::Client;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub(super) type VerifyFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(), RelayError>> + Send + 'a>>;

pub(super) trait CaptchaProvider: Send + Sync {
    fn verify<'a>(&'a self, token: &'a str, remote_ip: Option<&'a str>) -> VerifyFuture<'a>;
}

struct DisabledProvider;

impl CaptchaProvider for DisabledProvider {
    fn verify<'a>(&'a self, _token: &'a str, _remote_ip: Option<&'a str>) -> VerifyFuture<'a> {
        Box::pin(async { Ok(()) })
    }
}

#[derive(Clone)]
pub struct CaptchaVerifier {
    provider: Arc<dyn CaptchaProvider>,
}

impl CaptchaVerifier {
    pub fn new(client: Client, config: CaptchaConfig) -> Self {
        let provider: Arc<dyn CaptchaProvider> = match config {
            CaptchaConfig::Disabled => Arc::new(DisabledProvider),
            CaptchaConfig::Turnstile(cfg) => {
                Arc::new(turnstile::TurnstileProvider::new(client, cfg))
            }
        };
        Self { provider }
    }

    pub async fn verify(&self, token: &str, remote_ip: Option<&str>) -> Result<(), RelayError> {
        self.provider.verify(token, remote_ip).await
    }
}
