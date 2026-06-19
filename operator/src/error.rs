use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum OperatorError {
    #[error("config: {0}")]
    Config(String),
    #[error("finalizer: {0}")]
    Finalizer(String),
    #[error("controller fatal: {0}")]
    ControllerFatal(String),
    #[error(transparent)]
    Kube(#[from] kube::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
    #[error(transparent)]
    LeaderElection(#[from] kube_lease_manager::LeaseManagerError),
    #[error(transparent)]
    Envy(#[from] envy::Error),
}

pub(crate) type Result<T> = std::result::Result<T, OperatorError>;

impl From<String> for OperatorError {
    fn from(err: String) -> Self {
        Self::Finalizer(err)
    }
}

impl From<ech_k8s::CrMetaError> for OperatorError {
    fn from(err: ech_k8s::CrMetaError) -> Self {
        Self::ControllerFatal(err.to_string())
    }
}

impl From<ech_k8s::ReconcilerMetaError> for OperatorError {
    fn from(err: ech_k8s::ReconcilerMetaError) -> Self {
        Self::ControllerFatal(err.to_string())
    }
}
