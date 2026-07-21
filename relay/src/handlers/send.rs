use crate::app_state::AppState;
use crate::error;
use crate::handlers::BoardObject;
use crate::types::{ContentKind, Intent, MAX_TEXT_SIZE, PostPart};
use async_trait::async_trait;
use aws_sdk_kms::primitives::Blob;
use aws_sdk_kms::types::EncryptionAlgorithmSpec;
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
        description: Option<&str>,
        topic: Option<&str>,
        media_files: &[TempFile],
        intent: &Intent,
    ) -> Result<(), error::RelayError>;
    async fn cleanup(&self, state: &AppState);
}

#[derive(Deserialize)]
struct NewBoardPayload {
    #[allow(dead_code)]
    slug: Vec<u8>,
    description_hash: Option<Address>,
    #[allow(dead_code)]
    max_media: u64,
    #[allow(dead_code)]
    bump_limit: u64,
}

#[async_trait]
impl IntentPayload for NewBoardPayload {
    async fn verify(
        &self,
        state: &AppState,
        _text: &Option<MultipartBytes>,
        description: Option<&str>,
        _topic: Option<&str>,
        _media_files: &[TempFile],
        _intent: &Intent,
    ) -> Result<(), error::RelayError> {
        verify_plaintext(state, &self.description_hash, description).await
    }
    async fn cleanup(&self, state: &AppState) {
        if let Some(hash) = &self.description_hash {
            let _ = state.seaweed.delete(ContentKind::PlainText, hash).await;
        }
    }
}

#[derive(Deserialize)]
struct NewThreadPayload {
    topic_hash: Option<Address>,
    text_hash: Option<Address>,
    media_hashes: Vec<Address>,
}

#[async_trait]
impl IntentPayload for NewThreadPayload {
    async fn verify(
        &self,
        state: &AppState,
        text: &Option<MultipartBytes>,
        _description: Option<&str>,
        topic: Option<&str>,
        media_files: &[TempFile],
        intent: &Intent,
    ) -> Result<(), error::RelayError> {
        verify_content(
            state,
            text,
            media_files,
            intent.objects[3].id,
            &self.text_hash,
            &self.media_hashes,
        )
        .await?;
        if let Some(t) = topic {
            if t.len() > 50 {
                return Err(error::RelayError::SponsorBuild(
                    "topic exceeds 50 chars".into(),
                ));
            }
        }
        verify_plaintext(state, &self.topic_hash, topic).await
    }
    async fn cleanup(&self, state: &AppState) {
        cleanup_content(state, &self.text_hash, &self.media_hashes).await;
        if let Some(hash) = &self.topic_hash {
            let _ = state.seaweed.delete(ContentKind::PlainText, hash).await;
        }
    }
}

#[derive(Deserialize)]
struct NewPostPayload {
    text_hash: Option<Address>,
    media_hashes: Vec<Address>,
}

#[async_trait]
impl IntentPayload for NewPostPayload {
    async fn verify(
        &self,
        state: &AppState,
        text: &Option<MultipartBytes>,
        _description: Option<&str>,
        _topic: Option<&str>,
        media_files: &[TempFile],
        intent: &Intent,
    ) -> Result<(), error::RelayError> {
        verify_content(
            state,
            text,
            media_files,
            intent.objects[2].id,
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
    board_id: Address,
    text_hash: &Option<Address>,
    media_hashes: &[Address],
) -> Result<(), error::RelayError> {
    let board = fetch_board(state, board_id).await?;

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

async fn verify_plaintext(
    state: &AppState,
    hash: &Option<Address>,
    value: Option<&str>,
) -> Result<(), error::RelayError> {
    match (hash, value) {
        (Some(hash), Some(text)) => {
            let data = text.as_bytes();
            verify_hash(hash, data)?;
            state
                .seaweed
                .put(ContentKind::PlainText, hash, data)
                .await?;
            Ok(())
        }
        (None, None) => Ok(()),
        (Some(_), None) => Err(error::RelayError::SponsorBuild(
            "plaintext hash present but no content provided".into(),
        )),
        (None, Some(_)) => Err(error::RelayError::SponsorBuild(
            "plaintext provided but intent hash is None".into(),
        )),
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

pub(crate) async fn verify_uid(
    state: &AppState,
    uid: &[u8],
    remote_ip: &str,
) -> Result<(), error::RelayError> {
    if uid.is_empty() {
        return Err(error::RelayError::SponsorBuild("uid is empty".into()));
    }

    let decrypted = state
        .kms
        .decrypt()
        .key_id(&state.kms_moderator)
        .ciphertext_blob(Blob::new(uid.to_vec()))
        .encryption_algorithm(EncryptionAlgorithmSpec::RsaesOaepSha256)
        .send()
        .await
        .map_err(|e| error::RelayError::SponsorBuild(format!("kms decrypt uid: {e}")))?;

    let hmac_decrypted = decrypted
        .plaintext()
        .ok_or_else(|| error::RelayError::SponsorBuild("kms decrypt: no plaintext".into()))?;

    let mac_output = state
        .kms
        .generate_mac()
        .key_id(&state.kms_hmac)
        .message(Blob::new(remote_ip.as_bytes()))
        .mac_algorithm(aws_sdk_kms::types::MacAlgorithmSpec::HmacSha256)
        .send()
        .await
        .map_err(|e| error::RelayError::Internal(format!("kms generate_mac: {e}")))?;

    let expected_mac = mac_output
        .mac()
        .ok_or_else(|| error::RelayError::Internal("kms generate_mac: no mac".into()))?;

    if hmac_decrypted.as_ref() != expected_mac.as_ref() {
        return Err(error::RelayError::SponsorBuild(
            "uid verification failed".into(),
        ));
    }

    Ok(())
}

pub(crate) async fn handle_send(
    state: &AppState,
    intent: Intent,
    signature_bytes: Vec<u8>,
    text: Option<MultipartBytes>,
    description: Option<String>,
    topic: Option<String>,
    media_files: Vec<TempFile>,
) -> Result<Vec<u8>, error::RelayError> {
    let payload_err = |e| error::RelayError::SponsorBuild(format!("failed to decode payload: {e}"));
    let payload: Option<Box<dyn IntentPayload>> =
        match (intent.module.as_str(), intent.function.as_str()) {
            ("forum", "apply_forum_intent") => match intent.payload.first() {
                Some(&3) => Some(Box::new(
                    bcs::from_bytes::<NewBoardPayload>(&intent.payload[1..])
                        .map_err(payload_err)?,
                )),
                _ => None,
            },
            ("forum", "apply_board_intent") => match intent.payload.first() {
                Some(&7) => Some(Box::new(
                    bcs::from_bytes::<NewThreadPayload>(&intent.payload[1..])
                        .map_err(payload_err)?,
                )),
                _ => None,
            },
            ("forum", "apply_thread_board_intent") => match intent.payload.first() {
                Some(&6) => Some(Box::new(
                    bcs::from_bytes::<NewPostPayload>(&intent.payload[1..]).map_err(payload_err)?,
                )),
                _ => None,
            },
            _ => None,
        };

    if let Some(ref p) = payload {
        p.verify(
            state,
            &text,
            description.as_deref(),
            topic.as_deref(),
            &media_files,
            &intent,
        )
        .await?;
    }

    let mut attempt = 0u64;
    let result = loop {
        attempt += 1;
        match state
            .upstream
            .broadcast_signed(&state.sponsor.sign_as_sender(
                build_transaction(state, &intent, &signature_bytes).await?,
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
    signature_bytes: &[u8],
) -> Result<Transaction, error::RelayError> {
    let intent_bytes = bcs::to_bytes(intent).map_err(|e| {
        error::RelayError::SponsorBuild(format!("failed to encode intent: {e}"))
    })?;
    let mut inputs = vec![
        Input::Pure(bcs::to_bytes(&intent_bytes).map_err(|e| {
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
