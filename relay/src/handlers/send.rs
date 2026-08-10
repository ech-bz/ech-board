use crate::app_state::AppState;
use crate::error;
use crate::handlers::{Bans, BoardObject, Registry, load_board, load_forum, load_thread};
use crate::types::{ContentKind, IntentV2, MAX_TEXT_SIZE, PostPart, RequestV2};
use async_trait::async_trait;
use aws_sdk_kms::primitives::Blob;
use blake2::Digest;
use blake2::digest::consts::U32;
use futures::StreamExt;
use rand::seq::SliceRandom;
use serde::Deserialize;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::UNIX_EPOCH;
use sui_sdk_types::{
    Address, Identifier, Input, MoveCall, ProgrammableTransaction, Transaction, TransactionKind,
    TypeTag,
};

type Blake2b = blake2::Blake2b<U32>;

const MAX_TX_SIZE: usize = 131072;

use actix_multipart::form::{bytes::Bytes as MultipartBytes, tempfile::TempFile};

#[async_trait]
trait IntentPayload: Send + Sync {
    async fn verify(
        &self,
        state: &AppState,
        text: &Option<MultipartBytes>,
        description: Option<&str>,
        topic: Option<&str>,
        reason: Option<&str>,
        name: Option<&str>,
        media_files: &[TempFile],
        intent: &IntentV2,
    ) -> Result<(), error::RelayError>;
    async fn cleanup(&self, state: &AppState);
}

#[derive(Deserialize)]
struct NewBoardPayload {
    #[allow(dead_code)]
    slug: Vec<u8>,
    #[allow(dead_code)]
    max_media: u64,
    #[allow(dead_code)]
    bump_limit: u64,
    description_hash: Option<Address>,
}

#[async_trait]
impl IntentPayload for NewBoardPayload {
    async fn verify(
        &self,
        state: &AppState,
        _text: &Option<MultipartBytes>,
        description: Option<&str>,
        _topic: Option<&str>,
        _reason: Option<&str>,
        _name: Option<&str>,
        _media_files: &[TempFile],
        _intent: &IntentV2,
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
struct NewThreadV2Payload {
    topic_hash: Option<Address>,
    text_hash: Option<Address>,
    media_hashes: Vec<Address>,
    name_hash: Option<Address>,
    #[allow(dead_code)]
    vote_keys: Vec<Address>,
    #[allow(dead_code)]
    multi_vote: bool,
}

#[derive(Deserialize)]
struct NewPostV2Payload {
    #[allow(dead_code)]
    thread: Address,
    text_hash: Option<Address>,
    media_hashes: Vec<Address>,
    name_hash: Option<Address>,
    #[allow(dead_code)]
    vote_keys: Vec<Address>,
    #[allow(dead_code)]
    multi_vote: bool,
}

macro_rules! impl_thread_payload {
    ($t:ty) => {
        #[async_trait]
        impl IntentPayload for $t {
            async fn verify(
                &self,
                state: &AppState,
                text: &Option<MultipartBytes>,
                _description: Option<&str>,
                topic: Option<&str>,
                _reason: Option<&str>,
                name: Option<&str>,
                media_files: &[TempFile],
                intent: &IntentV2,
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
                    if t.len() > 150 {
                        return Err(error::RelayError::SponsorBuild(
                            "topic exceeds 150 chars".into(),
                        ));
                    }
                }
                verify_plaintext(state, &self.topic_hash, topic).await?;
                verify_plaintext(state, &self.name_hash, name).await
            }
            async fn cleanup(&self, state: &AppState) {
                cleanup_content(state, &self.text_hash, &self.media_hashes).await;
                if let Some(hash) = &self.topic_hash {
                    let _ = state.seaweed.delete(ContentKind::PlainText, hash).await;
                }
                if let Some(hash) = &self.name_hash {
                    let _ = state.seaweed.delete(ContentKind::PlainText, hash).await;
                }
            }
        }
    };
}

macro_rules! impl_post_payload {
    ($t:ty) => {
        #[async_trait]
        impl IntentPayload for $t {
            async fn verify(
                &self,
                state: &AppState,
                text: &Option<MultipartBytes>,
                _description: Option<&str>,
                _topic: Option<&str>,
                _reason: Option<&str>,
                name: Option<&str>,
                media_files: &[TempFile],
                intent: &IntentV2,
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
                verify_plaintext(state, &self.name_hash, name).await
            }
            async fn cleanup(&self, state: &AppState) {
                cleanup_content(state, &self.text_hash, &self.media_hashes).await;
                if let Some(hash) = &self.name_hash {
                    let _ = state.seaweed.delete(ContentKind::PlainText, hash).await;
                }
            }
        }
    };
}

impl_thread_payload!(NewThreadV2Payload);
impl_post_payload!(NewPostV2Payload);

#[derive(Deserialize)]
struct SetTextPayload {
    text_hash: Option<Address>,
}

#[async_trait]
impl IntentPayload for SetTextPayload {
    async fn verify(
        &self,
        state: &AppState,
        text: &Option<MultipartBytes>,
        _description: Option<&str>,
        _topic: Option<&str>,
        _reason: Option<&str>,
        _name: Option<&str>,
        _media_files: &[TempFile],
        intent: &IntentV2,
    ) -> Result<(), error::RelayError> {
        verify_content(
            state,
            text,
            &[],
            intent.objects[3].id,
            &self.text_hash,
            &[],
        )
        .await
    }
    async fn cleanup(&self, state: &AppState) {
        if let Some(hash) = &self.text_hash {
            let _ = state.seaweed.delete(ContentKind::Text, hash).await;
        }
    }
}

#[derive(Deserialize)]
struct SetTopicPayload {
    topic_hash: Option<Address>,
}

#[async_trait]
impl IntentPayload for SetTopicPayload {
    async fn verify(
        &self,
        state: &AppState,
        _text: &Option<MultipartBytes>,
        _description: Option<&str>,
        topic: Option<&str>,
        _reason: Option<&str>,
        _name: Option<&str>,
        _media_files: &[TempFile],
        _intent: &IntentV2,
    ) -> Result<(), error::RelayError> {
        if let Some(t) = topic {
            if t.len() > 150 {
                return Err(error::RelayError::SponsorBuild(
                    "topic exceeds 150 chars".into(),
                ));
            }
        }
        verify_plaintext(state, &self.topic_hash, topic).await
    }
    async fn cleanup(&self, state: &AppState) {
        if let Some(hash) = &self.topic_hash {
            let _ = state.seaweed.delete(ContentKind::PlainText, hash).await;
        }
    }
}

#[derive(Deserialize)]
struct SetDescriptionPayload {
    description_hash: Option<Address>,
}

#[async_trait]
impl IntentPayload for SetDescriptionPayload {
    async fn verify(
        &self,
        state: &AppState,
        _text: &Option<MultipartBytes>,
        description: Option<&str>,
        _topic: Option<&str>,
        _reason: Option<&str>,
        _name: Option<&str>,
        _media_files: &[TempFile],
        _intent: &IntentV2,
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
struct BanPayload {
    #[allow(dead_code)]
    level: Address,
    #[allow(dead_code)]
    mask: u8,
    #[allow(dead_code)]
    ip_hash: Address,
    reason_hash: Address,
    #[allow(dead_code)]
    expires: u64,
}

#[async_trait]
impl IntentPayload for BanPayload {
    async fn verify(
        &self,
        state: &AppState,
        _text: &Option<MultipartBytes>,
        _description: Option<&str>,
        _topic: Option<&str>,
        reason: Option<&str>,
        _name: Option<&str>,
        _media_files: &[TempFile],
        _intent: &IntentV2,
    ) -> Result<(), error::RelayError> {
        if let Some(reason) = reason {
            verify_plaintext(state, &Some(self.reason_hash), Some(reason)).await?;
        }
        Ok(())
    }
    async fn cleanup(&self, state: &AppState) {
        let _ = state.seaweed.delete(ContentKind::PlainText, &self.reason_hash).await;
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

    if media_hashes.len() > board.projection.max_media() as usize {
        return Err(error::RelayError::SponsorBuild(format!(
            "media count {} exceeds board max_media {}",
            media_hashes.len(),
            board.projection.max_media()
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
        let meta = crate::thumbnail::compute_meta(&data, file.file.path())?;
        let meta_bcs = bcs::to_bytes(&meta)
            .map_err(|e| error::RelayError::Internal(format!("bcs encode media meta: {e}")))?;
        state.seaweed.put(ContentKind::Media, hash, &data).await?;
        state
            .seaweed
            .put(ContentKind::Thumbnail, hash, &thumb)
            .await?;
        state
            .seaweed
            .put(ContentKind::MediaMeta, hash, &meta_bcs)
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
    load_board(&state.upstream, board_id)
        .await
        .map_err(|e| error::RelayError::SponsorBuild(format!("load board: {e}")))
}

pub(crate) async fn handle_send(
    state: &AppState,
    intents: Vec<(IntentV2, Vec<u8>)>,
    remote_ip: &str,
    captcha: Option<&str>,
    text: Option<MultipartBytes>,
    description: Option<String>,
    topic: Option<String>,
    reason: Option<String>,
    name: Option<String>,
    tripcode: Option<String>,
    media_files: Vec<TempFile>,
) -> Result<Vec<u8>, error::RelayError> {
    let payload_err = |e| error::RelayError::SponsorBuild(format!("failed to decode payload: {e}"));
    let mut payloads: Vec<Box<dyn IntentPayload>> = Vec::new();
    for (intent, _signature_bytes) in &intents {
        let (event_tag, event_payload) = split_event(&intent.payload)?;
        validate_target(intent, event_tag)?;
        let payload: Option<Box<dyn IntentPayload>> = match (intent.function.as_str(), event_tag) {
            ("forum_apply_intent_uid", "new_board") => Some(Box::new(
                bcs::from_bytes::<NewBoardPayload>(event_payload).map_err(payload_err)?,
            )),
            ("board_apply_intent_uid_captcha", "new_thread_v2")
            | ("board_apply_intent_uid_tripcode_captcha", "new_thread_v2")
            | ("board_apply_intent_uid_geo_captcha", "new_thread_v2")
            | ("board_apply_intent_uid_geo_tripcode_captcha", "new_thread_v2") => Some(Box::new(
                bcs::from_bytes::<NewThreadV2Payload>(event_payload).map_err(payload_err)?,
            )),
            ("board_apply_thread_intent_uid_captcha", "new_post_v2")
            | ("board_apply_thread_intent_uid_tripcode_captcha", "new_post_v2")
            | ("board_apply_thread_intent_uid_geo_captcha", "new_post_v2")
            | ("board_apply_thread_intent_uid_geo_tripcode_captcha", "new_post_v2") => Some(Box::new(
                bcs::from_bytes::<NewPostV2Payload>(event_payload).map_err(payload_err)?,
            )),
            ("thread_apply_post_intent_uid", "post_set_text") => Some(Box::new(
                bcs::from_bytes::<SetTextPayload>(event_payload).map_err(payload_err)?,
            )),
            ("thread_apply_intent_uid", "set_topic") => Some(Box::new(
                bcs::from_bytes::<SetTopicPayload>(event_payload).map_err(payload_err)?,
            )),
            ("board_apply_intent_uid", "set_description") => Some(Box::new(
                bcs::from_bytes::<SetDescriptionPayload>(event_payload).map_err(payload_err)?,
            )),
            ("forum_apply_post_intent_uid", "ban")
            | ("board_apply_post_intent_uid", "ban")
            | ("thread_apply_post_intent_uid", "ban") => Some(Box::new(
                bcs::from_bytes::<BanPayload>(event_payload).map_err(payload_err)?,
            )),
            _ => None,
        };
        if let Some(p) = payload {
            p.verify(
                state,
                &text,
                description.as_deref(),
                topic.as_deref(),
                reason.as_deref(),
                name.as_deref(),
                &media_files,
                intent,
            )
            .await?;
            payloads.push(p);
        }
    }

    let resolved: Vec<Result<Vec<u8>, error::RelayError>> = futures::stream::iter(intents.iter())
        .map(|(intent, _)| resolve_requests(state, intent, remote_ip, tripcode.as_deref(), captcha))
        .buffered(32)
        .collect()
        .await;

    let mut sealed_items: Vec<(IntentV2, Vec<u8>, Vec<u8>)> = Vec::new();
    for ((intent, signature_bytes), responses) in intents.iter().zip(resolved) {
        let responses = responses?;
        let sealed = seal_responses(&state.sponsor, signature_bytes, &responses)?;
        sealed_items.push((intent.clone(), signature_bytes.clone(), sealed));
    }

    let mut attempt = 0u64;
    let result = loop {
        attempt += 1;
        let tx = build_transaction_many(state, &sealed_items).await?;
        let tx_size = bcs::to_bytes(&tx)
            .map_err(|e| error::RelayError::SponsorBuild(format!("failed to encode transaction: {e}")))?
            .len();
        if tx_size > MAX_TX_SIZE {
            return Err(error::RelayError::SponsorBuild(format!(
                "batch too large: {tx_size} bytes > {MAX_TX_SIZE}, reduce chunk size"
            )));
        }
        match state
            .upstream
            .broadcast_signed(&state.sponsor.sign_as_sender(tx))
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

    if result.is_err() {
        for p in &payloads {
            p.cleanup(state).await;
        }
    }

    bcs::to_bytes(&result?)
        .map_err(|e| error::RelayError::SponsorBuild(format!("bcs encode SendResponse: {e}")))
}

async fn resolve_requests(
    state: &AppState,
    intent: &IntentV2,
    remote_ip: &str,
    tripcode: Option<&str>,
    captcha: Option<&str>,
) -> Result<Vec<u8>, error::RelayError> {
    let mut results = Vec::new();
    for req in &intent.requests {
        match req {
            RequestV2::Uid => {
                let ip32 = remote_ip
                    .parse::<std::net::Ipv4Addr>()
                    .map_err(|e| error::RelayError::SponsorBuild(format!("invalid ipv4: {e}")))?
                    .to_bits();
                let masks: [(u8, u32); 4] = [
                    (32, 0xFFFFFFFF),
                    (24, 0xFFFFFF00),
                    (20, 0xFFFFF000),
                    (16, 0xFFFF0000),
                ];
                let mut hmacs: [[u8; 32]; 4] = [[0u8; 32]; 4];
                let mut uid_plaintext = Vec::new();
                for (idx, (mask_byte, mask)) in masks.iter().enumerate() {
                    let masked = ip32 & mask;
                    let mut msg = vec![*mask_byte];
                    msg.extend_from_slice(&masked.to_be_bytes());
                    let mac: [u8; 32] = state
                        .kms
                        .generate_mac()
                        .key_id(&state.kms_hmac)
                        .message(Blob::new(msg))
                        .mac_algorithm(aws_sdk_kms::types::MacAlgorithmSpec::HmacSha256)
                        .send()
                        .await
                        .map_err(|e| error::RelayError::SponsorBuild(format!("kms hmac: {e}")))?
                        .mac()
                        .ok_or_else(|| error::RelayError::SponsorBuild("kms hmac: no mac".into()))?
                        .as_ref()
                        .try_into()
                        .map_err(|_| error::RelayError::SponsorBuild("hmac not 32 bytes".into()))?;
                    hmacs[idx] = mac;
                    uid_plaintext.extend_from_slice(&bcs::to_bytes(&mac).map_err(|e| {
                        error::RelayError::SponsorBuild(format!("bcs encode hmac: {e}"))
                    })?);
                }

                check_bans_inner(state, intent, &hmacs).await?;

                let encrypted = state
                    .kms
                    .encrypt()
                    .key_id(&state.kms_moderator)
                    .plaintext(Blob::new(uid_plaintext))
                    .send()
                    .await
                    .map_err(|e| {
                        error::RelayError::SponsorBuild(format!("kms encrypt uid: {e}"))
                    })?;
                let ciphertext = encrypted.ciphertext_blob().ok_or_else(|| {
                    error::RelayError::SponsorBuild("kms encrypt: no ciphertext".into())
                })?;
                results.extend_from_slice(&bcs::to_bytes(ciphertext.as_ref()).map_err(|e| {
                    error::RelayError::SponsorBuild(format!("bcs encode uid: {e}"))
                })?);
            }
            RequestV2::Ip32(domain) => {
                let ip32 = remote_ip
                    .parse::<std::net::Ipv4Addr>()
                    .map_err(|e| error::RelayError::SponsorBuild(format!("invalid ipv4: {e}")))?
                    .to_bits();
                let mut message = domain.as_bytes().to_vec();
                message.extend_from_slice(&ip32.to_be_bytes());
                let mac: [u8; 32] = state
                    .kms
                    .generate_mac()
                    .key_id(&state.kms_hmac)
                    .message(Blob::new(message))
                    .mac_algorithm(aws_sdk_kms::types::MacAlgorithmSpec::HmacSha256)
                    .send()
                    .await
                    .map_err(|e| error::RelayError::SponsorBuild(format!("kms hmac: {e}")))?
                    .mac()
                    .ok_or_else(|| error::RelayError::SponsorBuild("kms hmac: no mac".into()))?
                    .as_ref()
                    .try_into()
                    .map_err(|_| error::RelayError::SponsorBuild("hmac not 32 bytes".into()))?;
                results.extend_from_slice(&mac);
            }
            RequestV2::Tripcode => {
                let raw = tripcode.ok_or_else(|| {
                    error::RelayError::SponsorBuild(
                        "tripcode requested but not provided".into(),
                    )
                })?;
                let (secured, trip) = if let Some(seed) = raw.strip_prefix("##") {
                    (
                        true,
                        crate::tripcode::secure_tripcode(seed, &state.secure_tripcode_key)?,
                    )
                } else if let Some(seed) = raw.strip_prefix('#') {
                    (false, crate::tripcode::tripcode(seed)?)
                } else {
                    return Err(error::RelayError::SponsorBuild(
                        "tripcode must start with # or ##".into(),
                    ));
                };
                let trip_bytes = trip.as_bytes().to_vec();
                results.push(secured as u8);
                results.extend_from_slice(
                    &bcs::to_bytes(&trip_bytes).map_err(|e| {
                        error::RelayError::SponsorBuild(format!("bcs encode tripcode: {e}"))
                    })?,
                );
            }
            RequestV2::Geo => {
                let ip = remote_ip
                    .parse::<std::net::Ipv4Addr>()
                    .map_err(|e| error::RelayError::SponsorBuild(format!("invalid ipv4: {e}")))?
                    .into();
                let code = state.geoip.country_code(ip).unwrap_or(0);
                results.extend_from_slice(&code.to_le_bytes());
            }
            RequestV2::Captcha => {
                let token = captcha.ok_or_else(|| {
                    error::RelayError::SponsorBuild("captcha requested but not provided".into())
                })?;
                state.captcha.verify(token, remote_ip).await?;
                results.push(1u8);
            }
        }
    }
    Ok(results)
}

async fn check_bans_inner(
    state: &AppState,
    intent: &IntentV2,
    hmacs: &[[u8; 32]; 4],
) -> Result<(), error::RelayError> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| error::RelayError::SponsorBuild(format!("system time: {e}")))?
        .as_millis() as u64;

    let (forum_index, board_index, thread_index) = hierarchy_indices(&intent.function)?;
    let forum = load_forum(&state.upstream, intent.objects[forum_index].id).await?;
    let board = match board_index {
        Some(index) => Some(load_board(&state.upstream, intent.objects[index].id).await?),
        None => None,
    };
    let thread = match thread_index {
        Some(index) => Some(load_thread(&state.upstream, intent.objects[index].id).await?),
        None => None,
    };
    let mut levels: Vec<&Bans> = Vec::with_capacity(3);
    if !board
        .as_ref()
        .is_some_and(|board| board.projection.ignore_forum_bans())
    {
        levels.push(&forum.projection.bans());
    }
    if let Some(board) = &board {
        levels.push(board.projection.bans());
    }
    if let Some(thread) = &thread {
        levels.push(thread.projection.bans());
    }

    for bans in levels {
        let registries: [(u8, &Registry); 4] = [
            (32, &bans.ip32),
            (24, &bans.ip24),
            (20, &bans.ip20),
            (16, &bans.ip16),
        ];

        for (mask_idx, (mask_byte, reg)) in registries.iter().enumerate() {
            let mut hash_input = reg.entries.id.as_bytes().to_vec();
            hash_input.push(*mask_byte);
            hash_input.extend_from_slice(&hmacs[mask_idx]);
            let hash_bytes = Blake2b::digest(&hash_input);
            let hash_le: Vec<u8> = hash_bytes.iter().rev().copied().collect();

            let fields = state.upstream.list_dynamic_fields(reg.index.id).await?;
            for (name, _child, value) in &fields {
                if name.as_slice() == hash_le.as_slice() {
                    if let Some(v) = value {
                        if let Ok(entry_id) = bcs::from_bytes::<u64>(v) {
                            let entry_obj_id = reg
                                .entries
                                .id
                                .derive_dynamic_child_id(&TypeTag::U64, &entry_id.to_le_bytes());
                            if let Some(entry) =
                                &state.upstream.fetch_objects([entry_obj_id]).await?[0]
                            {
                                let field = entry
                                    .contents()
                                    .deserialize::<BanEntryField>()
                                    .map_err(|e| {
                                        error::RelayError::SponsorBuild(format!(
                                            "ban entry decode: {e}"
                                        ))
                                    })?;
                                if field.value.expires > now_ms {
                                    return Err(error::RelayError::SponsorBuild("banned".into()));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

#[derive(Deserialize)]
struct BanEntryField {
    #[allow(dead_code)]
    id: Address,
    #[allow(dead_code)]
    name: u64,
    value: BanValue,
}

#[derive(Deserialize)]
struct BanValue {
    #[allow(dead_code)]
    reason_hash: Address,
    expires: u64,
}

fn hierarchy_indices(
    function: &str,
) -> Result<(usize, Option<usize>, Option<usize>), error::RelayError> {
    match function {
        "forum_apply_intent_uid" => Ok((2, None, None)),
        "forum_apply_post_intent_uid"
        | "board_apply_post_intent_uid"
        | "thread_apply_post_intent_uid"
        | "post_apply_intent_uid"
        | "post_apply_intent_uid_ip32" => Ok((2, Some(3), Some(4))),
        "board_apply_intent_uid"
        | "board_apply_intent_uid_tripcode"
        | "board_apply_intent_uid_geo"
        | "board_apply_intent_uid_geo_tripcode"
        | "board_apply_intent_uid_captcha"
        | "board_apply_intent_uid_tripcode_captcha"
        | "board_apply_intent_uid_geo_captcha"
        | "board_apply_intent_uid_geo_tripcode_captcha" => Ok((2, Some(3), None)),
        "board_apply_thread_intent_uid"
        | "board_apply_thread_intent_uid_tripcode"
        | "board_apply_thread_intent_uid_geo"
        | "board_apply_thread_intent_uid_geo_tripcode"
        | "board_apply_thread_intent_uid_captcha"
        | "board_apply_thread_intent_uid_tripcode_captcha"
        | "board_apply_thread_intent_uid_geo_captcha"
        | "board_apply_thread_intent_uid_geo_tripcode_captcha" => Ok((2, Some(3), Some(4))),
        "thread_apply_intent_uid" => Ok((1, Some(2), Some(3))),
        _ => Err(error::RelayError::SponsorBuild(
            "unsupported intent hierarchy".into(),
        )),
    }
}

pub(crate) fn seal_responses(
    sponsor: &crate::sponsor::SponsorService,
    intent_sig: &[u8],
    inner: &[u8],
) -> Result<Vec<u8>, error::RelayError> {
    let mut msg = intent_sig.to_vec();
    msg.extend_from_slice(inner);
    let relay_sig = sponsor.sign_blake2b(&msg);
    let relay_pk = sponsor.sponsor_public_key();

    let mut sealed = bcs::to_bytes(&relay_sig).map_err(|e| {
        error::RelayError::SponsorBuild(format!("failed to encode relay signature: {e}"))
    })?;
    sealed.extend_from_slice(relay_pk.as_bytes());
    sealed.extend_from_slice(inner);

    bcs::to_bytes(&sealed).map_err(|e| {
        error::RelayError::SponsorBuild(format!("failed to encode sealed responses: {e}"))
    })
}

fn split_event(event: &[u8]) -> Result<(&str, &[u8]), error::RelayError> {
    let (len, prefix_len) = read_uleb128(event)?;
    let end = prefix_len
        .checked_add(len)
        .filter(|end| *end <= event.len())
        .ok_or_else(|| error::RelayError::SponsorBuild("invalid event tag length".into()))?;
    let tag = std::str::from_utf8(&event[prefix_len..end])
        .map_err(|e| error::RelayError::SponsorBuild(format!("invalid event tag: {e}")))?;
    Ok((tag, &event[end..]))
}

fn read_uleb128(bytes: &[u8]) -> Result<(usize, usize), error::RelayError> {
    let mut value = 0usize;
    let mut shift = 0u32;
    for (index, byte) in bytes.iter().copied().enumerate() {
        let digit = (byte & 0x7f) as usize;
        value = value
            .checked_add(digit.checked_shl(shift).ok_or_else(|| {
                error::RelayError::SponsorBuild("event tag length overflow".into())
            })?)
            .ok_or_else(|| error::RelayError::SponsorBuild("event tag length overflow".into()))?;
        if byte & 0x80 == 0 {
            return Ok((value, index + 1));
        }
        shift += 7;
        if shift >= usize::BITS {
            break;
        }
    }
    Err(error::RelayError::SponsorBuild(
        "invalid event tag length".into(),
    ))
}

fn validate_target(intent: &IntentV2, event_tag: &str) -> Result<(), error::RelayError> {
    if intent.module != "main" {
        return Err(error::RelayError::SponsorBuild(
            "intent module must be main".into(),
        ));
    }
    let allowed: &[&str] = match intent.function.as_str() {
        "forum_apply_intent_uid" => &[
            "upgrade",
            "add_moderator",
            "del_moderator",
            "new_board",
            "set_timestamp_precision",
        ],
        "forum_apply_post_intent_uid" => &["ban", "unban"],
        "board_apply_intent_uid" => &[
            "upgrade",
            "add_moderator",
            "del_moderator",
            "set_max_media",
            "set_bump_limit",
            "set_closed",
            "set_deleted",
            "new_thread_migrate_v2",
            "set_description",
            "set_ignore_forum_bans",
            "set_reactions",
            "set_pinned",
        ],
        "board_apply_thread_intent_uid" => &["new_post_migrate_v2"],
        "board_apply_intent_uid_tripcode" => &["new_thread_migrate_v2"],
        "board_apply_intent_uid_geo" => &["new_thread_migrate_v2"],
        "board_apply_intent_uid_geo_tripcode" => &["new_thread_migrate_v2"],
        "board_apply_thread_intent_uid_tripcode" => &["new_post_migrate_v2"],
        "board_apply_thread_intent_uid_geo" => &["new_post_migrate_v2"],
        "board_apply_thread_intent_uid_geo_tripcode" => &["new_post_migrate_v2"],
        "board_apply_intent_uid_captcha" => &["new_thread_v2"],
        "board_apply_intent_uid_tripcode_captcha" => &["new_thread_v2"],
        "board_apply_intent_uid_geo_captcha" => &["new_thread_v2"],
        "board_apply_intent_uid_geo_tripcode_captcha" => &["new_thread_v2"],
        "board_apply_thread_intent_uid_captcha" => &["new_post_v2"],
        "board_apply_thread_intent_uid_tripcode_captcha" => &["new_post_v2"],
        "board_apply_thread_intent_uid_geo_captcha" => &["new_post_v2"],
        "board_apply_thread_intent_uid_geo_tripcode_captcha" => &["new_post_v2"],
        "board_apply_post_intent_uid" => &["ban", "unban"],
        "thread_apply_intent_uid" => &[
            "upgrade",
            "add_moderator",
            "del_moderator",
            "set_closed",
            "set_deleted",
            "set_topic",
            "set_admin",
        ],
        "thread_apply_post_intent_uid" => &["ban", "unban", "post_set_deleted", "post_set_text"],
        "post_apply_intent_uid" => &["upgrade", "ban_media", "unban_media", "set_deleted"],
        "post_apply_intent_uid_ip32" => &["set_reaction", "vote_v2"],
        _ => {
            return Err(error::RelayError::SponsorBuild(
                "unsupported intent function".into(),
            ));
        }
    };
    if !allowed.contains(&event_tag) {
        return Err(error::RelayError::SponsorBuild(
            "event is not allowed for intent function".into(),
        ));
    }
    Ok(())
}

pub(crate) async fn build_transaction(
    state: &AppState,
    intent: &IntentV2,
    signature_bytes: &[u8],
    sealed_responses: Vec<u8>,
) -> Result<Transaction, error::RelayError> {
    build_transaction_many(
        state,
        &[(intent.clone(), signature_bytes.to_vec(), sealed_responses)],
    )
    .await
}

async fn build_transaction_many(
    state: &AppState,
    items: &[(IntentV2, Vec<u8>, Vec<u8>)],
) -> Result<Transaction, error::RelayError> {
    let mut unique_objects: Vec<(Address, bool)> = Vec::new();
    let mut object_index: HashMap<(Address, bool), usize> = HashMap::new();
    for (intent, _, _) in items {
        for obj in &intent.objects {
            let key = (obj.id, obj.mutable);
            if !object_index.contains_key(&key) {
                object_index.insert(key, unique_objects.len());
                unique_objects.push(key);
            }
        }
    }
    let resolved = state.upstream.resolve_inputs(&unique_objects).await?;

    let mut inputs: Vec<Input> = Vec::new();
    let mut input_index: HashMap<(Address, bool), usize> = HashMap::new();
    for (key, inp) in unique_objects.iter().zip(resolved) {
        input_index.insert(*key, inputs.len());
        inputs.push(inp);
    }

    let mut commands = Vec::new();
    for (intent, signature_bytes, sealed) in items {
        let intent_bytes = bcs::to_bytes(intent)
            .map_err(|e| error::RelayError::SponsorBuild(format!("failed to encode intent: {e}")))?;
        let base = inputs.len();
        inputs.push(Input::Pure(bcs::to_bytes(&intent_bytes).map_err(|e| {
            error::RelayError::SponsorBuild(format!("failed to encode intent bytes: {e}"))
        })?));
        inputs.push(Input::Pure(bcs::to_bytes(&signature_bytes.clone()).map_err(|e| {
            error::RelayError::SponsorBuild(format!("failed to encode signature: {e}"))
        })?));
        inputs.push(Input::Pure(sealed.clone()));
        let mut arguments: Vec<sui_sdk_types::Argument> = vec![
            sui_sdk_types::Argument::Input(base as u16),
            sui_sdk_types::Argument::Input((base + 1) as u16),
            sui_sdk_types::Argument::Input((base + 2) as u16),
        ];
        for obj in &intent.objects {
            let idx = input_index[&(obj.id, obj.mutable)];
            arguments.push(sui_sdk_types::Argument::Input(idx as u16));
        }
        commands.push(sui_sdk_types::Command::MoveCall(MoveCall {
            package: state.package_id,
            module: Identifier::from_str(intent.module.as_str()).map_err(|e| {
                error::RelayError::SponsorBuild(format!("failed to parse module name: {e}"))
            })?,
            function: Identifier::from_str(intent.function.as_str()).map_err(|e| {
                error::RelayError::SponsorBuild(format!("failed to parse function name: {e}"))
            })?,
            type_arguments: vec![],
            arguments,
        }));
    }

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
            budget: state.sponsor_gas_budget.saturating_mul(items.len() as u64),
        },
        expiration: sui_sdk_types::TransactionExpiration::None,
    })
}
