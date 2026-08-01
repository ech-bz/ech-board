use crate::app_state::AppState;
use crate::error;
use crate::handlers::{Bans, BoardObject, Registry, load_board, load_forum, load_thread};
use crate::types::{ContentKind, Intent, MAX_TEXT_SIZE, PostPart, Request};
use async_trait::async_trait;
use aws_sdk_kms::primitives::Blob;
use blake2::Digest;
use blake2::digest::consts::U32;
use rand::seq::SliceRandom;
use serde::Deserialize;
use std::str::FromStr;
use std::time::UNIX_EPOCH;
use sui_sdk_types::{
    Address, Identifier, Input, MoveCall, ProgrammableTransaction, Transaction, TransactionKind,
    TypeTag,
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
    #[allow(dead_code)]
    vote_keys: Vec<Address>,
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
    #[allow(dead_code)]
    thread: Address,
    text_hash: Option<Address>,
    media_hashes: Vec<Address>,
    #[allow(dead_code)]
    vote_keys: Vec<Address>,
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
            intent.objects[3].id,
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
    load_board(&state.upstream, board_id)
        .await
        .map_err(|e| error::RelayError::SponsorBuild(format!("load board: {e}")))
}

pub(crate) async fn handle_send(
    state: &AppState,
    intent: Intent,
    signature_bytes: Vec<u8>,
    remote_ip: &str,
    text: Option<MultipartBytes>,
    description: Option<String>,
    topic: Option<String>,
    media_files: Vec<TempFile>,
) -> Result<Vec<u8>, error::RelayError> {
    let (event_tag, event_payload) = split_event(&intent.payload)?;
    validate_target(&intent, event_tag)?;
    let payload_err = |e| error::RelayError::SponsorBuild(format!("failed to decode payload: {e}"));
    let payload: Option<Box<dyn IntentPayload>> = match (intent.function.as_str(), event_tag) {
        ("forum_apply_intent_uid", "new_board") => Some(Box::new(
            bcs::from_bytes::<NewBoardPayload>(event_payload).map_err(payload_err)?,
        )),
        ("board_apply_intent_uid", "new_thread") => Some(Box::new(
            bcs::from_bytes::<NewThreadPayload>(event_payload).map_err(payload_err)?,
        )),
        ("board_apply_thread_intent_uid", "new_post") => Some(Box::new(
            bcs::from_bytes::<NewPostPayload>(event_payload).map_err(payload_err)?,
        )),
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

    let responses = resolve_requests(state, &intent, remote_ip).await?;
    let sealed = seal_responses(&state.sponsor, &signature_bytes, &responses)?;

    let mut attempt = 0u64;
    let result = loop {
        attempt += 1;
        match state
            .upstream
            .broadcast_signed(&state.sponsor.sign_as_sender(
                build_transaction(state, &intent, &signature_bytes, sealed.clone()).await?,
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

async fn resolve_requests(
    state: &AppState,
    intent: &Intent,
    remote_ip: &str,
) -> Result<Vec<u8>, error::RelayError> {
    let mut results = Vec::new();
    for req in &intent.requests {
        match req {
            Request::Uid => {
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
            Request::Ip32(domain) => {
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
        }
    }
    Ok(results)
}

async fn check_bans_inner(
    state: &AppState,
    intent: &Intent,
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
        .is_some_and(|board| board.projection.ignore_forum_bans)
    {
        levels.push(&forum.projection.bans);
    }
    if let Some(board) = &board {
        levels.push(&board.projection.bans);
    }
    if let Some(thread) = &thread {
        levels.push(&thread.projection.bans);
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
        "board_apply_intent_uid" => Ok((2, Some(3), None)),
        "board_apply_thread_intent_uid" => Ok((2, Some(3), Some(4))),
        "thread_apply_intent_uid" => Ok((1, Some(2), Some(3))),
        _ => Err(error::RelayError::SponsorBuild(
            "unsupported intent hierarchy".into(),
        )),
    }
}

fn seal_responses(
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

fn validate_target(intent: &Intent, event_tag: &str) -> Result<(), error::RelayError> {
    if intent.module != "main" {
        return Err(error::RelayError::SponsorBuild(
            "intent module must be main".into(),
        ));
    }
    let allowed: &[&str] = match intent.function.as_str() {
        "forum_apply_intent_uid" => &[
            "add_moderator",
            "del_moderator",
            "new_board",
            "set_timestamp_precision",
        ],
        "forum_apply_post_intent_uid" => &["ban", "unban"],
        "board_apply_intent_uid" => &[
            "add_moderator",
            "del_moderator",
            "set_max_media",
            "set_bump_limit",
            "set_closed",
            "set_deleted",
            "new_thread",
            "set_description",
            "set_ignore_forum_bans",
            "set_reactions",
        ],
        "board_apply_thread_intent_uid" => &["new_post"],
        "board_apply_post_intent_uid" => &["ban", "unban"],
        "thread_apply_intent_uid" => &[
            "add_moderator",
            "del_moderator",
            "set_closed",
            "set_deleted",
            "set_pinned",
            "set_topic",
            "set_admin",
        ],
        "thread_apply_post_intent_uid" => &["ban", "unban"],
        "post_apply_intent_uid" => &["set_deleted", "set_text", "remove_media"],
        "post_apply_intent_uid_ip32" => &["set_reaction", "vote"],
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

async fn build_transaction(
    state: &AppState,
    intent: &Intent,
    signature_bytes: &[u8],
    sealed_responses: Vec<u8>,
) -> Result<Transaction, error::RelayError> {
    let intent_bytes = bcs::to_bytes(intent)
        .map_err(|e| error::RelayError::SponsorBuild(format!("failed to encode intent: {e}")))?;
    let mut inputs = vec![
        Input::Pure(bcs::to_bytes(&intent_bytes).map_err(|e| {
            error::RelayError::SponsorBuild(format!("failed to encode intent bytes: {e}"))
        })?),
        Input::Pure(bcs::to_bytes(&signature_bytes.to_vec()).map_err(|e| {
            error::RelayError::SponsorBuild(format!("failed to encode signature: {e}"))
        })?),
        Input::Pure(sealed_responses),
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
