use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub captcha: CaptchaConfig,
    pub upstream: UpstreamConfig,
    pub sponsor: SponsorConfig,
    pub forum_package_id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub bind: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum CaptchaConfig {
    Disabled,
    Turnstile(TurnstileConfig),
}

#[derive(Debug, Deserialize, Clone)]
pub struct TurnstileConfig {
    pub verify_url: String,
    pub secret: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct UpstreamConfig {
    pub submit_url: String,
    pub request_timeout_ms: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SponsorConfig {
    pub private_key_base64: String,
    pub gas_budget: u64,
    pub gas_price: u64,
}

#[derive(Debug, Deserialize)]
struct EnvConfig {
    server_bind: String,
    upstream_submit_url: String,
    upstream_request_timeout_ms: u64,
    sponsor_private_key_base64: String,
    sponsor_gas_budget: u64,
    sponsor_gas_price: u64,
    captcha_provider: String,
    captcha_turnstile_verify_url: Option<String>,
    captcha_turnstile_secret: Option<String>,
    forum_package_id: String,
}

pub fn load() -> Result<AppConfig, crate::error::RelayError> {
    let raw = envy::prefixed("RELAY_")
        .from_env::<EnvConfig>()
        .map_err(crate::error::RelayError::ConfigEnv)?;

    let captcha = match raw.captcha_provider.trim().to_ascii_lowercase().as_str() {
        "disabled" => CaptchaConfig::Disabled,
        "turnstile" => {
            let verify_url = raw
                .captcha_turnstile_verify_url
                .ok_or_else(|| crate::error::RelayError::ConfigInvalid(
                    "RELAY_CAPTCHA_TURNSTILE_VERIFY_URL is required when RELAY_CAPTCHA_PROVIDER=turnstile"
                        .to_string(),
                ))?;
            let secret = raw
                .captcha_turnstile_secret
                .ok_or_else(|| crate::error::RelayError::ConfigInvalid(
                    "RELAY_CAPTCHA_TURNSTILE_SECRET is required when RELAY_CAPTCHA_PROVIDER=turnstile"
                        .to_string(),
                ))?;
            CaptchaConfig::Turnstile(TurnstileConfig { verify_url, secret })
        }
        other => {
            return Err(crate::error::RelayError::ConfigInvalid(format!(
                "unsupported RELAY_CAPTCHA_PROVIDER value: {other}"
            )))
        }
    };

    Ok(AppConfig {
        server: ServerConfig {
            bind: raw.server_bind,
        },
        captcha,
        upstream: UpstreamConfig {
            submit_url: raw.upstream_submit_url,
            request_timeout_ms: raw.upstream_request_timeout_ms,
        },
        sponsor: SponsorConfig {
            private_key_base64: raw.sponsor_private_key_base64,
            gas_budget: raw.sponsor_gas_budget,
            gas_price: raw.sponsor_gas_price,
        },
        forum_package_id: raw.forum_package_id,
    })
}
