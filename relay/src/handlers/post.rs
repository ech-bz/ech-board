use std::collections::{HashMap, HashSet};

use crate::app_state::AppState;
use crate::cache::CACHE_NS;
use crate::error::RelayError;
use serde::{Deserialize, Serialize};
use sui_sdk_types::Address;

use super::fetch_content;
use super::fetch_media_meta;
use super::{DynamicFields, load_roots_fields};
use super::board::{BoardObject, load_board};
use super::thread::{ThreadObject, load_thread};
use super::{Moderators, list_mods};
use crate::types::{BanKey, ContentKind, EntityRoot, MediaMeta, Sender, Tripcode};

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct PostObject {
    pub(crate) root: EntityRoot,
    pub(crate) projection: PostProjection,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) enum PostProjection {
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

pub(super) fn decode_post(
    root: EntityRoot,
    fields: DynamicFields,
) -> Result<PostObject, RelayError> {
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
        _ => return Err(RelayError::Internal(format!(
            "post version {version} not supported"
        ))),
    };
    Ok(PostObject {
        root,
        projection,
    })
}

pub(crate) async fn load_post(
    upstream: &crate::upstream::UpstreamSender,
    id: Address,
) -> Result<PostObject, RelayError> {
    load_posts(upstream, &[id])
        .await?
        .remove(&id)
        .ok_or_else(|| RelayError::Internal(format!("post {id} not found")))
}

pub(crate) async fn load_posts(
    upstream: &crate::upstream::UpstreamSender,
    ids: &[Address],
) -> Result<HashMap<Address, PostObject>, RelayError> {
    let (roots, mut fields) = load_roots_fields(upstream, ids).await?;
    let posts = ids
        .iter()
        .zip(roots)
        .map(|(id, root)| {
            let fields = fields
                .remove(id)
                .ok_or_else(|| RelayError::Internal(format!("post fields {id} missing")))?;
            decode_post(root, fields)
        })
        .collect::<Result<Vec<_>, RelayError>>()?;
    Ok(posts.into_iter().map(|p| (p.root.id, p)).collect())
}

#[derive(Serialize)]
pub(crate) struct PostView {
    pub(crate) post: PostObject,
    pub(crate) thread: ThreadObject,
    pub(crate) board: BoardObject,
    pub(crate) text: HashMap<Address, Vec<u8>>,
    pub(crate) media_meta: HashMap<Address, MediaMeta>,
    pub(crate) moderators: Moderators,
}

pub(crate) async fn fetch(state: &AppState, post_uid: Address) -> Result<Vec<u8>, RelayError> {
    let key = format!("{CACHE_NS}:post:{post_uid}");
    state
        .cache
        .get_or_build(key, async {
            let post = load_post(&state.upstream, post_uid).await?;
            if post.projection.deleted() {
                return Err(RelayError::NotFound("post deleted".into()));
            }
            let thread = load_thread(&state.upstream, post.projection.thread()).await?;
            if thread.projection.deleted() {
                return Err(RelayError::NotFound("thread deleted".into()));
            }
            let board = load_board(&state.upstream, thread.projection.board()).await?;
            if board.projection.deleted() {
                return Err(RelayError::NotFound("board deleted".into()));
            }

            let mut text_hashes = HashSet::new();
            if let Some(h) = post.projection.text_hash() {
                text_hashes.insert(h);
            }
            let text = fetch_content(&state.seaweed, ContentKind::Text, text_hashes).await;

            let media_hashes: HashSet<Address> =
                post.projection.media_hashes().iter().copied().collect();
            let media_meta = fetch_media_meta(&state.seaweed, media_hashes).await;

            let moderators = Moderators {
                forum_admin: Some(state.forum.projection.admin()),
                forum_mods: list_mods(&state.upstream, state.forum.projection.mods().id).await?,
                board_mods: list_mods(&state.upstream, board.projection.mods().id).await?,
                thread_mods: list_mods(&state.upstream, thread.projection.mods().id).await?,
                thread_admin: *thread.projection.admin(),
            };

            let response = PostView {
                post,
                thread,
                board,
                text,
                media_meta,
                moderators,
            };

            bcs::to_bytes(&response)
                .map_err(|e| RelayError::Internal(format!("bcs encode PostView: {e}")))
        })
        .await
}
