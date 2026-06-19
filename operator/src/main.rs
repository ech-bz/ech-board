mod config;
mod constants;
mod crds;
mod error;
mod reconcilers;
mod support;
mod workflow;

use crate::workflow::BoardWorkflow;
use ech_k8s::{OperatorSpec, RuntimeSettings};
use error::OperatorError;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), OperatorError> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .compact()
        .init();

    let config = config::Config::from_env()?;

    ech_k8s::Operator::run(OperatorSpec {
        runtime: RuntimeSettings {
            health_port: config.operator.health_port,
            reconcile_interval: config.operator.reconcile_interval(),
            error_backoff: config.operator.error_backoff(),
        },
        leader: config.leader,
        field_manager: "ech-board-operator",
        workflow: BoardWorkflow {
            operator: config.operator.clone(),
        },
    })
    .await
}
