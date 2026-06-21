use crate::error::{RelayError, UpstreamFailure, UpstreamFailureKind};
use crate::types::{RelayEvent, SendResponse};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::str::FromStr;
use std::time::Duration;
use sui_rpc::field::{FieldMask, FieldMaskUtil};
use sui_rpc::proto::sui::rpc::v2::owner::OwnerKind;
use sui_rpc::proto::sui::rpc::v2::{
    BatchGetObjectsRequest, DynamicField, ExecuteTransactionRequest, ListDynamicFieldsRequest,
    ListOwnedObjectsRequest,
};
use sui_rpc::proto::sui::rpc::v2::{GetObjectRequest, get_object_result};
use sui_sdk_types::Address;
use sui_sdk_types::Digest;
use sui_sdk_types::Input;
use sui_sdk_types::Mutability;
use sui_sdk_types::ObjectReference;
use sui_sdk_types::SharedInput;
use sui_sdk_types::TypeTag;

#[derive(Serialize, Deserialize)]
struct FeedEntry<T> {
    id: Address,
    value: T,
}

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
                    Err(RelayError::UpstreamAllFailed(UpstreamFailure::new(
                        failure_kind,
                        format!(
                            "{url}: transaction executed but failed in effects status: {kind}: {details}"
                        ),
                    )))
                }
            }
            Ok(Err(err)) => {
                Err(RelayError::UpstreamAllFailed(UpstreamFailure::new(
                    UpstreamFailureKind::Rpc,
                    format!("{url}: {err}"),
                )))
            }
            Err(err) => {
                Err(RelayError::UpstreamAllFailed(UpstreamFailure::new(
                    UpstreamFailureKind::Timeout,
                    format!("{url}: {err}"),
                )))
            }
        }
    }

    pub async fn object_ref(&self, object_id: Address) -> Result<ObjectReference, RelayError> {
        let Some(ref object) = self.fetch_objects([object_id]).await?[0] else {
            return Err(RelayError::UpstreamAllFailed(UpstreamFailure::new(
                UpstreamFailureKind::ObjectRead,
                "object not found",
            )));
        };
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
        Ok(ObjectReference::new(object_id, version, digest))
    }

    pub async fn fetch_feed<T: serde::de::DeserializeOwned>(
        &self,
        feed_id: Address,
        start: u64,
        end: u64,
    ) -> Result<Vec<T>, RelayError> {
        self.fetch_objects(
            (start..end).map(|i| feed_id.derive_object_id(&TypeTag::U64, &i.to_le_bytes())),
        )
        .await?
        .into_iter()
        .flatten()
        .map(|obj| {
            obj.contents()
                .deserialize::<FeedEntry<T>>()
                .map(|entry| entry.value)
                .map_err(|e| RelayError::Internal(format!("bcs decode FeedEntry: {e}")))
        })
        .collect()
    }

    pub async fn fetch_feed_raw(
        &self,
        feed_id: Address,
        start: u64,
        end: u64,
    ) -> Result<Vec<Vec<u8>>, RelayError> {
        self.fetch_objects(
            (start..end).map(|i| feed_id.derive_object_id(&TypeTag::U64, &i.to_le_bytes())),
        )
        .await?
        .into_iter()
        .flatten()
        .map(|obj| {
            Ok(obj.contents().value
                .as_ref()
                .ok_or_else(|| RelayError::Internal("feed entry has no bcs contents".into()))?
                .to_vec())
        })
        .collect()
    }

    pub async fn list_dynamic_fields(
        &self,
        parent_id: Address,
    ) -> Result<Vec<(Vec<u8>, Option<Address>, Option<Vec<u8>>)>, RelayError> {
        let client = self.client.clone();
        let request = ListDynamicFieldsRequest::const_default()
            .with_parent(parent_id.to_string())
            .with_page_size(1000)
            .with_read_mask(FieldMask::from_str("parent,field_id,name.value,child_id,value.value"));
        let stream = client.list_dynamic_fields(request);
        let mut stream = Box::pin(stream);
        let mut results = Vec::new();
        while let Some(field_res) = stream.next().await {
            let field: DynamicField = field_res.map_err(|e| {
                RelayError::UpstreamAllFailed(UpstreamFailure::new(
                    UpstreamFailureKind::ObjectRead,
                    format!("list_dynamic_fields: {e}"),
                ))
            })?;
            let name_bytes = field
                .name
                .and_then(|n| n.value)
                .map(|b| b.to_vec())
                .unwrap_or_default();
            let child_id = field.child_id.and_then(|s| Address::from_str(&s).ok());
            let value_bytes = field.value.and_then(|bcs| bcs.value.map(|b| b.to_vec()));
            results.push((name_bytes, child_id, value_bytes));
        }
        Ok(results)
    }

    pub async fn resolve_inputs(
        &self,
        objects: &[(Address, bool)],
    ) -> Result<Vec<Input>, RelayError> {
        let ids: Vec<Address> = objects.iter().map(|(id, _)| *id).collect();
        let fetched = self.fetch_objects(&ids).await?;

        objects
            .iter()
            .zip(fetched.iter())
            .map(|((object_id, mutable), obj)| {
                let obj = obj.as_ref().ok_or_else(|| {
                    RelayError::UpstreamAllFailed(UpstreamFailure::new(
                        UpstreamFailureKind::ObjectRead,
                        "object not found",
                    ))
                })?;

                let owner = obj.owner.as_ref().ok_or_else(|| {
                    RelayError::UpstreamAllFailed(UpstreamFailure::new(
                        UpstreamFailureKind::ObjectRead,
                        "object owner missing",
                    ))
                })?;

                if owner.kind == Some(OwnerKind::Shared as i32) {
                    let initial_shared_version = owner.version.ok_or_else(|| {
                        RelayError::UpstreamAllFailed(UpstreamFailure::new(
                            UpstreamFailureKind::ObjectRead,
                            "initial shared version missing",
                        ))
                    })?;
                    let mutability = if *mutable {
                        Mutability::Mutable
                    } else {
                        Mutability::Immutable
                    };
                    Ok(Input::Shared(SharedInput::new(
                        *object_id,
                        initial_shared_version,
                        mutability,
                    )))
                } else {
                    Err(RelayError::UpstreamAllFailed(UpstreamFailure::new(
                        UpstreamFailureKind::ObjectRead,
                        "intent object is not shared",
                    )))
                }
            })
            .collect()
    }

    pub async fn fetch_objects(
        &self,
        object_ids: impl IntoIterator<Item = impl Borrow<Address>>,
    ) -> Result<Vec<Option<sui_rpc::proto::sui::rpc::v2::Object>>, RelayError> {
        let mut client = self.client.clone();
        let mut request = BatchGetObjectsRequest::default();
        request.read_mask = Some(FieldMask::from_str("owner,version,digest,contents"));
        request.requests = object_ids
            .into_iter()
            .map(|object_id| GetObjectRequest::new(object_id.borrow()))
            .collect();
        let response = tokio::time::timeout(
            Duration::from_millis(self.timeout_ms),
            client.ledger_client().batch_get_objects(request),
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

        response
            .into_inner()
            .objects
            .into_iter()
            .map(|object| {
                object
                    .result
                    .ok_or_else(|| {
                        RelayError::UpstreamAllFailed(UpstreamFailure::new(
                            UpstreamFailureKind::ObjectRead,
                            "object not found",
                        ))
                    })
                    .map(|object| match object {
                        get_object_result::Result::Object(object) => Some(object),
                        get_object_result::Result::Error(_) => None,
                        _ => unreachable!("unexpected get_object_result variant"),
                    })
            })
            .collect::<Result<Vec<_>, _>>()
    }

    pub async fn list_gas_objects(&self, sponsor: Address) -> Result<Vec<Address>, RelayError> {
        let client = self.client.clone();
        let request = ListOwnedObjectsRequest::default().with_owner(&sponsor);
        let mut stream = Box::pin(client.list_owned_objects(request));
        let mut gas_ids = Vec::new();
        while let Some(object_res) = stream.next().await {
            let object = object_res.map_err(|e| {
                RelayError::SponsorBuild(format!("failed to list sponsor objects: {e}"))
            })?;
            let Some(obj_type) = object.object_type.as_deref() else {
                continue;
            };
            if obj_type.contains("Coin") && obj_type.contains("sui::SUI") {
                let id = Address::from_str(object.object_id())
                    .map_err(|e| RelayError::SponsorBuild(format!("invalid coin id: {e}")))?;
                gas_ids.push(id);
            }
        }
        if gas_ids.is_empty() {
            return Err(RelayError::SponsorBuild(
                "sponsor has no SUI gas objects".to_string(),
            ));
        }
        eprintln!("relay sponsor gas pool loaded size={}", gas_ids.len());
        Ok(gas_ids)
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
