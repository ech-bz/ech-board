use crate::error::{OperatorError, Result};
use ech_k8s::LeaderSettings;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug)]
pub(crate) struct Config {
    pub(crate) operator: OperatorSettings,
    pub(crate) leader: LeaderSettings,
}

impl Config {
    pub(crate) fn from_env() -> Result<Self> {
        let operator: OperatorSettings = envy::prefixed("OPERATOR_").from_env()?;
        let leader: LeaderSettings = envy::prefixed("OPERATOR_LEADER_").from_env()?;
        if leader.lease_grace_seconds == 0
            || leader.lease_grace_seconds >= leader.lease_duration_seconds
        {
            return Err(OperatorError::Config(
                "lease grace must be positive and shorter than lease duration".into(),
            ));
        }
        Ok(Self { operator, leader })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct OperatorSettings {
    pub(crate) health_port: u16,
    pub(crate) reconcile_interval_seconds: u64,
    pub(crate) error_backoff_seconds: u64,
    pub(crate) worker_image: String,
    pub(crate) relay_image: String,
    pub(crate) move_repo: String,
    pub(crate) move_git_ref: String,
    pub(crate) move_package_path: String,
}

impl OperatorSettings {
    pub(crate) fn reconcile_interval(&self) -> Duration {
        Duration::from_secs(self.reconcile_interval_seconds)
    }

    pub(crate) fn error_backoff(&self) -> Duration {
        Duration::from_secs(self.error_backoff_seconds)
    }
}
