use crate::app_state::AppState;
use crate::error::RelayError;
use crate::handlers::nonce::NonceInfo;
use crate::handlers::send;
use crate::handlers;
use crate::handlers::board::load_board;
use crate::thumbnail;
use crate::types::{ContentKind, IntentObject, IntentV2, PostPart, RequestV2};
use aws_sdk_kms::primitives::Blob;
use base64::Engine;
use blake2::Digest;
use blake2::digest::consts::U32;
use ed25519_dalek::Signer;
use rand::RngCore;
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
    #[serde(default)]
    op: bool,
    timestamp_ms: u64,
    #[serde(default)]
    subject: Option<String>,
    #[serde(default)]
    comment: Option<String>,
    #[serde(default)]
    menu: Option<String>,
    #[serde(default)]
    answers: Vec<String>,
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
    #[serde(default)]
    keys: HashMap<u64, [u8; 32]>,
}

#[derive(Serialize, Deserialize)]
struct BoardState {
    board_id: Address,
    post_count: u64,
    #[serde(default)]
    cursor: u64,
    done: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct Mapping {
    board: Address,
    thread: Address,
    post: Address,
    num: u64,
}

fn load_dump(path: &Path) -> Result<Dump, RelayError> {
    let raw = std::fs::read(path)
        .map_err(|e| RelayError::Internal(format!("read dump: {e}")))?;
    if path.extension().and_then(|e| e.to_str()) == Some("json") {
        return serde_json::from_slice(&raw)
            .map_err(|e| RelayError::Internal(format!("parse dump: {e}")));
    }
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(&raw[..]);
    let headers: Vec<String> = rdr
        .headers()
        .map_err(|e| RelayError::Internal(format!("csv headers: {e}")))?
        .iter()
        .map(|h| h.trim().to_string())
        .collect();
    let mut boards: Vec<DumpBoard> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for record in rdr.records() {
        let rec = record.map_err(|e| RelayError::Internal(format!("csv record: {e}")))?;
        let slug = csv_get(&headers, &rec, "board").unwrap_or("").trim().to_string();
        if slug.is_empty() {
            continue;
        }
        let bi = match index.get(&slug) {
            Some(&i) => i,
            None => {
                let i = boards.len();
                boards.push(DumpBoard {
                    slug: slug.clone(),
                    max_media: 0,
                    bump_limit: 0,
                    description: None,
                    posts: Vec::new(),
                });
                index.insert(slug.clone(), i);
                i
            }
        };
        let post = DumpPost {
            num: csv_num(csv_get(&headers, &rec, "num")),
            parent: csv_num(csv_get(&headers, &rec, "parent")),
            op: csv_bool(csv_get(&headers, &rec, "op")),
            timestamp_ms: csv_num(csv_get(&headers, &rec, "timestamp"))
                .saturating_mul(1000),
            subject: csv_opt(csv_get(&headers, &rec, "subject")),
            comment: csv_opt(csv_get(&headers, &rec, "comment")),
            menu: csv_opt(csv_get(&headers, &rec, "menu")),
            answers: csv_json_strings(csv_get(&headers, &rec, "answers")),
            trip_plain: csv_opt(csv_get(&headers, &rec, "trip_plain")),
            ip: csv_opt(csv_get(&headers, &rec, "ip")),
            ip_country_code: csv_opt(csv_get(&headers, &rec, "ip_country_code")),
            force_geo: csv_bool(csv_get(&headers, &rec, "force_geo")),
            sticky: csv_bool(csv_get(&headers, &rec, "sticky")),
            closed: csv_bool(csv_get(&headers, &rec, "closed")),
            deleted: csv_bool(csv_get(&headers, &rec, "deleted"))
                || csv_bool(csv_get(&headers, &rec, "deleted_by_thread_deletion"))
                || csv_bool(csv_get(&headers, &rec, "deleted_by_board_deletion"))
                || csv_bool(csv_get(&headers, &rec, "deleted_by_endless_excess"))
                || csv_bool(csv_get(&headers, &rec, "deleted_by_delall"))
                || csv_bool(csv_get(&headers, &rec, "deleted_by_owner"))
                || csv_bool(csv_get(&headers, &rec, "deleted_by_op"))
                || csv_bool(csv_get(&headers, &rec, "deleted_by_autodeletion")),
            enable_multiple_votes: csv_bool(csv_get(&headers, &rec, "enable_multiple_votes")),
            files: csv_json_files(csv_get(&headers, &rec, "files")),
            reactions: Vec::new(),
            poll_votes: Vec::new(),
        };
        boards[bi].posts.push(post);
    }
    Ok(Dump { boards })
}

fn csv_get<'a>(headers: &[String], rec: &'a csv::StringRecord, name: &str) -> Option<&'a str> {
    headers.iter().position(|h| h == name).and_then(|i| rec.get(i))
}

fn csv_num(v: Option<&str>) -> u64 {
    v.and_then(|s| s.trim().parse().ok()).unwrap_or(0)
}

fn csv_bool(v: Option<&str>) -> bool {
    matches!(
        v,
        Some("1") | Some("true") | Some("True") | Some("TRUE") | Some("yes") | Some("YES")
    )
}

fn csv_opt(v: Option<&str>) -> Option<String> {
    let s = v.unwrap_or("").trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn csv_json_strings(v: Option<&str>) -> Vec<String> {
    let s = v.unwrap_or("").trim();
    if s.is_empty() || s == "null" {
        return Vec::new();
    }
    serde_json::from_str::<Vec<String>>(s).unwrap_or_default()
}

fn csv_json_files(v: Option<&str>) -> Vec<DumpFile> {
    let s = v.unwrap_or("").trim();
    if s.is_empty() || s == "null" {
        return Vec::new();
    }
    let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(s) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|el| {
            let md5 = el
                .get("md5")
                .and_then(|m| m.as_str())
                .or_else(|| el.as_str());
            md5.map(|m| DumpFile { md5: m.to_string() })
        })
        .collect()
}

pub(crate) async fn run(state: &AppState, opts: ImportOptions) -> Result<(), RelayError> {
    let admin = parse_admin_key(&opts.admin_key)?;
    let dump: Dump = load_dump(&opts.dump)?;

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

async fn find_board(state: &AppState, slug: &str) -> Result<Option<Address>, RelayError> {
    let boards_table_id = state.forum.projection.boards().id;
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
        if board.projection.slug() == slug {
            return Ok(Some(addr));
        }
    }
    Ok(None)
}

fn clock() -> Address {
    Address::from_hex("0x6").expect("clock id")
}

fn random_seed() -> [u8; 32] {
    let mut seed = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut seed);
    seed
}

fn intent_for(
    sender: &Address,
    function: &str,
    requests: Vec<RequestV2>,
    objects: Vec<IntentObject>,
    payload: Vec<u8>,
    nonce: u64,
) -> IntentV2 {
    IntentV2 {
        module: "main".into(),
        function: function.into(),
        nonce,
        objects,
        requests,
        payload,
        public_key: *sender,
        tweak: Address::ZERO,
    }
}

fn board_objects(state: &AppState, sender: &Address, rest: Vec<Address>) -> Vec<IntentObject> {
    let mut objs = vec![IntentObject {
        id: clock(),
        mutable: false,
    }];
    let nonce_shard = shard_id(&state.forum.projection.nonce_shards(), sender);
    objs.push(IntentObject {
        id: nonce_shard,
        mutable: true,
    });
    objs.push(IntentObject {
        id: state.forum.root.id,
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

fn sign_intent(admin: &ed25519_dalek::SigningKey, intent: &IntentV2) -> Vec<u8> {
    let intent_bytes = bcs::to_bytes(intent).expect("intent bcs");
    let digest = Blake2b::digest(&intent_bytes);
    admin.sign(&digest).to_bytes().to_vec()
}

async fn broadcast(
    state: &AppState,
    intent: &IntentV2,
    signature: Vec<u8>,
    responses: &[u8],
) -> Result<Vec<Address>, RelayError> {
    let sealed = send::seal_responses(&state.sponsor, &signature, responses)?;
    let tx = send::build_transaction(state, intent, &signature, sealed).await?;
    let signed = state.sponsor.sign_as_sender(tx);
    let mut attempt = 0u64;
    loop {
        attempt += 1;
        match state.upstream.broadcast_signed(&signed).await {
            Ok(response) => return Ok(response.created),
            Err(err) => {
                eprintln!("import tx attempt={attempt} error={err}");
                if attempt >= 3 || !err.is_retryable_upstream() {
                    return Err(err);
                }
            }
        }
    }
}

async fn find_created(
    state: &AppState,
    created: &[Address],
) -> Result<(Option<Address>, Option<u64>, Option<Address>, Option<u64>), RelayError> {
    let mut thread = None;
    let mut post = None;
    for &id in created {
        if let Ok(t) = handlers::thread::load_thread(&state.upstream, id).await {
            thread = Some((id, t.projection.number()));
        }
        if let Ok(p) = handlers::post::load_post(&state.upstream, id).await {
            post = Some((id, p.projection.number()));
        }
    }
    Ok((
        thread.map(|x| x.0),
        thread.map(|x| x.1),
        post.map(|x| x.0),
        post.map(|x| x.1),
    ))
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
        if let Ok(meta) = thumbnail::compute_meta(&data, &tmp) {
            if let Ok(meta_bcs) = bcs::to_bytes(&meta) {
                let _ = state.seaweed.put(ContentKind::MediaMeta, &hash, &meta_bcs).await;
            }
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

fn parse_name_and_trip(input: &str) -> (Option<String>, Option<String>) {
    match input.find('#') {
        Some(idx) => {
            let name = input[..idx].trim();
            let trip = input[idx..].to_string();
            (
                if name.is_empty() {
                    None
                } else {
                    Some(name.to_string())
                },
                Some(trip),
            )
        }
        None => {
            let name = input.trim();
            (
                if name.is_empty() {
                    None
                } else {
                    Some(name.to_string())
                },
                None,
            )
        }
    }
}

fn compute_trip(raw: &str, key: &str) -> Result<Option<(bool, String)>, RelayError> {
    if let Some(seed) = raw.strip_prefix("##") {
        Ok(Some((true, crate::tripcode::secure_tripcode(seed, key)?)))
    } else if let Some(seed) = raw.strip_prefix('#') {
        Ok(Some((false, crate::tripcode::tripcode(seed)?)))
    } else {
        Ok(None)
    }
}

fn name_and_trip(
    state: &AppState,
    post: &DumpPost,
) -> Result<(Option<String>, Option<(bool, String)>), RelayError> {
    let plain = post.trip_plain.as_deref().unwrap_or("");
    let (name, trip_part) = parse_name_and_trip(plain);
    let trip = match trip_part {
        Some(t) => compute_trip(&t, &state.secure_tripcode_key)?,
        None => None,
    };
    Ok((name, trip))
}

async fn migrate_requests(
    state: &AppState,
    post: &DumpPost,
    thread: bool,
    trip: &Option<(bool, String)>,
) -> Result<(String, Vec<RequestV2>, Vec<u8>), RelayError> {
    let uid = uid_response(state, old_ip(post)).await?;
    let geo = post.force_geo.then(|| geo_response(&post.ip_country_code));
    let base = if thread {
        "board_apply_intent_uid"
    } else {
        "board_apply_thread_intent_uid"
    };
    let (function, requests) = match (geo, trip) {
        (Some(_), Some(_)) => (
            format!("{base}_geo_tripcode"),
            vec![RequestV2::Uid, RequestV2::Geo, RequestV2::Tripcode],
        ),
        (Some(_), None) => (
            format!("{base}_geo"),
            vec![RequestV2::Uid, RequestV2::Geo],
        ),
        (None, Some(_)) => (
            format!("{base}_tripcode"),
            vec![RequestV2::Uid, RequestV2::Tripcode],
        ),
        (None, None) => (base.to_string(), vec![RequestV2::Uid]),
    };
    let mut inner = uid;
    if let Some(code) = geo {
        inner.extend_from_slice(&code.to_le_bytes());
    }
    if let Some((secured, t)) = trip {
        inner.extend_from_slice(&trip_response(*secured, t)?);
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
            cursor: 0,
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
    let client = reqwest::Client::new();
    let mut posts: Vec<&DumpPost> = board.posts.iter().collect();
    posts.sort_by_key(|p| p.num);

    let mut post_count = state_file.boards[&slug].post_count;
    let mut cursor = state_file.boards[&slug].cursor;
    for post in posts.into_iter() {
        if post.num <= cursor {
            continue;
        }
        let seed = if post.op && post.parent != 0 {
            let op_seed = *state_file.keys.get(&post.parent).ok_or_else(|| {
                RelayError::Internal(format!("thread OP key {} not found", post.parent))
            })?;
            *state_file.keys.entry(post.num).or_insert(op_seed)
        } else {
            *state_file.keys.entry(post.num).or_insert_with(random_seed)
        };
        let signer = ed25519_dalek::SigningKey::from_bytes(&seed);
        let sender = Address::new(signer.verifying_key().to_bytes());
        if post.parent == 0 {
            let (thread_addr, op_addr, op_num) = migrate_thread(
                state,
                &signer,
                &sender,
                &client,
                opts,
                board_id,
                &state_file.mapping,
                post,
            )
            .await?;
            state_file.mapping.insert(
                post.num,
                Mapping {
                    board: board_id,
                    thread: thread_addr,
                    post: op_addr,
                    num: op_num,
                },
            );
        } else {
            let parent_mapping = state_file.mapping.get(&post.parent).ok_or_else(|| {
                RelayError::Internal(format!("parent {} not migrated", post.parent))
            })?;
            let thread_addr = parent_mapping.thread;
            let (post_addr, post_num) = migrate_post(
                state,
                &signer,
                &sender,
                &client,
                opts,
                board_id,
                &state_file.mapping,
                post,
                thread_addr,
            )
            .await?;
            state_file.mapping.insert(
                post.num,
                Mapping {
                    board: board_id,
                    thread: thread_addr,
                    post: post_addr,
                    num: post_num,
                },
            );
        }
        post_count += 1;
        cursor = post.num;
        let bs = state_file.boards.get_mut(&slug).unwrap();
        bs.post_count = post_count;
        bs.cursor = cursor;
        save_state(&opts.state_path, state_file);
        let new_num = state_file.mapping.get(&post.num).map(|m| m.num).unwrap_or(post_count);
        eprintln!("imported old#{} -> new #{}", post.num, new_num);
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
    signer: &ed25519_dalek::SigningKey,
    sender: &Address,
    client: &reqwest::Client,
    opts: &ImportOptions,
    board_id: Address,
    mapping: &HashMap<u64, Mapping>,
    post: &DumpPost,
) -> Result<(Address, Address, u64), RelayError> {
    let (name, trip) = name_and_trip(state, post)?;
    let (function, requests, responses) = migrate_requests(state, post, true, &trip).await?;
    let media_hashes = upload_files(state, client, opts, post).await?;
    let text_hash = upload_text(
        state,
        &post_parts(state.forum.root.id, board_id, &opts.board, mapping, post),
    )
    .await?;
    let topic_hash = upload_plaintext(state, post.subject.as_deref().unwrap_or("")).await?;
    let name_hash = upload_plaintext(state, name.as_deref().unwrap_or("")).await?;
    let keys = vote_keys(state, post).await?;

    let nonce = next_nonce(state, sender).await?;
    let mut payload = event("new_thread_migrate_v2");
    push_bytes(&mut payload, &post.timestamp_ms);
    push_bytes(&mut payload, &topic_hash);
    push_bytes(&mut payload, &text_hash);
    push_bytes(&mut payload, &media_hashes);
    push_bytes(&mut payload, &name_hash);
    push_bytes(&mut payload, &keys);
    push_bytes(&mut payload, &post.enable_multiple_votes);

    let intent = intent_for(
        sender,
        &function,
        requests,
        board_objects(state, sender, vec![board_id]),
        payload,
        nonce,
    );
    let sig = sign_intent(signer, &intent);
    let created = broadcast(state, &intent, sig, &responses).await?;
    let (thread_addr, _thread_num, post_addr, post_num) = find_created(state, &created).await?;
    Ok((
        thread_addr.ok_or_else(|| {
            RelayError::Internal(format!("created thread not found for old#{}", post.num))
        })?,
        post_addr.ok_or_else(|| {
            RelayError::Internal(format!("created post not found for old#{}", post.num))
        })?,
        post_num.ok_or_else(|| {
            RelayError::Internal(format!("created post number not found for old#{}", post.num))
        })?,
    ))
}

async fn migrate_post(
    state: &AppState,
    signer: &ed25519_dalek::SigningKey,
    sender: &Address,
    client: &reqwest::Client,
    opts: &ImportOptions,
    board_id: Address,
    mapping: &HashMap<u64, Mapping>,
    post: &DumpPost,
    thread_addr: Address,
) -> Result<(Address, u64), RelayError> {
    let (name, trip) = name_and_trip(state, post)?;
    let (function, requests, responses) = migrate_requests(state, post, false, &trip).await?;
    let media_hashes = upload_files(state, client, opts, post).await?;
    let text_hash = upload_text(
        state,
        &post_parts(state.forum.root.id, board_id, &opts.board, mapping, post),
    )
    .await?;
    let name_hash = upload_plaintext(state, name.as_deref().unwrap_or("")).await?;
    let keys = vote_keys(state, post).await?;

    let nonce = next_nonce(state, sender).await?;
    let mut payload = event("new_post_migrate_v2");
    push_bytes(&mut payload, &post.timestamp_ms);
    push_bytes(&mut payload, &thread_addr);
    push_bytes(&mut payload, &text_hash);
    push_bytes(&mut payload, &media_hashes);
    push_bytes(&mut payload, &name_hash);
    push_bytes(&mut payload, &keys);
    push_bytes(&mut payload, &post.enable_multiple_votes);

    let intent = intent_for(
        sender,
        &function,
        requests,
        board_objects(state, sender, vec![board_id, thread_addr]),
        payload,
        nonce,
    );
    let sig = sign_intent(signer, &intent);
    let created = broadcast(state, &intent, sig, &responses).await?;
    let (_thread_addr, _thread_num, post_addr, post_num) = find_created(state, &created).await?;
    Ok((
        post_addr.ok_or_else(|| {
            RelayError::Internal(format!("created post not found for old#{}", post.num))
        })?,
        post_num.ok_or_else(|| {
            RelayError::Internal(format!("created post number not found for old#{}", post.num))
        })?,
    ))
}

fn menu_to_bbcode(menu: &str) -> Option<String> {
    let menu = menu.trim();
    if menu.is_empty() || menu == "null" || menu == "[]" {
        return None;
    }
    let arr: Vec<serde_json::Value> = serde_json::from_str(menu).ok()?;
    let mut out = String::new();
    for section in arr {
        let name = section
            .get("sectionName")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !name.is_empty() {
            out.push_str(&format!("[b]{name}[/b]\n"));
        }
        if let Some(links) = section.get("links").and_then(|v| v.as_array()) {
            for link in links {
                let label = link.get("label").and_then(|v| v.as_str()).unwrap_or("");
                let url = link.get("url").and_then(|v| v.as_str()).unwrap_or("");
                if !label.is_empty() && !url.is_empty() {
                    out.push_str(&format!("[link url={url}]{label}[/link]\n"));
                }
            }
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn post_parts(
    forum: Address,
    board_id: Address,
    slug: &str,
    mapping: &HashMap<u64, Mapping>,
    post: &DumpPost,
) -> Vec<PostPart> {
    let mut text = post.comment.clone().unwrap_or_default();
    if post.parent == 0 {
        if let Some(menu) = post.menu.as_deref() {
            if let Some(bb) = menu_to_bbcode(menu) {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(&bb);
            }
        }
    }
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    bbcode_to_parts(&text, forum, board_id, slug, mapping)
}

fn bbcode_to_parts(
    text: &str,
    forum: Address,
    board_id: Address,
    slug: &str,
    mapping: &HashMap<u64, Mapping>,
) -> Vec<PostPart> {
    let mut out = Vec::new();
    let mut i = 0;
    let mut plain = String::new();
    let chars: Vec<char> = text.chars().collect();
    while i < chars.len() {
        if chars[i] == '>' && i + 1 < chars.len() && chars[i + 1] == '>' {
            let mut j = i + 2;
            let start = j;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            if j > start {
                let num: u64 = chars[start..j].iter().collect::<String>().parse().unwrap_or(0);
                if let Some(m) = mapping.get(&num) {
                    if m.board == board_id {
                        if !plain.is_empty() {
                            out.push(PostPart::Plain(std::mem::take(&mut plain)));
                        }
                        out.push(PostPart::ReplyTo(
                            forum,
                            m.board,
                            m.post,
                            format!("{slug}/{}", m.num),
                        ));
                        i = j;
                        continue;
                    }
                }
            }
            plain.push_str(">>");
            i += 2;
            continue;
        }
        if chars[i] == '[' {
            if let Some(end) = find_close(&chars, i) {
                let tag: String = chars[i + 1..end].iter().collect();
                if let Some((variant, close)) = simple_tag(&tag) {
                    if let Some(close_idx) = find_str(&chars, close, end + 1) {
                        if !plain.is_empty() {
                            out.push(PostPart::Plain(std::mem::take(&mut plain)));
                        }
                        let inner: String = chars[end + 1..close_idx].iter().collect();
                        let children = bbcode_to_parts(&inner, forum, board_id, slug, mapping);
                        out.push(make_variant(variant, children));
                        i = close_idx + close.len();
                        continue;
                    }
                } else if let Some(url) = tag.strip_prefix("url=") {
                    if let Some(close_idx) = find_str(&chars, "[/url]", end + 1) {
                        if !plain.is_empty() {
                            out.push(PostPart::Plain(std::mem::take(&mut plain)));
                        }
                        let inner: String = chars[end + 1..close_idx].iter().collect();
                        let children = bbcode_to_parts(&inner, forum, board_id, slug, mapping);
                        out.push(PostPart::Link {
                            url: url.to_string(),
                            children,
                        });
                        i = close_idx + "[/url]".len();
                        continue;
                    }
                } else if let Some(url) = tag.strip_prefix("link url=") {
                    if let Some(close_idx) = find_str(&chars, "[/link]", end + 1) {
                        if !plain.is_empty() {
                            out.push(PostPart::Plain(std::mem::take(&mut plain)));
                        }
                        let inner: String = chars[end + 1..close_idx].iter().collect();
                        let children = bbcode_to_parts(&inner, forum, board_id, slug, mapping);
                        out.push(PostPart::Link {
                            url: url.to_string(),
                            children,
                        });
                        i = close_idx + "[/link]".len();
                        continue;
                    }
                }
            }
        }
        plain.push(chars[i]);
        i += 1;
    }
    if !plain.is_empty() {
        out.push(PostPart::Plain(plain));
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
        let mut payload = event("set_reaction_v2");
        push_bytes(&mut payload, &None::<Address>);
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
        vec![RequestV2::Uid],
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
    let nonce_shard = shard_id(&state.forum.projection.nonce_shards(), admin_pk);
    let mut payload = event(tag);
    fields(&mut payload);
    let intent = intent_for(
        admin_pk,
        "thread_apply_intent_uid",
        vec![RequestV2::Uid],
        vec![
            IntentObject {
                id: nonce_shard,
                mutable: true,
            },
            IntentObject {
                id: state.forum.root.id,
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
        vec![RequestV2::Uid, RequestV2::Ip32(post_id)],
        board_objects(state, admin_pk, vec![board_id, post_id]),
        payload,
        nonce,
    );
    let sig = sign_intent(admin, &intent);
    broadcast(state, &intent, sig, responses).await?;
    Ok(())
}
