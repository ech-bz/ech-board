use std::path::Path;

pub use ech_board_common::{
    RelayCaptchaConfig as CaptchaConfig, RelayConfig as AppConfig,
    RelaySponsorConfig as SponsorConfig, RelayTurnstileConfig as TurnstileConfig,
};

pub fn load(path: &Path) -> Result<AppConfig, crate::error::RelayError> {
    serde_json::from_str(&std::fs::read_to_string(path).map_err(|e| {
        crate::error::RelayError::ConfigInvalid(format!("failed to read config file: {e}"))
    })?)
    .map_err(|e| crate::error::RelayError::ConfigInvalid(format!("failed to parse config: {e}")))
}
