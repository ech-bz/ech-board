use crate::error::{RelayError, UpstreamFailure, UpstreamFailureKind};
use crate::types::{RelayEvent, SendResponse};
use std::str::FromStr;
use std::time::Duration;
use sui_rpc::field::{FieldMask, FieldMaskUtil};
use sui_rpc::proto::sui::rpc::v2::owner::OwnerKind;
use sui_rpc::proto::sui::rpc::v2::ExecuteTransactionRequest;
use sui_rpc::proto::sui::rpc::v2::GetObjectRequest;
use sui_sdk_types::Address;
use sui_sdk_types::Digest;
use sui_sdk_types::ObjectReference;
use sui_sdk_types::SharedInput;

#[derive(Clone)]
pub struct UpstreamSender {
    client: sui_rpc::Client,
    url: String,
    timeout_ms: u64,
}

impl UpstreamSender {
    pub fn new(client: sui_rpc::Client, url: String, timeout_ms: u64) -> Self {
        Self {
            client,
            url,
            timeout_ms,
        }
    }

    pub async fn broadcast_signed(
        &self,
        signed: &sui_sdk_types::SignedTransaction,
    ) -> Result<SendResponse, RelayError> {
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
        request.read_mask = Some(FieldMask::from_str("effects.status,events"));
        let mut exec = client.execution_client();

        let result = tokio::time::timeout(
            Duration::from_millis(self.timeout_ms),
            exec.execute_transaction(request),
        )
        .await;

        match result {
            Ok(Ok(response)) => {
                let response = response.into_inner();
                let status = response
                    .transaction
                    .as_ref()
                    .and_then(|tx| tx.effects.as_ref())
                    .and_then(|effects| effects.status.clone());
                let success = status.as_ref().and_then(|s| s.success).unwrap_or(false);

                if success {
                    let events = response
                        .transaction
                        .as_ref()
                        .and_then(|tx| tx.events.as_ref())
                        .map(|events| {
                            events
                                .events()
                                .iter()
                                .map(|event| RelayEvent {
                                    package_id: event
                                        .package_id_opt()
                                        .unwrap_or_default()
                                        .to_string(),
                                    module: event.module_opt().unwrap_or_default().to_string(),
                                    sender: event.sender_opt().unwrap_or_default().to_string(),
                                    event_type: event
                                        .event_type_opt()
                                        .unwrap_or_default()
                                        .to_string(),
                                    contents: event
                                        .contents()
                                        .value_opt()
                                        .map(|bytes| bytes.to_vec())
                                        .unwrap_or_default(),
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    eprintln!("relay upstream ok url={}", url);
                    Ok(SendResponse {
                        accepted_by: vec![url],
                        digest,
                        events,
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
                            (
                                "UNKNOWN".to_string(),
                                "no execution error details".to_string(),
                            )
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

    pub async fn object_ref(&self, object_id: &Address) -> Result<ObjectReference, RelayError> {
        let object = self.fetch_object(object_id).await?;
        let version = object.version.ok_or_else(|| {
            RelayError::UpstreamAllFailed(UpstreamFailure::new(
                UpstreamFailureKind::ObjectRead,
                "object version missing",
            ))
        })?;
        let digest = object
            .digest
            .as_ref()
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
        Ok(ObjectReference::new(*object_id, version, digest))
    }

    pub async fn shared_input(
        &self,
        object_id: &Address,
        mutable: bool,
    ) -> Result<SharedInput, RelayError> {
        let object = self.fetch_object(object_id).await?;

        let owner = object.owner.ok_or_else(|| {
            RelayError::UpstreamAllFailed(UpstreamFailure::new(
                UpstreamFailureKind::ObjectRead,
                "object owner missing",
            ))
        })?;

        if owner.kind != Some(OwnerKind::Shared as i32) {
            return Err(RelayError::UpstreamAllFailed(UpstreamFailure::new(
                UpstreamFailureKind::ObjectRead,
                "object is not shared",
            )));
        }

        let initial_shared_version = owner.version.ok_or_else(|| {
            RelayError::UpstreamAllFailed(UpstreamFailure::new(
                UpstreamFailureKind::ObjectRead,
                "initial shared version missing",
            ))
        })?;

        Ok(SharedInput::new(
            *object_id,
            initial_shared_version,
            mutable,
        ))
    }

    async fn fetch_object(
        &self,
        object_id: &Address,
    ) -> Result<sui_rpc::proto::sui::rpc::v2::Object, RelayError> {
        let mut client = self.client.clone();
        let mut request = GetObjectRequest::new(object_id);
        request.read_mask = Some(FieldMask::from_str("owner,version,digest"));
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

        response.into_inner().object.ok_or_else(|| {
            RelayError::UpstreamAllFailed(UpstreamFailure::new(
                UpstreamFailureKind::ObjectRead,
                "object not found",
            ))
        })
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
