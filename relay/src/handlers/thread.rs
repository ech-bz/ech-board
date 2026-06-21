use std::collections::{HashMap, HashSet};

use crate::app_state::AppState;
use crate::cache::CACHE_NS;
use crate::error::RelayError;
use serde::{Deserialize, Serialize};
use sui_sdk_types::Address;

use super::fetch_content;
use super::fetch_media_meta;
use super::{DynamicFields, load_root_fields};
use super::board::load_posts_and_board;
use super::post::PostObject;
use super::{Moderators, list_mods};
use crate::types::{Bans, ContentKind, EntityRoot, Feed, MediaMeta, Table};
use futures::StreamExt;

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct ThreadObject {
    pub(crate) root: EntityRoot,
    pub(crate) projection: ThreadProjection,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) enum ThreadProjection {
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

pub(super) fn decode_thread(
    root: EntityRoot,
    fields: DynamicFields,
) -> Result<ThreadObject, RelayError> {
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
        _ => return Err(RelayError::Internal(format!(
            "thread version {version} not supported"
        ))),
    };
    Ok(ThreadObject {
        root,
        projection,
    })
}

pub(crate) async fn load_thread(
    upstream: &crate::upstream::UpstreamSender,
    id: Address,
) -> Result<ThreadObject, RelayError> {
    let (root, fields) = load_root_fields(upstream, id).await?;
    decode_thread(root, fields)
}

pub(crate) async fn load_threads(
    upstream: &crate::upstream::UpstreamSender,
    ids: &[Address],
) -> Result<HashMap<Address, ThreadObject>, RelayError> {
    let (objects, fields) = tokio::join!(
        upstream.fetch_objects(ids.iter().copied()),
        futures::stream::iter(ids.iter().copied().map(|id| async move {
            (id, DynamicFields::load(upstream, id).await)
        }))
        .buffer_unordered(64)
        .collect::<Vec<_>>(),
    );
    let objects = objects?;
    let mut fields: HashMap<Address, Result<DynamicFields, RelayError>> =
        fields.into_iter().collect();
    let threads = ids
        .iter()
        .zip(objects.into_iter())
        .filter_map(|(id, object)| {
            let root = object.as_ref()?.contents().deserialize::<EntityRoot>().ok()?;
            let fields = fields.remove(id)?.ok()?;
            decode_thread(root, fields).ok()
        })
        .collect::<Vec<_>>();
    Ok(threads.into_iter().map(|t| (t.root.id, t)).collect())
}

#[derive(Serialize)]
pub(crate) struct ThreadView {
    pub(crate) thread: ThreadObject,
    pub(crate) posts: Vec<PostObject>,
    pub(crate) text: HashMap<Address, Vec<u8>>,
    pub(crate) plain_text: HashMap<Address, Vec<u8>>,
    pub(crate) media_meta: HashMap<Address, MediaMeta>,
    pub(crate) moderators: Moderators,
}

pub(crate) async fn fetch(state: &AppState, thread_uid: Address) -> Result<Vec<u8>, RelayError> {
    let thread = load_thread(&state.upstream, thread_uid).await?;
    if thread.projection.deleted() {
        return Err(RelayError::NotFound("thread deleted".into()));
    }
    let pcounter = thread.projection.posts().counter;
    let rgen = state
        .cache
        .gen_get(&format!("gen:thread:{thread_uid}"))
        .await;
    let key = format!("{CACHE_NS}:thread:{thread_uid}:{pcounter}:{rgen}");
    state
        .cache
        .get_or_build(key, async {
            let post_ids = state
                .upstream
                .fetch_feed(
                    thread.projection.posts().id,
                    1,
                    thread.projection.posts().counter + 1,
                )
                .await?;
            let (mut posts, board) =
                load_posts_and_board(&state.upstream, post_ids, thread.projection.board()).await?;

            if board.projection.deleted() {
                return Err(RelayError::NotFound("board deleted".into()));
            }

            posts.sort_by_key(|p| p.projection.number());

            let text_hashes: HashSet<Address> = posts
                .iter()
                .filter(|p| !p.projection.deleted())
                .filter_map(|p| p.projection.text_hash())
                .collect();

            let mut plain_text_hashes = HashSet::new();
            if let Some(h) = thread.projection.topic_hash() {
                plain_text_hashes.insert(h);
            }
            for post in posts.iter().filter(|p| !p.projection.deleted()) {
                if let Some(h) = post.projection.name_hash() {
                    plain_text_hashes.insert(h);
                }
            }

            let media_hashes: HashSet<Address> = posts
                .iter()
                .filter(|p| !p.projection.deleted())
                .flat_map(|p| p.projection.media_hashes().iter().copied())
                .collect();

            let (text, plain_text, media_meta) = tokio::join!(
                fetch_content(&state.seaweed, ContentKind::Text, text_hashes),
                fetch_content(&state.seaweed, ContentKind::PlainText, plain_text_hashes),
                fetch_media_meta(&state.seaweed, media_hashes),
            );

            let (forum_mods, board_mods, thread_mods) = tokio::join!(
                list_mods(&state.upstream, state.forum.projection.mods().id),
                list_mods(&state.upstream, board.projection.mods().id),
                list_mods(&state.upstream, thread.projection.mods().id),
            );
            let moderators = Moderators {
                forum_admin: Some(state.forum.projection.admin()),
                forum_mods: forum_mods?,
                board_mods: board_mods?,
                thread_mods: thread_mods?,
                thread_admin: *thread.projection.admin(),
            };

            let response = ThreadView {
                thread,
                posts,
                text,
                plain_text,
                media_meta,
                moderators,
            };

            bcs::to_bytes(&response)
                .map_err(|e| RelayError::Internal(format!("bcs encode ThreadView: {e}")))
        })
        .await
}
