use crate::app_state::AppState;
use crate::error::RelayError;
use crate::handlers::nonce::NonceInfo;
use crate::handlers::send;
use crate::handlers::{self, load_board};
use crate::thumbnail;
use crate::types::{ContentKind, Intent, IntentObject, PostPart, Request};
use aws_sdk_kms::primitives::Blob;
use base64::Engine;
use blake2::Digest;
use blake2::digest::consts::U32;
use ed25519_dalek::Signer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use sui_sdk_types::{Address, TypeTag};

type Blake2b = blake2::Blake2b<U32>;

pub(crate) struct ImportOptions {
    pub(crate) board: String,
    pub(crate) dump: PathBuf,
    pub(crate) admin_key: String,
    pub(crate) file_base: String,
    pub(crate) file_key: String,
    pub(crate) state_path: PathBuf,
}

#[derive(Deserialize)]
struct Dump {
    boards: Vec<DumpBoard>,
}

#[derive(Deserialize)]
struct DumpBoard {
    slug: String,
    #[allow(dead_code)]
    #[serde(default)]
    max_media: u64,
    #[allow(dead_code)]
    #[serde(default)]
    bump_limit: u64,
    #[allow(dead_code)]
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    posts: Vec<DumpPost>,
}

#[derive(Deserialize)]
struct DumpPost {
    num: u64,
    parent: u64,
    timestamp_ms: u64,
    #[serde(default)]
    subject: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    comment: Option<String>,
    #[serde(default)]
    menu: Option<String>,
    #[serde(default)]
    answers: Vec<String>,
    #[serde(default)]
    trip: Option<String>,
    #[serde(default)]
    trip_plain: Option<String>,
    #[serde(default)]
    ip: Option<String>,
    #[serde(default)]
    ip_country_code: Option<String>,
    #[serde(default)]
    force_geo: bool,
    #[serde(default)]
    sticky: bool,
    #[serde(default)]
    closed: bool,
    #[serde(default)]
    deleted: bool,
    #[serde(default)]
    enable_multiple_votes: bool,
    #[serde(default)]
    files: Vec<DumpFile>,
    #[serde(default)]
    reactions: Vec<DumpReaction>,
    #[serde(default)]
    poll_votes: Vec<DumpPollVote>,
}

#[derive(Deserialize)]
struct DumpFile {
    md5: String,
}

#[derive(Deserialize)]
struct DumpReaction {
    icon: String,
    #[serde(default)]
    ip: Option<String>,
}

#[derive(Deserialize)]
struct DumpPollVote {
    option: u64,
    #[serde(default)]
    ip: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct State {
    boards: HashMap<String, BoardState>,
    mapping: HashMap<u64, Mapping>,
}

#[derive(Serialize, Deserialize)]
struct BoardState {
    board_id: Address,
    post_count: u64,
    done: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct Mapping {
    board: Address,
    thread: Address,
    post: Address,
    num: u64,
}

#[derive(Deserialize)]
struct TableEntry {
    #[allow(dead_code)]
    id: Address,
    #[allow(dead_code)]
    name: u64,
    value: Address,
}

pub(crate) async fn run(state: &AppState, opts: ImportOptions) -> Result<(), RelayError> {
    let admin = parse_admin_key(&opts.admin_key)?;
    let dump: Dump = serde_json::from_slice(&std::fs::read(&opts.dump).map_err(|e| {
        RelayError::Internal(format!("read dump: {e}"))
    })?)
    .map_err(|e| RelayError::Internal(format!("parse dump: {e}")))?;

    let mut state_file = load_state(&opts.state_path);
    let board = dump
        .boards
        .iter()
        .find(|b| b.slug == opts.board)
        .ok_or_else(|| RelayError::Internal(format!("board '{}' not found in dump", opts.board)))?;

    import_board(state, &opts, &admin, board, &mut state_file).await?;
    save_state(&opts.state_path, &state_file);
    Ok(())
}

fn parse_admin_key(input: &str) -> Result<ed25519_dalek::SigningKey, RelayError> {
    let seed = if input.len() == 64 && input.chars().all(|c| c.is_ascii_hexdigit()) {
        let bytes = hex::decode(input)
            .map_err(|e| RelayError::SponsorBuild(format!("admin key hex: {e}")))?;
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&bytes);
        seed
    } else {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(input.as_bytes())
            .map_err(|e| RelayError::SponsorBuild(format!("admin key base64: {e}")))?;
        if bytes.len() != 33 {
            return Err(RelayError::SponsorBuild(
                "admin key must be 33-byte base64 or 64-char hex".into(),
            ));
        }
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&bytes[1..]);
        seed
    };
    Ok(ed25519_dalek::SigningKey::from_bytes(&seed))
}

fn load_state(path: &Path) -> State {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn save_state(path: &Path, state: &State) {
    let bytes = serde_json::to_vec(state).expect("serialize state");
    let _ = std::fs::write(path, bytes);
}

fn shard_id(nonce_shards: &Address, sender: &Address) -> Address {
    let mut buf = vec![0u8];
    buf.extend_from_slice(sender.as_ref() as &[u8]);
    let addr = Address::new(Blake2b::digest(&buf).into());
    let hash = Blake2b::digest(&bcs::to_bytes(&addr).unwrap());
    let val = u64::from_be_bytes(hash[24..].try_into().unwrap());
    let index = val % 512;
    nonce_shards.derive_object_id(&TypeTag::U64, &index.to_le_bytes())
}

async fn table_value(
    state: &AppState,
    table_id: Address,
    key: u64,
) -> Result<Address, RelayError> {
    let child_id = table_id.derive_dynamic_child_id(&TypeTag::U64, &key.to_le_bytes());
    let object = state
        .upstream
        .fetch_objects([child_id])
        .await?
        .into_iter()
        .next()
        .flatten()
        .ok_or_else(|| RelayError::Internal(format!("table entry {key} not found")))?;
    let entry: TableEntry = object
        .contents()
        .deserialize()
        .map_err(|e| RelayError::Internal(format!("table entry decode: {e}")))?;
    Ok(entry.value)
}

async fn find_board(state: &AppState, slug: &str) -> Result<Option<Address>, RelayError> {
    let boards_table_id = state.forum.projection.boards.id;
    let fields = state.upstream.list_dynamic_fields(boards_table_id).await?;
    for (_name, _child, value) in &fields {
        let Some(value) = value else {
            continue;
        };
        let Ok(addr) = bcs::from_bytes::<Address>(value) else {
            continue;
        };
        let Ok(board) = load_board(&state.upstream, addr).await else {
            continue;
        };
        if board.projection.slug == slug {
            return Ok(Some(addr));
        }
    }
    Ok(None)
}

fn clock() -> Address {
    Address::from_hex("0x6").expect("clock id")
}

fn intent_for(
    admin_pk: &Address,
    function: &str,
    requests: Vec<Request>,
    objects: Vec<IntentObject>,
    payload: Vec<u8>,
    nonce: u64,
) -> Intent {
    Intent {
        module: "main".into(),
        function: function.into(),
        nonce,
        objects,
        requests,
        payload,
        public_key: *admin_pk,
        tweak: Address::ZERO,
    }
}

fn board_objects(state: &AppState, admin_pk: &Address, rest: Vec<Address>) -> Vec<IntentObject> {
    let mut objs = vec![IntentObject {
        id: clock(),
        mutable: false,
    }];
    let nonce_shard = shard_id(&state.forum.projection.nonce_shards, admin_pk);
    objs.push(IntentObject {
        id: nonce_shard,
        mutable: true,
    });
    objs.push(IntentObject {
        id: state.forum.id,
        mutable: true,
    });
    for id in rest {
        objs.push(IntentObject {
            id,
            mutable: true,
        });
    }
    objs
}

fn event(tag: &str) -> Vec<u8> {
    let mut payload = Vec::new();
    bcs::serialize_into(&mut payload, tag).expect("serialize tag");
    payload
}

fn push_bytes(payload: &mut Vec<u8>, value: &impl Serialize) {
    bcs::serialize_into(payload, value).expect("serialize field");
}

async fn next_nonce(state: &AppState, admin_pk: &Address) -> Result<u64, RelayError> {
    let bytes = handlers::nonce::fetch(state, admin_pk).await?;
    let info: NonceInfo =
        bcs::from_bytes(&bytes).map_err(|e| RelayError::Internal(format!("nonce decode: {e}")))?;
    Ok(info.nonce)
}

fn sign_intent(admin: &ed25519_dalek::SigningKey, intent: &Intent) -> Vec<u8> {
    let intent_bytes = bcs::to_bytes(intent).expect("intent bcs");
    let digest = Blake2b::digest(&intent_bytes);
    admin.sign(&digest).to_bytes().to_vec()
}

async fn broadcast(
    state: &AppState,
    intent: &Intent,
    signature: Vec<u8>,
    responses: &[u8],
) -> Result<(), RelayError> {
    let sealed = send::seal_responses(&state.sponsor, &signature, responses)?;
    let tx = send::build_transaction(state, intent, &signature, sealed).await?;
    let signed = state.sponsor.sign_as_sender(tx);
    let mut attempt = 0u64;
    loop {
        attempt += 1;
        match state.upstream.broadcast_signed(&signed).await {
            Ok(_) => return Ok(()),
            Err(err) => {
                eprintln!("import tx attempt={attempt} error={err}");
                if attempt >= 3 || !err.is_retryable_upstream() {
                    return Err(err);
                }
            }
        }
    }
}

fn blake2b(data: &[u8]) -> Address {
    Address::new(Blake2b::digest(data).into())
}

async fn uid_response(state: &AppState, old_ip: &str) -> Result<Vec<u8>, RelayError> {
    let ip: std::net::Ipv4Addr = old_ip.parse().unwrap_or(std::net::Ipv4Addr::new(0, 0, 0, 0));
    let ip32 = ip.to_bits();
    let masks: [(u8, u32); 4] = [
        (32, 0xFFFFFFFF),
        (24, 0xFFFFFF00),
        (20, 0xFFFFF000),
        (16, 0xFFFF0000),
    ];
    let mut uid_plaintext = Vec::new();
    for (mask_byte, mask) in masks {
        let masked = ip32 & mask;
        let mut msg = vec![mask_byte];
        msg.extend_from_slice(&masked.to_be_bytes());
        let mac: [u8; 32] = state
            .kms
            .generate_mac()
            .key_id(&state.kms_hmac)
            .message(Blob::new(msg))
            .mac_algorithm(aws_sdk_kms::types::MacAlgorithmSpec::HmacSha256)
            .send()
            .await
            .map_err(|e| RelayError::SponsorBuild(format!("kms hmac: {e}")))?
            .mac()
            .ok_or_else(|| RelayError::SponsorBuild("kms hmac: no mac".into()))?
            .as_ref()
            .try_into()
            .map_err(|_| RelayError::SponsorBuild("hmac not 32 bytes".into()))?;
        uid_plaintext.extend_from_slice(&bcs::to_bytes(&mac).map_err(|e| {
            RelayError::SponsorBuild(format!("bcs encode hmac: {e}"))
        })?);
    }
    let encrypted = state
        .kms
        .encrypt()
        .key_id(&state.kms_moderator)
        .plaintext(Blob::new(uid_plaintext))
        .send()
        .await
        .map_err(|e| RelayError::SponsorBuild(format!("kms encrypt uid: {e}")))?;
    let ciphertext = encrypted
        .ciphertext_blob()
        .ok_or_else(|| RelayError::SponsorBuild("kms encrypt: no ciphertext".into()))?;
    bcs::to_bytes(ciphertext.as_ref())
        .map_err(|e| RelayError::SponsorBuild(format!("bcs encode uid: {e}")))
}

async fn ip32_response(
    state: &AppState,
    domain: &Address,
    old_ip: &str,
) -> Result<Vec<u8>, RelayError> {
    let ip: std::net::Ipv4Addr = old_ip.parse().unwrap_or(std::net::Ipv4Addr::new(0, 0, 0, 0));
    let mut message = domain.as_bytes().to_vec();
    message.extend_from_slice(&ip.to_bits().to_be_bytes());
    let mac: [u8; 32] = state
        .kms
        .generate_mac()
        .key_id(&state.kms_hmac)
        .message(Blob::new(message))
        .mac_algorithm(aws_sdk_kms::types::MacAlgorithmSpec::HmacSha256)
        .send()
        .await
        .map_err(|e| RelayError::SponsorBuild(format!("kms hmac: {e}")))?
        .mac()
        .ok_or_else(|| RelayError::SponsorBuild("kms hmac: no mac".into()))?
        .as_ref()
        .try_into()
        .map_err(|_| RelayError::SponsorBuild("hmac not 32 bytes".into()))?;
    Ok(mac.to_vec())
}

fn trip_response(secured: bool, trip: &str) -> Result<Vec<u8>, RelayError> {
    let mut out = vec![secured as u8];
    out.extend_from_slice(
        &bcs::to_bytes(&trip.as_bytes().to_vec())
            .map_err(|e| RelayError::SponsorBuild(format!("bcs encode trip: {e}")))?,
    );
    Ok(out)
}

fn geo_response(country: &Option<String>) -> u32 {
    country
        .as_deref()
        .and_then(|code| iso3166::Country::from_alpha2(code).map(|c| c.id as u32))
        .unwrap_or(0)
}

fn old_ip(post: &DumpPost) -> &str {
    post.ip.as_deref().unwrap_or("")
}

async fn upload_plaintext(state: &AppState, value: &str) -> Result<Option<Address>, RelayError> {
    if value.is_empty() {
        return Ok(None);
    }
    let hash = blake2b(value.as_bytes());
    state
        .seaweed
        .put(ContentKind::PlainText, &hash, value.as_bytes())
        .await?;
    Ok(Some(hash))
}

async fn upload_text(state: &AppState, parts: &[PostPart]) -> Result<Option<Address>, RelayError> {
    if parts.is_empty() {
        return Ok(None);
    }
    let bytes = bcs::to_bytes(parts)
        .map_err(|e| RelayError::SponsorBuild(format!("postpart bcs: {e}")))?;
    if bytes.len() > crate::types::MAX_TEXT_SIZE {
        return Ok(None);
    }
    let hash = blake2b(&bytes);
    state.seaweed.put(ContentKind::Text, &hash, &bytes).await?;
    Ok(Some(hash))
}

async fn upload_media(
    state: &AppState,
    client: &reqwest::Client,
    opts: &ImportOptions,
    md5: &str,
) -> Result<Option<Address>, RelayError> {
    if md5.len() != 32 {
        return Ok(None);
    }
    let url = format!(
        "{}/{}/{}/{}/{}.orig",
        opts.file_base.trim_end_matches('/'),
        opts.file_key,
        &md5[0..2],
        &md5[2..4],
        md5
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| RelayError::Internal(format!("download media: {e}")))?;
    if !resp.status().is_success() {
        eprintln!("media {md5} download HTTP {}", resp.status());
        return Ok(None);
    }
    let data = resp
        .bytes()
        .await
        .map_err(|e| RelayError::Internal(format!("media body: {e}")))?
        .to_vec();
    if let Err(e) = thumbnail::validate(&data) {
        eprintln!("media {md5} rejected: {e}");
        return Ok(None);
    }
    let hash = blake2b(&data);
    let tmp = std::env::temp_dir().join(format!("ech-import-{md5}.bin"));
    if std::fs::write(&tmp, &data).is_ok() {
        if let Ok(thumb) = thumbnail::generate(&data, &tmp) {
            let _ = state.seaweed.put(ContentKind::Thumbnail, &hash, &thumb).await;
        }
        let _ = std::fs::remove_file(&tmp);
    }
    state.seaweed.put(ContentKind::Media, &hash, &data).await?;
    Ok(Some(hash))
}

async fn upload_files(
    state: &AppState,
    client: &reqwest::Client,
    opts: &ImportOptions,
    post: &DumpPost,
) -> Result<Vec<Address>, RelayError> {
    let mut hashes = Vec::new();
    for f in &post.files {
        if let Some(h) = upload_media(state, client, opts, &f.md5).await? {
            hashes.push(h);
        }
    }
    Ok(hashes)
}

async fn vote_keys(state: &AppState, post: &DumpPost) -> Result<Vec<Address>, RelayError> {
    let mut keys = Vec::new();
    for label in &post.answers {
        let hash = blake2b(label.as_bytes());
        state
            .seaweed
            .put(ContentKind::PlainText, &hash, label.as_bytes())
            .await?;
        keys.push(hash);
    }
    Ok(keys)
}

async fn migrate_requests(
    state: &AppState,
    post: &DumpPost,
) -> Result<(String, Vec<Request>, Vec<u8>), RelayError> {
    let uid = uid_response(state, old_ip(post)).await?;
    let geo = post.force_geo.then(|| geo_response(&post.ip_country_code));
    let trip = post
        .trip
        .as_ref()
        .or(post.trip_plain.as_ref())
        .map(|t| (post.trip.is_some(), t.as_str()));
    let (function, requests) = match (geo, trip) {
        (Some(_), Some(_)) => (
            "board_apply_thread_intent_uid_geo_tripcode",
            vec![Request::Uid, Request::Geo, Request::Tripcode],
        ),
        (Some(_), None) => (
            "board_apply_thread_intent_uid_geo",
            vec![Request::Uid, Request::Geo],
        ),
        (None, Some(_)) => (
            "board_apply_thread_intent_uid_tripcode",
            vec![Request::Uid, Request::Tripcode],
        ),
        (None, None) => ("board_apply_thread_intent_uid", vec![Request::Uid]),
    };
    let mut inner = uid;
    if let Some(code) = geo {
        inner.extend_from_slice(&code.to_le_bytes());
    }
    if let Some((secured, t)) = trip {
        inner.extend_from_slice(&trip_response(secured, t)?);
    }
    Ok((function.to_string(), requests, inner))
}

async fn import_board(
    state: &AppState,
    opts: &ImportOptions,
    admin: &ed25519_dalek::SigningKey,
    board: &DumpBoard,
    state_file: &mut State,
) -> Result<(), RelayError> {
    let admin_pk = Address::new(admin.verifying_key().to_bytes());
    let slug = board.slug.clone();
    state_file
        .boards
        .entry(slug.clone())
        .or_insert_with(|| BoardState {
            board_id: Address::ZERO,
            post_count: 0,
            done: false,
        });
    let mut board_id = state_file.boards[&slug].board_id;
    if board_id == Address::ZERO {
        let found = find_board(state, &slug).await?;
        board_id = found.ok_or_else(|| {
            RelayError::Internal(format!(
                "board /{} not found on chain — create it first",
                slug
            ))
        })?;
        state_file.boards.get_mut(&slug).unwrap().board_id = board_id;
    }
    let loaded = load_board(&state.upstream, board_id).await?;
    let posts_table = loaded.projection.posts.id;
    let threads_table = loaded.projection.threads.id;

    let client = reqwest::Client::new();
    let mut posts: Vec<&DumpPost> = board.posts.iter().collect();
    posts.sort_by_key(|p| p.num);

    let mut post_count = state_file.boards[&slug].post_count;
    for post in posts.into_iter() {
        if post.num <= post_count {
            continue;
        }
        let number = post_count + 1;
        if post.parent == 0 {
            migrate_thread(
                state, admin, &admin_pk, &client, opts, board_id, post, number,
            )
            .await?;
            let op_addr = table_value(state, posts_table, number).await?;
            let thread_addr = table_value(state, threads_table, number).await?;
            state_file.mapping.insert(
                post.num,
                Mapping {
                    board: board_id,
                    thread: thread_addr,
                    post: op_addr,
                    num: number,
                },
            );
        } else {
            let parent_mapping = state_file.mapping.get(&post.parent).ok_or_else(|| {
                RelayError::Internal(format!("parent {} not migrated", post.parent))
            })?;
            let thread_addr = parent_mapping.thread;
            migrate_post(
                state, admin, &admin_pk, &client, opts, board_id, post, thread_addr, number,
            )
            .await?;
            let post_addr = table_value(state, posts_table, number).await?;
            state_file.mapping.insert(
                post.num,
                Mapping {
                    board: board_id,
                    thread: thread_addr,
                    post: post_addr,
                    num: number,
                },
            );
        }
        post_count = number;
        state_file.boards.get_mut(&slug).unwrap().post_count = post_count;
        save_state(&opts.state_path, state_file);
        eprintln!("imported old#{} -> new #{}", post.num, number);
    }

    if !state_file.boards[&slug].done {
        replay(state, admin, &admin_pk, &client, opts, board_id, board, state_file).await?;
        state_file.boards.get_mut(&slug).unwrap().done = true;
        save_state(&opts.state_path, state_file);
    }
    Ok(())
}

async fn migrate_thread(
    state: &AppState,
    admin: &ed25519_dalek::SigningKey,
    admin_pk: &Address,
    client: &reqwest::Client,
    opts: &ImportOptions,
    board_id: Address,
    post: &DumpPost,
    number: u64,
) -> Result<(), RelayError> {
    let (function, requests, responses) = migrate_requests(state, post).await?;
    let media_hashes = upload_files(state, client, opts, post).await?;
    let text_hash = upload_text(state, &post_parts(post).await).await?;
    let topic_hash = upload_plaintext(state, post.subject.as_deref().unwrap_or("")).await?;
    let name_hash = upload_plaintext(state, post.name.as_deref().unwrap_or("")).await?;
    let keys = vote_keys(state, post).await?;

    let nonce = next_nonce(state, admin_pk).await?;
    let mut payload = event("new_thread_migrate_v2");
    push_bytes(&mut payload, &post.timestamp_ms);
    push_bytes(&mut payload, &topic_hash);
    push_bytes(&mut payload, &text_hash);
    push_bytes(&mut payload, &media_hashes);
    push_bytes(&mut payload, &name_hash);
    push_bytes(&mut payload, &keys);
    push_bytes(&mut payload, &post.enable_multiple_votes);

    let intent = intent_for(
        admin_pk,
        &function,
        requests,
        board_objects(state, admin_pk, vec![board_id]),
        payload,
        nonce,
    );
    let sig = sign_intent(admin, &intent);
    broadcast(state, &intent, sig, &responses).await?;
    let _ = number;
    Ok(())
}

async fn migrate_post(
    state: &AppState,
    admin: &ed25519_dalek::SigningKey,
    admin_pk: &Address,
    client: &reqwest::Client,
    opts: &ImportOptions,
    board_id: Address,
    post: &DumpPost,
    thread_addr: Address,
    number: u64,
) -> Result<(), RelayError> {
    let (function, requests, responses) = migrate_requests(state, post).await?;
    let media_hashes = upload_files(state, client, opts, post).await?;
    let text_hash = upload_text(state, &post_parts(post).await).await?;
    let name_hash = upload_plaintext(state, post.name.as_deref().unwrap_or("")).await?;
    let keys = vote_keys(state, post).await?;

    let nonce = next_nonce(state, admin_pk).await?;
    let mut payload = event("new_post_migrate_v2");
    push_bytes(&mut payload, &post.timestamp_ms);
    push_bytes(&mut payload, &thread_addr);
    push_bytes(&mut payload, &text_hash);
    push_bytes(&mut payload, &media_hashes);
    push_bytes(&mut payload, &name_hash);
    push_bytes(&mut payload, &keys);
    push_bytes(&mut payload, &post.enable_multiple_votes);

    let intent = intent_for(
        admin_pk,
        &function,
        requests,
        board_objects(state, admin_pk, vec![board_id, thread_addr]),
        payload,
        nonce,
    );
    let sig = sign_intent(admin, &intent);
    broadcast(state, &intent, sig, &responses).await?;
    let _ = number;
    Ok(())
}

async fn post_parts(post: &DumpPost) -> Vec<PostPart> {
    let mut text = post.comment.clone().unwrap_or_default();
    if let Some(menu) = &post.menu {
        if !menu.trim().is_empty() {
            text.push('\n');
            text.push_str(menu);
        }
    }
    bbcode_to_parts(&text)
}

fn bbcode_to_parts(text: &str) -> Vec<PostPart> {
    let mut out = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = text.chars().collect();
    while i < chars.len() {
        if chars[i] == '[' {
            if let Some(end) = find_close(&chars, i) {
                let tag: String = chars[i + 1..end].iter().collect();
                if let Some((variant, close)) = simple_tag(&tag) {
                    if let Some(close_idx) = find_str(&chars, close, end + 1) {
                        let inner: String = chars[end + 1..close_idx].iter().collect();
                        let children = bbcode_to_parts(&inner);
                        out.push(make_variant(variant, children));
                        i = close_idx + close.len();
                        continue;
                    }
                } else if let Some(url) = tag.strip_prefix("url=") {
                    if let Some(close_idx) = find_str(&chars, "[/url]", end + 1) {
                        let inner: String = chars[end + 1..close_idx].iter().collect();
                        let children = bbcode_to_parts(&inner);
                        out.push(PostPart::Link {
                            url: url.to_string(),
                            children,
                        });
                        i = close_idx + "[/url]".len();
                        continue;
                    }
                }
            }
        }
        out.push(PostPart::Plain(chars[i].to_string()));
        i += 1;
    }
    out
}

fn find_close(chars: &[char], start: usize) -> Option<usize> {
    (start + 1..chars.len()).find(|&j| chars[j] == ']')
}

fn find_str(chars: &[char], needle: &str, start: usize) -> Option<usize> {
    let needle: Vec<char> = needle.chars().collect();
    if needle.is_empty() {
        return None;
    }
    let mut j = start;
    while j + needle.len() <= chars.len() {
        if chars[j..j + needle.len()] == needle[..] {
            return Some(j);
        }
        j += 1;
    }
    None
}

fn simple_tag(tag: &str) -> Option<(&'static str, &'static str)> {
    match tag {
        "b" => Some(("bold", "[/b]")),
        "i" => Some(("italic", "[/i]")),
        "code" => Some(("code", "[/code]")),
        "u" => Some(("underline", "[/u]")),
        "o" => Some(("overline", "[/o]")),
        "spoiler" => Some(("spoiler", "[/spoiler]")),
        "s" => Some(("strike", "[/s]")),
        "sup" => Some(("sup", "[/sup]")),
        "sub" => Some(("sub", "[/sub]")),
        _ => None,
    }
}

fn make_variant(variant: &str, children: Vec<PostPart>) -> PostPart {
    match variant {
        "bold" => PostPart::Bold(children),
        "italic" => PostPart::Italic(children),
        "code" => PostPart::Code(children),
        "underline" => PostPart::Underline(children),
        "overline" => PostPart::Overline(children),
        "spoiler" => PostPart::Spoiler(children),
        "strike" => PostPart::Strike(children),
        "sup" => PostPart::Sup(children),
        "sub" => PostPart::Sub(children),
        _ => PostPart::Plain(String::new()),
    }
}

async fn replay(
    state: &AppState,
    admin: &ed25519_dalek::SigningKey,
    admin_pk: &Address,
    _client: &reqwest::Client,
    _opts: &ImportOptions,
    board_id: Address,
    board: &DumpBoard,
    state_file: &mut State,
) -> Result<(), RelayError> {
    let mut posts: Vec<&DumpPost> = board.posts.iter().collect();
    posts.sort_by_key(|p| p.num);

    let mut pinned: Vec<Address> = Vec::new();
    let mut reaction_icons: Vec<Address> = Vec::new();
    let mut closed_threads: Vec<Address> = Vec::new();
    let mut deleted_threads: Vec<Address> = Vec::new();

    struct ReactionEvent {
        post: Address,
        icon: Address,
        ip: String,
    }
    struct VoteEvent {
        post: Address,
        options: Vec<Address>,
        ip: String,
    }
    let mut reaction_events: Vec<ReactionEvent> = Vec::new();
    let mut vote_events: Vec<VoteEvent> = Vec::new();

    for post in posts {
        let Some(m) = state_file.mapping.get(&post.num) else {
            continue;
        };
        if m.board != board_id {
            continue;
        }
        if post.sticky && !pinned.contains(&m.thread) {
            pinned.push(m.thread);
        }
        if post.parent == 0 && post.closed && !closed_threads.contains(&m.thread) {
            closed_threads.push(m.thread);
        }
        if post.parent == 0 && post.deleted && !deleted_threads.contains(&m.thread) {
            deleted_threads.push(m.thread);
        }

        let mut seen_icon = HashMap::new();
        for r in &post.reactions {
            let icon_hash = blake2b(r.icon.as_bytes());
            if !reaction_icons.contains(&icon_hash) {
                reaction_icons.push(icon_hash);
            }
            seen_icon.insert(r.ip.clone().unwrap_or_default(), icon_hash);
        }
        for (ip, icon) in seen_icon {
            reaction_events.push(ReactionEvent { post: m.post, icon, ip });
        }

        let index_hash: HashMap<u64, Address> = post
            .answers
            .iter()
            .enumerate()
            .map(|(i, label)| (i as u64, blake2b(label.as_bytes())))
            .collect();
        let mut seen_vote: HashMap<String, Vec<Address>> = HashMap::new();
        for v in &post.poll_votes {
            if let Some(hash) = index_hash.get(&v.option) {
                let options = seen_vote.entry(v.ip.clone().unwrap_or_default()).or_default();
                if !options.contains(hash) {
                    options.push(*hash);
                }
            }
        }
        for (ip, options) in seen_vote {
            if !options.is_empty() {
                vote_events.push(VoteEvent { post: m.post, options, ip });
            }
        }
    }

    if !reaction_icons.is_empty() {
        board_event(state, admin, admin_pk, board_id, "set_reactions", |p| {
            push_bytes(p, &reaction_icons);
        })
        .await?;
    }
    if !pinned.is_empty() {
        board_event(state, admin, admin_pk, board_id, "set_pinned", |p| {
            push_bytes(p, &pinned);
        })
        .await?;
    }
    for t in closed_threads {
        thread_event(state, admin, admin_pk, board_id, t, "set_closed", |p| {
            push_bytes(p, &true);
        })
        .await?;
    }
    for t in deleted_threads {
        thread_event(state, admin, admin_pk, board_id, t, "set_deleted", |p| {
            push_bytes(p, &true);
        })
        .await?;
    }
    for ev in reaction_events {
        let uid = uid_response(state, &ev.ip).await?;
        let ip32 = ip32_response(state, &ev.post, &ev.ip).await?;
        let mut payload = event("set_reaction");
        push_bytes(&mut payload, &ev.icon);
        let mut inner = uid;
        inner.extend_from_slice(&ip32);
        post_ip32_event(state, admin, admin_pk, board_id, ev.post, payload, &inner).await?;
    }
    for ev in vote_events {
        let uid = uid_response(state, &ev.ip).await?;
        let ip32 = ip32_response(state, &ev.post, &ev.ip).await?;
        let mut payload = event("vote_v2");
        push_bytes(&mut payload, &ev.options);
        let mut inner = uid;
        inner.extend_from_slice(&ip32);
        post_ip32_event(state, admin, admin_pk, board_id, ev.post, payload, &inner).await?;
    }
    Ok(())
}

async fn board_event(
    state: &AppState,
    admin: &ed25519_dalek::SigningKey,
    admin_pk: &Address,
    board_id: Address,
    tag: &str,
    fields: impl FnOnce(&mut Vec<u8>),
) -> Result<(), RelayError> {
    let nonce = next_nonce(state, admin_pk).await?;
    let mut payload = event(tag);
    fields(&mut payload);
    let intent = intent_for(
        admin_pk,
        "board_apply_intent_uid",
        vec![Request::Uid],
        board_objects(state, admin_pk, vec![board_id]),
        payload,
        nonce,
    );
    let uid = uid_response(state, "").await?;
    let sig = sign_intent(admin, &intent);
    broadcast(state, &intent, sig, &uid).await?;
    Ok(())
}

async fn thread_event(
    state: &AppState,
    admin: &ed25519_dalek::SigningKey,
    admin_pk: &Address,
    board_id: Address,
    thread_id: Address,
    tag: &str,
    fields: impl FnOnce(&mut Vec<u8>),
) -> Result<(), RelayError> {
    let nonce = next_nonce(state, admin_pk).await?;
    let nonce_shard = shard_id(&state.forum.projection.nonce_shards, admin_pk);
    let mut payload = event(tag);
    fields(&mut payload);
    let intent = intent_for(
        admin_pk,
        "thread_apply_intent_uid",
        vec![Request::Uid],
        vec![
            IntentObject {
                id: nonce_shard,
                mutable: true,
            },
            IntentObject {
                id: state.forum.id,
                mutable: true,
            },
            IntentObject {
                id: board_id,
                mutable: true,
            },
            IntentObject {
                id: thread_id,
                mutable: true,
            },
        ],
        payload,
        nonce,
    );
    let uid = uid_response(state, "").await?;
    let sig = sign_intent(admin, &intent);
    broadcast(state, &intent, sig, &uid).await?;
    Ok(())
}

async fn post_ip32_event(
    state: &AppState,
    admin: &ed25519_dalek::SigningKey,
    admin_pk: &Address,
    board_id: Address,
    post_id: Address,
    payload: Vec<u8>,
    responses: &[u8],
) -> Result<(), RelayError> {
    let nonce = next_nonce(state, admin_pk).await?;
    let intent = intent_for(
        admin_pk,
        "post_apply_intent_uid_ip32",
        vec![Request::Uid, Request::Ip32(post_id)],
        board_objects(state, admin_pk, vec![board_id, post_id]),
        payload,
        nonce,
    );
    let sig = sign_intent(admin, &intent);
    broadcast(state, &intent, sig, responses).await?;
    Ok(())
}
