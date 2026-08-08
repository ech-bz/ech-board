pub(crate) mod admin;
pub(crate) mod bans;
pub(crate) mod board;
pub(crate) mod content;
pub(crate) mod decrypt;
pub(crate) mod feed;
pub(crate) mod forum;
pub(crate) mod nonce;
pub(crate) mod post;
pub(crate) mod send;
pub(crate) mod thread;

use crate::seaweed::SeaweedClient;
use crate::types::{ContentKind, MediaMeta, Tripcode};
use actix_web::HttpRequest;
use futures::StreamExt;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use sui_sdk_types::Address;

#[derive(Deserialize)]
pub(crate) struct Pagination {
    pub(crate) cursor: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct Table {
    pub(super) id: Address,
    pub(super) size: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct Feed {
    pub(super) id: Address,
    pub(super) counter: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct Moderators {
    pub(super) forum_mods: Vec<Address>,
    pub(super) board_mods: Vec<Address>,
    pub(super) thread_mods: Vec<Address>,
    pub(super) forum_admin: Option<Address>,
    pub(super) thread_admin: Option<Address>,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct ForumObject {
    pub(super) id: Address,
    pub(super) entity: Entity,
    pub(super) projection: ForumProjection,
    pub(super) genesis: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct ForumProjection {
    pub(super) nonce_shards: Address,
    pub(super) admin: Address,
    pub(super) mods: Table,
    pub(super) bans: Bans,
    pub(super) boards: Table,
    pub(super) timestamp_precision_ms: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct BoardObject {
    pub(super) id: Address,
    pub(super) entity: Entity,
    pub(super) projection: BoardProjection,
    pub(super) genesis: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct BoardProjection {
    pub(super) slug: String,
    pub(super) description_hash: Option<Address>,
    pub(super) max_media: u64,
    pub(super) bump_limit: u64,
    pub(super) closed: bool,
    pub(super) deleted: bool,
    pub(super) ignore_forum_bans: bool,
    pub(super) mods: Table,
    pub(super) bans: Bans,
    pub(super) reactions: Vec<Address>,
    pub(super) pinned: Vec<Address>,
    pub(super) threads: Table,
    pub(super) posts: Table,
    pub(super) bumps: Feed,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct ThreadObject {
    pub(super) id: Address,
    pub(super) entity: Entity,
    pub(super) projection: ThreadProjection,
    pub(super) genesis: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct ThreadProjection {
    pub(super) board: Address,
    pub(super) number: u64,
    pub(super) topic_hash: Option<Address>,
    pub(super) op: Address,
    pub(super) closed: bool,
    pub(super) deleted: bool,
    pub(super) admin: Option<Address>,
    pub(super) mods: Table,
    pub(super) bans: Bans,
    pub(super) posts: Feed,
    pub(super) last_3: Vec<Address>,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct PostObject {
    pub(super) id: Address,
    pub(super) entity: Entity,
    pub(super) projection: PostProjection,
    pub(super) genesis: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct Sender {
    pub(super) pk: [u8; 32],
    pub(super) tweak: [u8; 32],
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct BanKey {
    pub(super) level: Address,
    pub(super) mask: u8,
    pub(super) ip_hash: Address,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct PostProjection {
    pub(super) thread: Address,
    pub(super) number: u64,
    pub(super) sender: Sender,
    pub(super) uid: Vec<u8>,
    pub(super) timestamp_ms: u64,
    pub(super) deleted: bool,
    pub(super) banned: Option<BanKey>,
    pub(super) text_hash: Option<Address>,
    pub(super) media_hashes: Vec<Address>,
    pub(super) banned_media: Vec<Address>,
    pub(super) name_hash: Option<Address>,
    pub(super) trip: Option<Tripcode>,
    pub(super) geo: Option<u32>,
    pub(super) mod_note: Option<Address>,
    pub(super) multi_vote: bool,
    pub(super) reactions: Vec<(Address, u64)>,
    pub(super) votes: Vec<(Address, u64)>,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct Registry {
    pub(super) counter: u64,
    pub(super) entries: Table,
    pub(super) identities: Table,
    pub(super) index: Table,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct Bans {
    pub(super) level: Address,
    pub(super) ip32: Registry,
    pub(super) ip24: Registry,
    pub(super) ip20: Registry,
    pub(super) ip16: Registry,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Shard {
    pub(super) id: Address,
    pub(super) shards: u64,
    pub(super) index: u64,
    pub(super) counters: Table,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct Entity {
    pub(super) feed: Feed,
    pub(super) version: u16,
}

#[derive(Deserialize)]
struct EntityRoot {
    #[allow(dead_code)]
    id: Address,
    entity: Entity,
    genesis: bool,
}

struct DynamicFields {
    values: HashMap<Vec<u8>, Vec<u8>>,
}

impl DynamicFields {
    async fn load(
        upstream: &crate::upstream::UpstreamSender,
        parent: Address,
    ) -> Result<Self, crate::error::RelayError> {
        let mut values = HashMap::new();
        for (encoded_name, _, value) in upstream.list_dynamic_fields(parent).await? {
            let name = bcs::from_bytes::<Vec<u8>>(&encoded_name).unwrap_or(encoded_name);
            if let Some(value) = value {
                values.insert(name, value);
            }
        }
        Ok(Self { values })
    }

    fn get<T: DeserializeOwned>(&self, name: &[u8]) -> Result<T, crate::error::RelayError> {
        let value = self.values.get(name).ok_or_else(|| {
            crate::error::RelayError::Internal(format!(
                "dynamic field '{}' not found",
                String::from_utf8_lossy(name),
            ))
        })?;
        bcs::from_bytes(value).map_err(|e| {
            crate::error::RelayError::Internal(format!(
                "dynamic field '{}' decode: {e}",
                String::from_utf8_lossy(name),
            ))
        })
    }

    fn get_or<T: DeserializeOwned>(&self, name: &[u8], default: T) -> T {
        self.values
            .get(name)
            .and_then(|value| bcs::from_bytes(value).ok())
            .unwrap_or(default)
    }
}

async fn load_root(
    upstream: &crate::upstream::UpstreamSender,
    id: Address,
) -> Result<EntityRoot, crate::error::RelayError> {
    upstream.fetch_objects([id]).await?[0]
        .as_ref()
        .ok_or_else(|| crate::error::RelayError::Internal(format!("object {id} not found")))?
        .contents()
        .deserialize::<EntityRoot>()
        .map_err(|e| crate::error::RelayError::Internal(format!("entity root {id} decode: {e}")))
}

async fn load_roots(
    upstream: &crate::upstream::UpstreamSender,
    ids: &[Address],
) -> Result<Vec<EntityRoot>, crate::error::RelayError> {
    let objects = upstream.fetch_objects(ids).await?;
    ids.iter()
        .zip(objects.into_iter())
        .map(|(id, object)| {
            object
                .as_ref()
                .ok_or_else(|| {
                    crate::error::RelayError::Internal(format!("object {id} not found"))
                })?
                .contents()
                .deserialize::<EntityRoot>()
                .map_err(|e| {
                    crate::error::RelayError::Internal(format!("entity root {id} decode: {e}"))
                })
        })
        .collect()
}

pub(super) async fn load_forum(
    upstream: &crate::upstream::UpstreamSender,
    id: Address,
) -> Result<ForumObject, crate::error::RelayError> {
    let (root, fields) = tokio::join!(
        load_root(upstream, id),
        DynamicFields::load(upstream, id),
    );
    let root = root?;
    let fields = fields?;
    Ok(ForumObject {
        id,
        entity: Entity { feed: root.entity.feed, version: root.entity.version },
        projection: ForumProjection {
            nonce_shards: fields.get(b"nonce_shards")?,
            admin: fields.get(b"admin")?,
            mods: fields.get(b"moderators")?,
            bans: fields.get(b"bans")?,
            boards: fields.get(b"boards")?,
            timestamp_precision_ms: fields.get(b"timestamp_precision")?,
        },
        genesis: root.genesis,
    })
}

fn decode_board(
    id: Address,
    root: EntityRoot,
    fields: DynamicFields,
) -> Result<BoardObject, crate::error::RelayError> {
    Ok(BoardObject {
        id,
        entity: Entity { feed: root.entity.feed, version: root.entity.version },
        projection: BoardProjection {
            slug: fields.get(b"slug")?,
            description_hash: fields.get(b"description_hash")?,
            max_media: fields.get(b"max_media")?,
            bump_limit: fields.get(b"bump_limit")?,
            closed: fields.get(b"closed")?,
            deleted: fields.get(b"deleted")?,
            ignore_forum_bans: fields.get(b"ignore_forum_bans")?,
            mods: fields.get(b"moderators")?,
            bans: fields.get(b"bans")?,
            reactions: fields.get(b"reactions")?,
            pinned: fields.get_or(b"pinned", vec![]),
            threads: fields.get(b"threads")?,
            posts: fields.get(b"posts")?,
            bumps: fields.get(b"bumps")?,
        },
        genesis: root.genesis,
    })
}

pub(super) async fn load_board(
    upstream: &crate::upstream::UpstreamSender,
    id: Address,
) -> Result<BoardObject, crate::error::RelayError> {
    let (root, fields) = tokio::join!(
        load_root(upstream, id),
        DynamicFields::load(upstream, id),
    );
    decode_board(id, root?, fields?)
}

async fn load_board_from_root(
    upstream: &crate::upstream::UpstreamSender,
    id: Address,
    root: EntityRoot,
) -> Result<BoardObject, crate::error::RelayError> {
    let fields = DynamicFields::load(upstream, id).await?;
    decode_board(id, root, fields)
}

pub(super) async fn load_posts_and_board(
    upstream: &crate::upstream::UpstreamSender,
    post_ids: Vec<Address>,
    board_id: Address,
) -> Result<(Vec<PostObject>, BoardObject), crate::error::RelayError> {
    let mut ids = post_ids.clone();
    ids.push(board_id);
    let mut roots = load_roots(upstream, &ids).await?.into_iter();
    let board_root = roots.next_back().ok_or_else(|| {
        crate::error::RelayError::Internal("load_posts_and_board: empty ids".to_string())
    })?;
    let board = load_board_from_root(upstream, board_id, board_root).await?;
    let posts = futures::stream::iter(post_ids.into_iter().zip(roots).map(|(id, root)| async move {
        let fields = DynamicFields::load(upstream, id).await?;
        decode_post(id, root, fields)
    }))
    .buffer_unordered(16)
    .collect::<Vec<_>>()
    .await
    .into_iter()
    .collect::<Result<Vec<_>, crate::error::RelayError>>()?;
    Ok((posts, board))
}

pub(super) async fn load_thread(
    upstream: &crate::upstream::UpstreamSender,
    id: Address,
) -> Result<ThreadObject, crate::error::RelayError> {
    let (root, fields) = tokio::join!(
        load_root(upstream, id),
        DynamicFields::load(upstream, id),
    );
    let root = root?;
    let fields = fields?;
    Ok(ThreadObject {
        id,
        entity: Entity { feed: root.entity.feed, version: root.entity.version },
        projection: ThreadProjection {
            board: fields.get(b"board")?,
            number: fields.get(b"number")?,
            topic_hash: fields.get(b"topic_hash")?,
            op: fields.get(b"op")?,
            closed: fields.get(b"closed")?,
            deleted: fields.get(b"deleted")?,
            admin: fields.get(b"admin")?,
            mods: fields.get(b"moderators")?,
            bans: fields.get(b"bans")?,
            posts: fields.get(b"posts")?,
            last_3: fields.get(b"last_posts")?,
        },
        genesis: root.genesis,
    })
}

fn decode_post(
    id: Address,
    root: EntityRoot,
    fields: DynamicFields,
) -> Result<PostObject, crate::error::RelayError> {
    let sender: Sender = fields.get(b"sender")?;
    Ok(PostObject {
        id,
        entity: Entity { feed: root.entity.feed, version: root.entity.version },
        projection: PostProjection {
            thread: fields.get(b"thread")?,
            number: fields.get(b"number")?,
            sender,
            uid: fields.get(b"uid")?,
            timestamp_ms: fields.get(b"timestamp_ms")?,
            deleted: fields.get(b"deleted")?,
            banned: fields.get(b"banned")?,
            text_hash: fields.get(b"text_hash")?,
            media_hashes: fields.get(b"media_hashes")?,
            banned_media: fields.get_or(b"banned_media", vec![]),
            name_hash: fields.get(b"name")?,
            trip: fields.get(b"trip")?,
            geo: fields.get(b"geo")?,
            mod_note: fields.get(b"mod_note")?,
            multi_vote: fields.get_or(b"multi_vote", false),
            reactions: fields.get(b"reactions")?,
            votes: fields.get(b"votes")?,
        },
        genesis: root.genesis,
    })
}

pub(super) async fn load_post(
    upstream: &crate::upstream::UpstreamSender,
    id: Address,
) -> Result<PostObject, crate::error::RelayError> {
    let (root, fields) = tokio::join!(
        load_root(upstream, id),
        DynamicFields::load(upstream, id),
    );
    decode_post(id, root?, fields?)
}

pub(super) async fn list_mods(
    upstream: &crate::upstream::UpstreamSender,
    mods_table_id: Address,
) -> Result<Vec<Address>, crate::error::RelayError> {
    let fields = upstream.list_dynamic_fields(mods_table_id).await?;
    let mut mods = Vec::with_capacity(fields.len());
    for (name_bytes, _, value) in fields {
        if value.is_none() {
            continue;
        }
        if let Ok(addr) = bcs::from_bytes::<Address>(&name_bytes) {
            mods.push(addr);
        }
    }
    Ok(mods)
}

pub(super) async fn fetch_content(
    seaweed: &SeaweedClient,
    kind: ContentKind,
    hashes: HashSet<Address>,
) -> HashMap<Address, Vec<u8>> {
    futures::stream::iter(hashes.into_iter().map(|addr| async move {
        for attempt in 0..3 {
            match seaweed.get(kind, &addr).await {
                Ok(Some(data)) => return Some((addr, data)),
                Ok(None) => return None,
                Err(_) if attempt < 2 => {
                    tokio::time::sleep(Duration::from_millis(50 * (attempt + 1))).await;
                }
                Err(_) => return None,
            }
        }
        None
    }))
    .buffer_unordered(32)
    .collect::<Vec<_>>()
    .await
    .into_iter()
    .flatten()
    .collect()
}

pub(super) async fn fetch_media_meta(
    seaweed: &SeaweedClient,
    hashes: HashSet<Address>,
) -> HashMap<Address, MediaMeta> {
    let mut out = HashMap::new();
    let mut missing = Vec::new();
    for h in hashes {
        match seaweed.get(ContentKind::MediaMeta, &h).await {
            Ok(Some(data)) => match bcs::from_bytes::<MediaMeta>(&data) {
                Ok(meta) => {
                    out.insert(h, meta);
                }
                Err(_) => missing.push(h),
            },
            _ => missing.push(h),
        }
    }
    for h in missing {
        if let Some(meta) = lazy_media_meta(seaweed, h).await {
            out.insert(h, meta);
        }
    }
    out
}

async fn lazy_media_meta(seaweed: &SeaweedClient, hash: Address) -> Option<MediaMeta> {
    let data = seaweed.get(ContentKind::Media, &hash).await.ok()??;
    let tmp = std::env::temp_dir().join(format!("ech-meta-{}.bin", hex::encode(hash.as_bytes())));
    if std::fs::write(&tmp, &data).is_err() {
        return None;
    }
    let meta = crate::thumbnail::compute_meta(&data, &tmp).ok();
    let _ = std::fs::remove_file(&tmp);
    if let Some(m) = &meta {
        if let Ok(meta_bcs) = bcs::to_bytes(m) {
            let _ = seaweed.put(ContentKind::MediaMeta, &hash, &meta_bcs).await;
        }
    }
    meta
}

pub(super) fn client_ip(req: &HttpRequest) -> Option<String> {
    req.connection_info()
        .realip_remote_addr()
        .map(|s| s.to_string())
}
