use crate::error::{RelayError, UpstreamFailure, UpstreamFailureKind};
use std::str::FromStr;
use std::time::Duration;
use sui_rpc::proto::sui::rpc::v2::ExecuteTransactionRequest;
use sui_rpc::proto::sui::rpc::v2::GetObjectRequest;
use sui_sdk_types::Address;
use sui_sdk_types::Digest;
use sui_sdk_types::ObjectReference;

#[derive(Clone)]
pub struct UpstreamSender {
    client: sui_rpc::Client,
    url: String,
    timeout_ms: u64,
}

#[derive(Clone, Debug)]
pub struct BroadcastResult {
    pub accepted_by: Vec<String>,
    pub digest: String,
}

impl UpstreamSender {
    pub fn new(
        client: sui_rpc::Client,
        url: String,
        timeout_ms: u64,
    ) -> Self {
        Self {
            client,
            url,
            timeout_ms,
        }
    }

    pub async fn broadcast_signed(
        &self,
        signed: &sui_sdk_types::SignedTransaction,
    ) -> Result<BroadcastResult, RelayError> {
        let digest = signed.transaction.digest().to_string();
        let url = self.url.clone();
        let mut client = sui_rpc::Client::new(url.as_str()).map_err(|e| {
            RelayError::UpstreamAllFailed(UpstreamFailure::new(
                UpstreamFailureKind::ClientInit,
                format!("{url}: failed to init rpc client: {e}"),
            ))
        })?;

        let mut request = ExecuteTransactionRequest::new(signed.transaction.clone().into());
        request.signatures = signed.signatures.iter().cloned().map(Into::into).collect();
        let mut exec = client.execution_client();

        let result = tokio::time::timeout(
            Duration::from_millis(self.timeout_ms),
            exec.execute_transaction(request),
        )
        .await;

        match result {
            Ok(Ok(response)) => {
                let status = response
                    .into_inner()
                    .transaction
                    .and_then(|tx| tx.effects)
                    .and_then(|effects| effects.status);
                let success = status.as_ref().and_then(|s| s.success).unwrap_or(false);

                if success {
                    eprintln!("relay upstream ok url={}", url);
                    Ok(BroadcastResult {
                        accepted_by: vec![url],
                        digest,
                    })
                } else {
                    let (kind, details) = status
                        .and_then(|s| s.error)
                        .map(|e| {
                            let kind = e.kind().as_str_name().to_string();
                            let description = e
                                .description
                                .unwrap_or_else(|| "no description".to_string());
                            (kind, description)
                        })
                        .unwrap_or_else(|| {
                            ("UNKNOWN".to_string(), "no execution error details".to_string())
                        });
                    let failure_kind = classify_execution_error_kind(kind.as_str());
                    eprintln!(
                        "relay upstream fail url={} kind={} details={}",
                        url, kind, details
                    );
                    Err(RelayError::UpstreamAllFailed(UpstreamFailure::new(
                        failure_kind,
                        format!(
                            "{url}: transaction executed but failed in effects status: {kind}: {details}"
                        ),
                    )))
                }
            }
            Ok(Err(err)) => {
                eprintln!("relay upstream rpc_err url={} error={}", url, err);
                Err(RelayError::UpstreamAllFailed(UpstreamFailure::new(
                    UpstreamFailureKind::Rpc,
                    format!("{url}: {err}"),
                )))
            }
            Err(err) => {
                eprintln!("relay upstream timeout url={} error={}", url, err);
                Err(RelayError::UpstreamAllFailed(UpstreamFailure::new(
                    UpstreamFailureKind::Timeout,
                    format!("{url}: {err}"),
                )))
            }
        }
    }

    pub async fn latest_object_ref(&self, object_id: &Address) -> Result<ObjectReference, RelayError> {
        let mut client = self.client.clone();
        let request = GetObjectRequest::new(object_id);
        let response = tokio::time::timeout(
            Duration::from_millis(self.timeout_ms),
            client.ledger_client().get_object(request),
        )
        .await
        .map_err(|e| {
            RelayError::UpstreamAllFailed(UpstreamFailure::new(
                UpstreamFailureKind::ObjectRead,
                format!("get_object timeout: {e}"),
            ))
        })?
        .map_err(|e| {
            RelayError::UpstreamAllFailed(UpstreamFailure::new(
                UpstreamFailureKind::ObjectRead,
                format!("get_object rpc error: {e}"),
            ))
        })?;

        let object = response
            .into_inner()
            .object
            .ok_or_else(|| {
                RelayError::UpstreamAllFailed(UpstreamFailure::new(
                    UpstreamFailureKind::ObjectRead,
                    "object not found",
                ))
            })?;
        let obj_id = object
            .object_id
            .ok_or_else(|| {
                RelayError::UpstreamAllFailed(UpstreamFailure::new(
                    UpstreamFailureKind::ObjectRead,
                    "object id missing",
                ))
            })
            .and_then(|s| {
                Address::from_str(s.as_str()).map_err(|e| {
                    RelayError::UpstreamAllFailed(UpstreamFailure::new(
                        UpstreamFailureKind::ObjectRead,
                        format!("invalid object id from rpc: {e}"),
                    ))
                })
            })?;
        let version = object
            .version
            .ok_or_else(|| {
                RelayError::UpstreamAllFailed(UpstreamFailure::new(
                    UpstreamFailureKind::ObjectRead,
                    "object version missing",
                ))
            })?;
        let digest = object
            .digest
            .ok_or_else(|| {
                RelayError::UpstreamAllFailed(UpstreamFailure::new(
                    UpstreamFailureKind::ObjectRead,
                    "object digest missing",
                ))
            })
            .and_then(|s| {
                Digest::from_str(s.as_str()).map_err(|e| {
                    RelayError::UpstreamAllFailed(UpstreamFailure::new(
                        UpstreamFailureKind::ObjectRead,
                        format!("invalid object digest from rpc: {e}"),
                    ))
                })
            })?;
        Ok(ObjectReference::new(obj_id, version, digest))
    }
}

fn classify_execution_error_kind(kind: &str) -> UpstreamFailureKind {
    match kind {
        "INPUT_OBJECT_DELETED"
        | "OBJECT_VERSION_UNAVAILABLE_FOR_CONSUMPTION"
        | "LOCK_CONFLICT"
        | "EXECUTION_CANCELED_DUE_TO_CONSENSUS_OBJECT_CONGESTION"
        | "EXECUTION_CANCELED_DUE_TO_RANDOMNESS_UNAVAILABLE" => {
            UpstreamFailureKind::ExecutionTransient
        }
        _ => UpstreamFailureKind::ExecutionDeterministic,
    }
}
