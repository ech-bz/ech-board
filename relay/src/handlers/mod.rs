pub(crate) mod admin;
pub(crate) mod bans;
pub(crate) mod board;
pub(crate) mod content;
pub(crate) mod decrypt;
pub(crate) mod feed;
pub(crate) mod forum;
pub(crate) mod nonce;
pub(crate) mod post;
pub(crate) mod reactions;
pub(crate) mod send;
pub(crate) mod thread;

use crate::seaweed::SeaweedClient;
use crate::types::{ContentKind, MediaMeta, Tripcode};
use actix_web::HttpRequest;
use futures::StreamExt;
use futures::TryStreamExt;
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
    pub(super) forum_admin: Option<Address>,
    pub(super) forum_mods: Vec<Address>,
    pub(super) board_mods: Vec<Address>,
    pub(super) thread_mods: Vec<Address>,
    pub(super) thread_admin: Option<Address>,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct ForumObject {
    pub(super) id: Address,
    pub(super) entity: Entity,
    pub(super) projection: ForumProjection,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) enum ForumProjection {
    V1 {
        nonce_shards: Address,
        admin: Address,
        mods: Table,
        bans: Bans,
        boards: Table,
        timestamp_precision_ms: u64,
    },
}

impl ForumProjection {
    pub fn nonce_shards(&self) -> Address {
        match self {
            ForumProjection::V1 { nonce_shards, .. } => *nonce_shards,
        }
    }

    pub fn admin(&self) -> Address {
        match self {
            ForumProjection::V1 { admin, .. } => *admin,
        }
    }

    pub fn mods(&self) -> &Table {
        match self {
            ForumProjection::V1 { mods, .. } => mods,
        }
    }

    pub fn bans(&self) -> &Bans {
        match self {
            ForumProjection::V1 { bans, .. } => bans,
        }
    }

    pub fn boards(&self) -> &Table {
        match self {
            ForumProjection::V1 { boards, .. } => boards,
        }
    }

    #[allow(dead_code)]
    pub fn timestamp_precision_ms(&self) -> u64 {
        match self {
            ForumProjection::V1 { timestamp_precision_ms, .. } => *timestamp_precision_ms,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct BoardObject {
    pub(super) id: Address,
    pub(super) entity: Entity,
    pub(super) projection: BoardProjection,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) enum BoardProjection {
    V1 {
        slug: String,
        description_hash: Option<Address>,
        max_media: u64,
        bump_limit: u64,
        closed: bool,
        deleted: bool,
        ignore_forum_bans: bool,
        mods: Table,
        bans: Bans,
        reactions: Vec<Address>,
        threads: Table,
        posts: Table,
        bumps: Feed,
    },
    V2 {
        slug: String,
        description_hash: Option<Address>,
        max_media: u64,
        bump_limit: u64,
        closed: bool,
        deleted: bool,
        pinned: Vec<Address>,
        ignore_forum_bans: bool,
        mods: Table,
        bans: Bans,
        reactions: Vec<Address>,
        threads: Table,
        posts: Table,
        bumps: Feed,
    },
}

impl BoardProjection {
    pub fn slug(&self) -> &str {
        match self {
            BoardProjection::V1 { slug, .. } => slug,
            BoardProjection::V2 { slug, .. } => slug,
        }
    }

    pub fn description_hash(&self) -> Option<Address> {
        match self {
            BoardProjection::V1 { description_hash, .. } => *description_hash,
            BoardProjection::V2 { description_hash, .. } => *description_hash,
        }
    }

    pub fn max_media(&self) -> u64 {
        match self {
            BoardProjection::V1 { max_media, .. } => *max_media,
            BoardProjection::V2 { max_media, .. } => *max_media,
        }
    }

    pub fn deleted(&self) -> bool {
        match self {
            BoardProjection::V1 { deleted, .. } => *deleted,
            BoardProjection::V2 { deleted, .. } => *deleted,
        }
    }

    pub fn ignore_forum_bans(&self) -> bool {
        match self {
            BoardProjection::V1 { ignore_forum_bans, .. } => *ignore_forum_bans,
            BoardProjection::V2 { ignore_forum_bans, .. } => *ignore_forum_bans,
        }
    }

    pub fn mods(&self) -> &Table {
        match self {
            BoardProjection::V1 { mods, .. } => mods,
            BoardProjection::V2 { mods, .. } => mods,
        }
    }

    pub fn bans(&self) -> &Bans {
        match self {
            BoardProjection::V1 { bans, .. } => bans,
            BoardProjection::V2 { bans, .. } => bans,
        }
    }

    pub fn reactions(&self) -> &[Address] {
        match self {
            BoardProjection::V1 { reactions, .. } => reactions,
            BoardProjection::V2 { reactions, .. } => reactions,
        }
    }

    pub fn threads(&self) -> &Table {
        match self {
            BoardProjection::V1 { threads, .. } => threads,
            BoardProjection::V2 { threads, .. } => threads,
        }
    }

    pub fn posts(&self) -> &Table {
        match self {
            BoardProjection::V1 { posts, .. } => posts,
            BoardProjection::V2 { posts, .. } => posts,
        }
    }

    pub fn bumps(&self) -> &Feed {
        match self {
            BoardProjection::V1 { bumps, .. } => bumps,
            BoardProjection::V2 { bumps, .. } => bumps,
        }
    }

    pub fn pinned(&self) -> &[Address] {
        match self {
            BoardProjection::V1 { .. } => &[],
            BoardProjection::V2 { pinned, .. } => pinned,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct ThreadObject {
    pub(super) id: Address,
    pub(super) entity: Entity,
    pub(super) projection: ThreadProjection,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) enum ThreadProjection {
    V1 {
        board: Address,
        number: u64,
        topic_hash: Option<Address>,
        op: Address,
        closed: bool,
        deleted: bool,
        pinned: bool,
        admin: Option<Address>,
        mods: Table,
        bans: Bans,
        posts: Feed,
        last_3: Vec<Address>,
    },
    V2 {
        board: Address,
        number: u64,
        topic_hash: Option<Address>,
        op: Address,
        closed: bool,
        deleted: bool,
        admin: Option<Address>,
        mods: Table,
        bans: Bans,
        posts: Feed,
        last_3: Vec<Address>,
    },
    V3 {
        board: Address,
        number: u64,
        topic_hash: Option<Address>,
        op: Address,
        closed: bool,
        deleted: bool,
        admin: Option<Address>,
        mods: Table,
        bans: Bans,
        posts: Feed,
        posts_deleted: u64,
        last_3: Vec<Address>,
    },
}

impl ThreadProjection {
    pub fn board(&self) -> Address {
        match self {
            ThreadProjection::V1 { board, .. } => *board,
            ThreadProjection::V2 { board, .. } => *board,
            ThreadProjection::V3 { board, .. } => *board,
        }
    }

    pub fn number(&self) -> u64 {
        match self {
            ThreadProjection::V1 { number, .. } => *number,
            ThreadProjection::V2 { number, .. } => *number,
            ThreadProjection::V3 { number, .. } => *number,
        }
    }

    pub fn topic_hash(&self) -> Option<Address> {
        match self {
            ThreadProjection::V1 { topic_hash, .. } => *topic_hash,
            ThreadProjection::V2 { topic_hash, .. } => *topic_hash,
            ThreadProjection::V3 { topic_hash, .. } => *topic_hash,
        }
    }

    pub fn op(&self) -> Address {
        match self {
            ThreadProjection::V1 { op, .. } => *op,
            ThreadProjection::V2 { op, .. } => *op,
            ThreadProjection::V3 { op, .. } => *op,
        }
    }

    pub fn deleted(&self) -> bool {
        match self {
            ThreadProjection::V1 { deleted, .. } => *deleted,
            ThreadProjection::V2 { deleted, .. } => *deleted,
            ThreadProjection::V3 { deleted, .. } => *deleted,
        }
    }

    pub fn admin(&self) -> &Option<Address> {
        match self {
            ThreadProjection::V1 { admin, .. } => admin,
            ThreadProjection::V2 { admin, .. } => admin,
            ThreadProjection::V3 { admin, .. } => admin,
        }
    }

    pub fn mods(&self) -> &Table {
        match self {
            ThreadProjection::V1 { mods, .. } => mods,
            ThreadProjection::V2 { mods, .. } => mods,
            ThreadProjection::V3 { mods, .. } => mods,
        }
    }

    pub fn bans(&self) -> &Bans {
        match self {
            ThreadProjection::V1 { bans, .. } => bans,
            ThreadProjection::V2 { bans, .. } => bans,
            ThreadProjection::V3 { bans, .. } => bans,
        }
    }

    pub fn posts(&self) -> &Feed {
        match self {
            ThreadProjection::V1 { posts, .. } => posts,
            ThreadProjection::V2 { posts, .. } => posts,
            ThreadProjection::V3 { posts, .. } => posts,
        }
    }

    pub fn last_3(&self) -> &[Address] {
        match self {
            ThreadProjection::V1 { last_3, .. } => last_3,
            ThreadProjection::V2 { last_3, .. } => last_3,
            ThreadProjection::V3 { last_3, .. } => last_3,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct PostObject {
    pub(super) id: Address,
    pub(super) entity: Entity,
    pub(super) projection: PostProjection,
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
pub(super) enum PostProjection {
    V1 {
        sender: Sender,
        thread: Address,
        number: u64,
        uid: Vec<u8>,
        timestamp_ms: u64,
        deleted: bool,
        banned: Option<BanKey>,
        text_hash: Option<Address>,
        media_hashes: Vec<Address>,
        reactions: Vec<(Address, u64)>,
        votes: Vec<(Address, u64)>,
        name_hash: Option<Address>,
        trip: Option<Tripcode>,
        geo: Option<u32>,
        mod_note: Option<Address>,
    },
    V2 {
        sender: Sender,
        thread: Address,
        number: u64,
        uid: Vec<u8>,
        timestamp_ms: u64,
        deleted: bool,
        banned: Option<BanKey>,
        text_hash: Option<Address>,
        media_hashes: Vec<Address>,
        reactions: Vec<(Address, u64)>,
        votes: Vec<(Address, u64)>,
        multi_vote: bool,
        name_hash: Option<Address>,
        trip: Option<Tripcode>,
        geo: Option<u32>,
        mod_note: Option<Address>,
    },
    V3 {
        sender: Sender,
        thread: Address,
        number: u64,
        uid: Vec<u8>,
        timestamp_ms: u64,
        deleted: bool,
        banned: Option<BanKey>,
        text_hash: Option<Address>,
        media_hashes: Vec<Address>,
        banned_media: Vec<Address>,
        reactions: Vec<(Address, u64)>,
        votes: Vec<(Address, u64)>,
        multi_vote: bool,
        name_hash: Option<Address>,
        trip: Option<Tripcode>,
        geo: Option<u32>,
        mod_note: Option<Address>,
    },
}

impl PostProjection {
    pub fn thread(&self) -> Address {
        match self {
            PostProjection::V1 { thread, .. } => *thread,
            PostProjection::V2 { thread, .. } => *thread,
            PostProjection::V3 { thread, .. } => *thread,
        }
    }

    pub fn number(&self) -> u64 {
        match self {
            PostProjection::V1 { number, .. } => *number,
            PostProjection::V2 { number, .. } => *number,
            PostProjection::V3 { number, .. } => *number,
        }
    }

    pub fn deleted(&self) -> bool {
        match self {
            PostProjection::V1 { deleted, .. } => *deleted,
            PostProjection::V2 { deleted, .. } => *deleted,
            PostProjection::V3 { deleted, .. } => *deleted,
        }
    }

    pub fn text_hash(&self) -> Option<Address> {
        match self {
            PostProjection::V1 { text_hash, .. } => *text_hash,
            PostProjection::V2 { text_hash, .. } => *text_hash,
            PostProjection::V3 { text_hash, .. } => *text_hash,
        }
    }

    pub fn media_hashes(&self) -> &[Address] {
        match self {
            PostProjection::V1 { media_hashes, .. } => media_hashes,
            PostProjection::V2 { media_hashes, .. } => media_hashes,
            PostProjection::V3 { media_hashes, .. } => media_hashes,
        }
    }

    pub fn name_hash(&self) -> Option<Address> {
        match self {
            PostProjection::V1 { name_hash, .. } => *name_hash,
            PostProjection::V2 { name_hash, .. } => *name_hash,
            PostProjection::V3 { name_hash, .. } => *name_hash,
        }
    }

    pub fn banned_media(&self) -> &[Address] {
        match self {
            PostProjection::V3 { banned_media, .. } => banned_media,
            _ => &[],
        }
    }
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
    #[allow(dead_code)]
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
    let version = root.entity.version;
    let projection = match version {
        1 => ForumProjection::V1 {
            nonce_shards: fields.get(b"nonce_shards")?,
            admin: fields.get(b"admin")?,
            mods: fields.get(b"moderators")?,
            bans: fields.get(b"bans")?,
            boards: fields.get(b"boards")?,
            timestamp_precision_ms: fields.get(b"timestamp_precision")?,
        },
        _ => return Err(crate::error::RelayError::Internal(format!(
            "forum version {version} not supported"
        ))),
    };
    Ok(ForumObject {
        id,
        entity: Entity { feed: root.entity.feed, version },
        projection,
    })
}

fn decode_board(
    id: Address,
    root: EntityRoot,
    fields: DynamicFields,
) -> Result<BoardObject, crate::error::RelayError> {
    let version = root.entity.version;
    let projection = match version {
        1 => BoardProjection::V1 {
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
            threads: fields.get(b"threads")?,
            posts: fields.get(b"posts")?,
            bumps: fields.get(b"bumps")?,
        },
        2 => BoardProjection::V2 {
            slug: fields.get(b"slug")?,
            description_hash: fields.get(b"description_hash")?,
            max_media: fields.get(b"max_media")?,
            bump_limit: fields.get(b"bump_limit")?,
            closed: fields.get(b"closed")?,
            deleted: fields.get(b"deleted")?,
            pinned: fields.get(b"pinned")?,
            ignore_forum_bans: fields.get(b"ignore_forum_bans")?,
            mods: fields.get(b"moderators")?,
            bans: fields.get(b"bans")?,
            reactions: fields.get(b"reactions")?,
            threads: fields.get(b"threads")?,
            posts: fields.get(b"posts")?,
            bumps: fields.get(b"bumps")?,
        },
        _ => return Err(crate::error::RelayError::Internal(format!(
            "board version {version} not supported"
        ))),
    };
    Ok(BoardObject {
        id,
        entity: Entity { feed: root.entity.feed, version },
        projection,
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
    .buffer_unordered(64)
    .collect::<Vec<_>>()
    .await
    .into_iter()
    .collect::<Result<Vec<_>, crate::error::RelayError>>()?;
    Ok((posts, board))
}

fn decode_thread(
    id: Address,
    root: EntityRoot,
    fields: DynamicFields,
) -> Result<ThreadObject, crate::error::RelayError> {
    let version = root.entity.version;
    let projection = match version {
        1 => ThreadProjection::V1 {
            board: fields.get(b"board")?,
            number: fields.get(b"number")?,
            topic_hash: fields.get(b"topic_hash")?,
            op: fields.get(b"op")?,
            closed: fields.get(b"closed")?,
            deleted: fields.get(b"deleted")?,
            pinned: fields.get(b"pinned")?,
            admin: fields.get(b"admin")?,
            mods: fields.get(b"moderators")?,
            bans: fields.get(b"bans")?,
            posts: fields.get(b"posts")?,
            last_3: fields.get(b"last_posts")?,
        },
        2 => ThreadProjection::V2 {
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
        3 => ThreadProjection::V3 {
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
            posts_deleted: fields.get(b"posts_deleted")?,
            last_3: fields.get(b"last_posts")?,
        },
        _ => return Err(crate::error::RelayError::Internal(format!(
            "thread version {version} not supported"
        ))),
    };
    Ok(ThreadObject {
        id,
        entity: Entity { feed: root.entity.feed, version },
        projection,
    })
}

pub(super) async fn load_thread(
    upstream: &crate::upstream::UpstreamSender,
    id: Address,
) -> Result<ThreadObject, crate::error::RelayError> {
    let (root, fields) = tokio::join!(
        load_root(upstream, id),
        DynamicFields::load(upstream, id),
    );
    decode_thread(id, root?, fields?)
}

pub(super) async fn load_threads(
    upstream: &crate::upstream::UpstreamSender,
    ids: &[Address],
) -> Result<HashMap<Address, ThreadObject>, crate::error::RelayError> {
    let objects = upstream.fetch_objects(ids.iter().copied()).await?;
    let threads = futures::stream::iter(ids.iter().zip(objects.into_iter()).map(|(id, object)| async move {
        let root = object
            .as_ref()
            .ok_or_else(|| {
                crate::error::RelayError::Internal(format!("thread object {id} not found"))
            })?
            .contents()
            .deserialize::<EntityRoot>()
            .map_err(|e| {
                crate::error::RelayError::Internal(format!("thread root {id} decode: {e}"))
            })?;
        let fields = DynamicFields::load(upstream, *id).await?;
        decode_thread(*id, root, fields)
    }))
    .buffer_unordered(64)
    .try_collect::<Vec<_>>()
    .await?;
    Ok(threads.into_iter().map(|t| (t.id, t)).collect())
}

fn decode_post(
    id: Address,
    root: EntityRoot,
    fields: DynamicFields,
) -> Result<PostObject, crate::error::RelayError> {
    let version = root.entity.version;
    let sender: Sender = fields.get(b"sender")?;
    let projection = match version {
        1 => PostProjection::V1 {
            sender: sender.clone(),
            thread: fields.get(b"thread")?,
            number: fields.get(b"number")?,
            uid: fields.get(b"uid")?,
            timestamp_ms: fields.get(b"timestamp_ms")?,
            deleted: fields.get(b"deleted")?,
            banned: fields.get(b"banned")?,
            text_hash: fields.get(b"text_hash")?,
            media_hashes: fields.get(b"media_hashes")?,
            reactions: fields.get(b"reactions")?,
            votes: fields.get(b"votes")?,
            name_hash: fields.get(b"name")?,
            trip: fields.get(b"trip")?,
            geo: fields.get(b"geo")?,
            mod_note: fields.get(b"mod_note")?,
        },
        2 => PostProjection::V2 {
            sender: sender.clone(),
            thread: fields.get(b"thread")?,
            number: fields.get(b"number")?,
            uid: fields.get(b"uid")?,
            timestamp_ms: fields.get(b"timestamp_ms")?,
            deleted: fields.get(b"deleted")?,
            banned: fields.get(b"banned")?,
            text_hash: fields.get(b"text_hash")?,
            media_hashes: fields.get(b"media_hashes")?,
            reactions: fields.get(b"reactions")?,
            votes: fields.get(b"votes")?,
            multi_vote: fields.get(b"multi_vote")?,
            name_hash: fields.get(b"name")?,
            trip: fields.get(b"trip")?,
            geo: fields.get(b"geo")?,
            mod_note: fields.get(b"mod_note")?,
        },
        3 => PostProjection::V3 {
            sender: sender.clone(),
            thread: fields.get(b"thread")?,
            number: fields.get(b"number")?,
            uid: fields.get(b"uid")?,
            timestamp_ms: fields.get(b"timestamp_ms")?,
            deleted: fields.get(b"deleted")?,
            banned: fields.get(b"banned")?,
            text_hash: fields.get(b"text_hash")?,
            media_hashes: fields.get(b"media_hashes")?,
            banned_media: fields.get(b"banned_media")?,
            reactions: fields.get(b"reactions")?,
            votes: fields.get(b"votes")?,
            multi_vote: fields.get(b"multi_vote")?,
            name_hash: fields.get(b"name")?,
            trip: fields.get(b"trip")?,
            geo: fields.get(b"geo")?,
            mod_note: fields.get(b"mod_note")?,
        },
        _ => return Err(crate::error::RelayError::Internal(format!(
            "post version {version} not supported"
        ))),
    };
    Ok(PostObject {
        id,
        entity: Entity { feed: root.entity.feed, version },
        projection,
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

pub(super) async fn load_posts(
    upstream: &crate::upstream::UpstreamSender,
    ids: &[Address],
) -> Result<HashMap<Address, PostObject>, crate::error::RelayError> {
    let objects = upstream.fetch_objects(ids.iter().copied()).await?;
    let posts = futures::stream::iter(ids.iter().zip(objects.into_iter()).map(|(id, object)| async move {
        let root = object
            .as_ref()
            .ok_or_else(|| {
                crate::error::RelayError::Internal(format!("post object {id} not found"))
            })?
            .contents()
            .deserialize::<EntityRoot>()
            .map_err(|e| {
                crate::error::RelayError::Internal(format!("post root {id} decode: {e}"))
            })?;
        let fields = DynamicFields::load(upstream, *id).await?;
        decode_post(*id, root, fields)
    }))
    .buffer_unordered(64)
    .try_collect::<Vec<_>>()
    .await?;
    Ok(posts.into_iter().map(|p| (p.id, p)).collect())
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
    let fetched: Vec<(Address, Option<MediaMeta>)> = futures::stream::iter(
        hashes.into_iter().map(|h| async move {
            match seaweed.get(ContentKind::MediaMeta, &h).await {
                Ok(Some(data)) => match bcs::from_bytes::<MediaMeta>(&data) {
                    Ok(meta) => (h, Some(meta)),
                    Err(_) => (h, None),
                },
                _ => (h, None),
            }
        }),
    )
    .buffer_unordered(32)
    .collect()
    .await;
    for (h, meta) in fetched {
        match meta {
            Some(meta) => {
                out.insert(h, meta);
            }
            None => missing.push(h),
        }
    }

    let lazy: Vec<(Address, Option<MediaMeta>)> = futures::stream::iter(
        missing.into_iter().map(|h| async move {
            (h, lazy_media_meta(seaweed, h).await)
        }),
    )
    .buffer_unordered(8)
    .collect()
    .await;
    for (h, meta) in lazy {
        if let Some(meta) = meta {
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
