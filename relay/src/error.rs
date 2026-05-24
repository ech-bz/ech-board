use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use std::fmt::{Display, Formatter};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamFailureKind {
    ClientInit,
    Timeout,
    Rpc,
    ExecutionTransient,
    ExecutionDeterministic,
    ObjectRead,
    Internal,
}

impl UpstreamFailureKind {
    pub fn is_retryable(self) -> bool {
        match self {
            UpstreamFailureKind::ClientInit
            | UpstreamFailureKind::Timeout
            | UpstreamFailureKind::Rpc
            | UpstreamFailureKind::ExecutionTransient
            | UpstreamFailureKind::ObjectRead
            | UpstreamFailureKind::Internal => true,
            UpstreamFailureKind::ExecutionDeterministic => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpstreamFailure {
    pub kind: UpstreamFailureKind,
    pub details: String,
}

impl UpstreamFailure {
    pub fn new(kind: UpstreamFailureKind, details: impl Into<String>) -> Self {
        Self {
            kind,
            details: details.into(),
        }
    }
}

impl Display for UpstreamFailure {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {}", self.kind, self.details)
    }
}

#[derive(Debug, Error)]
pub enum RelayError {
    #[error("failed to read config from env: {0}")]
    ConfigEnv(envy::Error),
    #[error("invalid config: {0}")]
    ConfigInvalid(String),
    #[error("captcha rejected")]
    CaptchaRejected,
    #[error("captcha verification request failed: {0}")]
    CaptchaRequest(reqwest::Error),
    #[error("captcha response decode failed: {0}")]
    CaptchaDecode(reqwest::Error),
    #[error("all upstream nodes rejected transaction: {0}")]
    UpstreamAllFailed(UpstreamFailure),
    #[error("invalid sponsor config: {0}")]
    SponsorConfig(String),
    #[error("failed to build sponsored transaction: {0}")]
    SponsorBuild(String),
}

impl RelayError {
    pub fn is_retryable_upstream(&self) -> bool {
        match self {
            RelayError::UpstreamAllFailed(failure) => failure.kind.is_retryable(),
            _ => false,
        }
    }
}

impl ResponseError for RelayError {
    fn status_code(&self) -> StatusCode {
        match self {
            RelayError::CaptchaRejected => StatusCode::UNAUTHORIZED,
            RelayError::UpstreamAllFailed(_) => StatusCode::BAD_GATEWAY,
            RelayError::CaptchaRequest(_)
            | RelayError::CaptchaDecode(_) => StatusCode::BAD_GATEWAY,
            RelayError::SponsorBuild(_) => StatusCode::BAD_REQUEST,
            RelayError::ConfigEnv(_) | RelayError::ConfigInvalid(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            RelayError::SponsorConfig(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .json(serde_json::json!({"error": self.to_string()}))
    }
}
