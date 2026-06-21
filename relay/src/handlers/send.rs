use crate::app_state::AppState;
use crate::error;
use crate::handlers::BoardObject;
use crate::types::{ContentKind, Intent, MAX_TEXT_SIZE, PostPart};
use async_trait::async_trait;
use blake2::Digest;
use blake2::digest::consts::U32;
use rand::seq::SliceRandom;
use serde::Deserialize;
use std::str::FromStr;
use sui_sdk_types::{
    Address, Identifier, Input, MoveCall, ProgrammableTransaction, Transaction, TransactionKind,
};

type Blake2b = blake2::Blake2b<U32>;

use actix_multipart::form::{bytes::Bytes as MultipartBytes, tempfile::TempFile};

#[async_trait]
trait IntentPayload: Send + Sync {
    async fn verify(
        &self,
        state: &AppState,
        text: &Option<MultipartBytes>,
        media_files: &[TempFile],
        intent: &Intent,
    ) -> Result<(), error::RelayError>;
    async fn cleanup(&self, state: &AppState);
}

#[derive(Deserialize)]
struct NewThreadPayload {
    text_hash: Option<Address>,
    media_hashes: Vec<Address>,
}

#[async_trait]
impl IntentPayload for NewThreadPayload {
    async fn verify(
        &self,
        state: &AppState,
        text: &Option<MultipartBytes>,
        media_files: &[TempFile],
        intent: &Intent,
    ) -> Result<(), error::RelayError> {
        verify_content(
            state,
            text,
            media_files,
            intent,
            &self.text_hash,
            &self.media_hashes,
        )
        .await
    }
    async fn cleanup(&self, state: &AppState) {
        cleanup_content(state, &self.text_hash, &self.media_hashes).await
    }
}

#[derive(Deserialize)]
struct NewPostPayload {
    #[allow(dead_code)]
    thread: u64,
    text_hash: Option<Address>,
    media_hashes: Vec<Address>,
}

#[async_trait]
impl IntentPayload for NewPostPayload {
    async fn verify(
        &self,
        state: &AppState,
        text: &Option<MultipartBytes>,
        media_files: &[TempFile],
        intent: &Intent,
    ) -> Result<(), error::RelayError> {
        verify_content(
            state,
            text,
            media_files,
            intent,
            &self.text_hash,
            &self.media_hashes,
        )
        .await
    }
    async fn cleanup(&self, state: &AppState) {
        cleanup_content(state, &self.text_hash, &self.media_hashes).await
    }
}

async fn verify_content(
    state: &AppState,
    text: &Option<MultipartBytes>,
    media_files: &[TempFile],
    intent: &Intent,
    text_hash: &Option<Address>,
    media_hashes: &[Address],
) -> Result<(), error::RelayError> {
    let board = fetch_board(state, intent.objects[1].id).await?;

    if media_hashes.len() > board.projection.max_media as usize {
        return Err(error::RelayError::SponsorBuild(format!(
            "media count {} exceeds board max_media {}",
            media_hashes.len(),
            board.projection.max_media
        )));
    }

    match (text_hash, text) {
        (Some(hash), Some(blob)) => {
            if blob.data.len() > MAX_TEXT_SIZE {
                return Err(error::RelayError::SponsorBuild(format!(
                    "text size {} exceeds max {}",
                    blob.data.len(),
                    MAX_TEXT_SIZE
                )));
            }
            let _parts: Vec<PostPart> = bcs::from_bytes(&blob.data).map_err(|e| {
                error::RelayError::SponsorBuild(format!("invalid PostPart bcs: {e}"))
            })?;
            verify_hash(hash, &blob.data)?;
            state
                .seaweed
                .put(ContentKind::Text, hash, &blob.data)
                .await?;
        }
        (None, None) => {}
        (Some(_), None) => {
            return Err(error::RelayError::SponsorBuild(
                "text_hash present but no text content provided".into(),
            ));
        }
        (None, Some(_)) => {
            return Err(error::RelayError::SponsorBuild(
                "text provided but intent text_hash is None".into(),
            ));
        }
    }

    if media_files.len() != media_hashes.len() {
        return Err(error::RelayError::SponsorBuild(format!(
            "media count mismatch: {} blobs vs {} hashes",
            media_files.len(),
            media_hashes.len()
        )));
    }

    for (hash, file) in media_hashes.iter().zip(media_files.iter()) {
        let data = tokio::fs::read(file.file.path())
            .await
            .map_err(|e| error::RelayError::SponsorBuild(format!("failed to read media: {e}")))?;
        if Blake2b::digest(&data).as_slice() != hash.as_bytes() {
            return Err(error::RelayError::SponsorBuild(
                "content hash mismatch".into(),
            ));
        }
        crate::thumbnail::validate(&data)?;
    }

    for (hash, file) in media_hashes.iter().zip(media_files.iter()) {
        let data = tokio::fs::read(file.file.path())
            .await
            .map_err(|e| error::RelayError::SponsorBuild(format!("failed to read media: {e}")))?;
        let thumb = crate::thumbnail::generate(&data, file.file.path())?;
        state.seaweed.put(ContentKind::Media, hash, &data).await?;
        state
            .seaweed
            .put(ContentKind::Thumbnail, hash, &thumb)
            .await?;
    }

    Ok(())
}

async fn cleanup_content(state: &AppState, text_hash: &Option<Address>, media_hashes: &[Address]) {
    if let Some(hash) = text_hash {
        let _ = state.seaweed.delete(ContentKind::Text, hash).await;
    }
    for hash in media_hashes {
        let _ = state.seaweed.delete(ContentKind::Media, hash).await;
        let _ = state.seaweed.delete(ContentKind::Thumbnail, hash).await;
    }
}

fn verify_hash(expected: &Address, blob: &[u8]) -> Result<(), error::RelayError> {
    if Blake2b::digest(blob).as_slice() != expected.as_bytes() {
        return Err(error::RelayError::SponsorBuild(
            "content hash mismatch".into(),
        ));
    }
    Ok(())
}

async fn fetch_board(
    state: &AppState,
    board_id: Address,
) -> Result<BoardObject, error::RelayError> {
    state
        .upstream
        .fetch_objects([board_id])
        .await?
        .into_iter()
        .flatten()
        .next()
        .ok_or_else(|| error::RelayError::SponsorBuild("board not found".to_string()))?
        .contents()
        .deserialize::<BoardObject>()
        .map_err(|e| error::RelayError::SponsorBuild(format!("bcs decode BoardObject: {e}")))
}

pub(crate) async fn handle_send(
    state: &AppState,
    intent_bytes: Vec<u8>,
    signature_bytes: Vec<u8>,
    text: Option<MultipartBytes>,
    media_files: Vec<TempFile>,
) -> Result<Vec<u8>, error::RelayError> {
    let intent: Intent = bcs::from_bytes(&intent_bytes)
        .map_err(|e| error::RelayError::SponsorBuild(format!("failed to decode intent: {e}")))?;

    let payload_err = |e| error::RelayError::SponsorBuild(format!("failed to decode payload: {e}"));
    let payload: Option<Box<dyn IntentPayload>> =
        match (intent.module.as_str(), intent.function.as_str()) {
            ("forum", "apply_board_intent") => match intent.payload.first() {
                Some(&6) => Some(Box::new(
                    bcs::from_bytes::<NewThreadPayload>(&intent.payload[1..])
                        .map_err(payload_err)?,
                )),
                _ => None,
            },
            ("forum", "apply_board_thread_intent") => match intent.payload.first() {
                Some(&7) => Some(Box::new(
                    bcs::from_bytes::<NewPostPayload>(&intent.payload[1..]).map_err(payload_err)?,
                )),
                _ => None,
            },
            _ => None,
        };

    if let Some(ref p) = payload {
        p.verify(state, &text, &media_files, &intent).await?;
    }

    let mut attempt = 0u64;
    let result = loop {
        attempt += 1;
        match state
            .upstream
            .broadcast_signed(&state.sponsor.sign_as_sender(
                build_transaction(state, &intent, &intent_bytes, &signature_bytes).await?,
            ))
            .await
        {
            Ok(result) => {
                eprintln!("relay send success attempts={attempt}");
                break Ok(result);
            }
            Err(err) => {
                let retryable = err.is_retryable_upstream();
                eprintln!("relay send retry attempt={attempt} retryable={retryable} error={err}");
                if !retryable {
                    break Err(err);
                }
            }
        }
    };

    if let Some(p) = payload
        && result.is_err()
    {
        p.cleanup(state).await;
    }

    bcs::to_bytes(&result?)
        .map_err(|e| error::RelayError::SponsorBuild(format!("bcs encode SendResponse: {e}")))
}

async fn build_transaction(
    state: &AppState,
    intent: &Intent,
    intent_raw: &[u8],
    signature_bytes: &[u8],
) -> Result<Transaction, error::RelayError> {
    let mut inputs = vec![
        Input::Pure(bcs::to_bytes(&intent_raw.to_vec()).map_err(|e| {
            error::RelayError::SponsorBuild(format!("failed to encode intent bytes: {e}"))
        })?),
        Input::Pure(bcs::to_bytes(&signature_bytes.to_vec()).map_err(|e| {
            error::RelayError::SponsorBuild(format!("failed to encode signature: {e}"))
        })?),
    ];
    let intent_objects: Vec<_> = intent.objects.iter().map(|o| (o.id, o.mutable)).collect();
    inputs.extend(state.upstream.resolve_inputs(&intent_objects).await?);

    let commands = vec![sui_sdk_types::Command::MoveCall(MoveCall {
        package: state.package_id,
        module: Identifier::from_str(intent.module.as_str()).map_err(|e| {
            error::RelayError::SponsorBuild(format!("failed to parse module name: {e}"))
        })?,
        function: Identifier::from_str(intent.function.as_str()).map_err(|e| {
            error::RelayError::SponsorBuild(format!("failed to parse function name: {e}"))
        })?,
        type_arguments: vec![],
        arguments: (0u16..inputs.len() as u16)
            .map(sui_sdk_types::Argument::Input)
            .collect(),
    })];

    Ok(Transaction {
        kind: TransactionKind::ProgrammableTransaction(ProgrammableTransaction {
            inputs,
            commands,
        }),
        sender: state.sponsor.sponsor_address(),
        gas_payment: sui_sdk_types::GasPayment {
            objects: vec![
                state
                    .upstream
                    .object_ref(
                        *state
                            .sponsor_gas_objects
                            .choose(&mut rand::thread_rng())
                            .expect("sponsor gas pool is guaranteed non-empty"),
                    )
                    .await?,
            ],
            owner: state.sponsor.sponsor_address(),
            price: state.sponsor_gas_price,
            budget: state.sponsor_gas_budget,
        },
        expiration: sui_sdk_types::TransactionExpiration::None,
    })
}
