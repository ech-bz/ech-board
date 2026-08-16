use std::collections::{HashMap, HashSet};

use crate::app_state::AppState;
use crate::cache::CACHE_NS;
use crate::error::RelayError;
use serde::{Deserialize, Serialize};
use sui_sdk_types::{Address, TypeTag};

use super::fetch_content;
use super::fetch_media_meta;
use super::post::{PostObject, decode_post, load_posts};
use super::{DynamicFields, load_root_fields, load_roots_fields};
use super::thread::{ThreadObject, load_threads};
use super::{Moderators, list_mods};
use crate::types::{Bans, ContentKind, EntityRoot, Feed, MediaMeta, Table};

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct BoardObject {
    pub(crate) root: EntityRoot,
    pub(crate) projection: BoardProjection,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) enum BoardProjection {
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

pub(super) fn decode_board(
    root: EntityRoot,
    fields: DynamicFields,
) -> Result<BoardObject, RelayError> {
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
        _ => return Err(RelayError::Internal(format!(
            "board version {version} not supported"
        ))),
    };
    Ok(BoardObject {
        root,
        projection,
    })
}

pub(crate) async fn load_board(
    upstream: &crate::upstream::UpstreamSender,
    id: Address,
) -> Result<BoardObject, RelayError> {
    let (root, fields) = load_root_fields(upstream, id).await?;
    decode_board(root, fields)
}

pub(crate) async fn load_posts_and_board(
    upstream: &crate::upstream::UpstreamSender,
    post_ids: Vec<Address>,
    board_id: Address,
) -> Result<(Vec<PostObject>, BoardObject), RelayError> {
    let mut ids = post_ids.clone();
    ids.push(board_id);
    let (roots, mut fields) = load_roots_fields(upstream, &ids).await?;
    let mut roots = roots.into_iter();
    let board_root = roots
        .next_back()
        .ok_or_else(|| RelayError::Internal("load_posts_and_board: empty ids".to_string()))?;
    let board_fields = fields
        .remove(&board_id)
        .ok_or_else(|| RelayError::Internal("board fields missing".to_string()))?;
    let board = decode_board(board_root, board_fields)?;
    let posts = post_ids
        .into_iter()
        .zip(roots)
        .map(|(id, root)| {
            let fields = fields
                .remove(&id)
                .ok_or_else(|| RelayError::Internal(format!("post fields {id} missing")))?;
            decode_post(root, fields)
        })
        .collect::<Result<Vec<_>, RelayError>>()?;
    Ok((posts, board))
}

const PAGE_THREADS: usize = 20;
const BUMP_CHUNK: u64 = 500;

pub(crate) async fn resolve_post(
    state: &AppState,
    board_uid: Address,
    number: u64,
) -> Result<Vec<u8>, RelayError> {
    let key = format!("{CACHE_NS}:resolvepost:{board_uid}:{number}");
    if let Some(cached) = state.cache.peek(&key).await {
        return Ok(cached);
    }
    let board = load_board(&state.upstream, board_uid).await?;
    if board.projection.deleted() {
        return Err(RelayError::NotFound("board deleted".into()));
    }
    let child_id = board
        .projection
        .posts()
        .id
        .derive_dynamic_child_id(&TypeTag::U64, &number.to_le_bytes());
    let object = state
        .upstream
        .fetch_objects([child_id])
        .await?
        .into_iter()
        .next()
        .flatten()
        .ok_or_else(|| RelayError::NotFound(format!("post {number} not found")))?;
    let entry: PostEntry = object.contents().deserialize().map_err(|e| {
        RelayError::Internal(format!("post table entry decode: {e}"))
    })?;
    let bytes = entry.value.as_bytes().to_vec();
    state.cache.store(key, &bytes).await;
    Ok(bytes)
}

pub(crate) async fn resolve_thread(
    state: &AppState,
    board_uid: Address,
    number: u64,
) -> Result<Vec<u8>, RelayError> {
    let key = format!("{CACHE_NS}:resolvethread:{board_uid}:{number}");
    if let Some(cached) = state.cache.peek(&key).await {
        return Ok(cached);
    }
    let board = load_board(&state.upstream, board_uid).await?;
    if board.projection.deleted() {
        return Err(RelayError::NotFound("board deleted".into()));
    }
    let child_id = board
        .projection
        .threads()
        .id
        .derive_dynamic_child_id(&TypeTag::U64, &number.to_le_bytes());
    let object = state
        .upstream
        .fetch_objects([child_id])
        .await?
        .into_iter()
        .next()
        .flatten()
        .ok_or_else(|| RelayError::NotFound(format!("thread {number} not found")))?;
    let entry: PostEntry = object.contents().deserialize().map_err(|e| {
        RelayError::Internal(format!("thread table entry decode: {e}"))
    })?;
    let bytes = entry.value.as_bytes().to_vec();
    state.cache.store(key, &bytes).await;
    Ok(bytes)
}

#[derive(Deserialize)]
struct PostEntry {
    #[allow(dead_code)]
    id: Address,
    #[allow(dead_code)]
    name: u64,
    value: Address,
}

#[derive(Serialize)]
pub(crate) struct BoardView {
    pub(crate) board: BoardObject,
    pub(crate) threads: Vec<ThreadObject>,
    pub(crate) op_posts: HashMap<Address, PostObject>,
    pub(crate) last_3: HashMap<Address, Vec<PostObject>>,
    pub(crate) text: HashMap<Address, Vec<u8>>,
    pub(crate) plain_text: HashMap<Address, Vec<u8>>,
    pub(crate) media_meta: HashMap<Address, MediaMeta>,
    pub(crate) next_cursor: Option<u64>,
    pub(crate) moderators: Moderators,
}

pub(crate) async fn fetch(
    state: &AppState,
    board_uid: Address,
    cursor: Option<u64>,
) -> Result<Vec<u8>, RelayError> {
    let board = load_board(&state.upstream, board_uid).await?;

    if board.projection.deleted() {
        return Err(RelayError::NotFound("board deleted".into()));
    }

    let pcounter = board.projection.posts().size;
    let rgen = state
        .cache
        .gen_get(&format!("gen:board:{board_uid}"))
        .await;
    let key = format!(
        "{CACHE_NS}:board:{board_uid}:{pcounter}:{rgen}:{}",
        cursor.unwrap_or(0)
    );
    if let Some(cached) = state.cache.peek(&key).await {
        return Ok(cached);
    }

    let counter = board.projection.bumps().counter;
    let mut end = cursor.unwrap_or(counter + 1);
    if end > counter + 1 {
        end = counter + 1;
    }

    let mut seen = HashSet::new();
    let mut thread_addrs: Vec<Address> = Vec::new();
    for addr in board.projection.pinned() {
        if seen.insert(*addr) {
            thread_addrs.push(*addr);
        }
    }

    let mut i = end;
    while thread_addrs.len() < PAGE_THREADS && i > 1 {
        let chunk_start = if i > BUMP_CHUNK { i - BUMP_CHUNK } else { 1 };
        let bump_addrs = state
            .upstream
            .fetch_feed(board.projection.bumps().id, chunk_start, i)
            .await?;
        let mut stop_at = None;
        for (off, addr) in bump_addrs.into_iter().enumerate().rev() {
            if seen.insert(addr) {
                thread_addrs.push(addr);
                if thread_addrs.len() >= PAGE_THREADS {
                    stop_at = Some(chunk_start + off as u64);
                    break;
                }
            }
        }
        i = stop_at.unwrap_or(chunk_start);
    }

    let thread_map = load_threads(&state.upstream, &thread_addrs).await?;
    let mut threads = Vec::with_capacity(thread_addrs.len());
    let mut op_addrs: Vec<(Address, Address)> = Vec::with_capacity(thread_addrs.len());
    let mut reply_addrs_by_thread: Vec<(Address, Vec<Address>)> =
        Vec::with_capacity(thread_addrs.len());

    for thread_id in thread_addrs {
        let Some(thread) = thread_map.get(&thread_id) else {
            continue;
        };
        let thread_uid = thread.root.id;
        op_addrs.push((thread_uid, thread.projection.op()));
        reply_addrs_by_thread.push((thread_uid, thread.projection.last_3().to_vec()));
        threads.push(thread.clone());
    }

    let all_post_ids: Vec<_> = op_addrs
        .iter()
        .map(|(_, id)| *id)
        .chain(
            reply_addrs_by_thread
                .iter()
                .flat_map(|(_, addrs)| addrs.iter().copied()),
        )
        .collect();

    let post_map = load_posts(&state.upstream, &all_post_ids).await?;

    let mut op_posts = HashMap::with_capacity(op_addrs.len());
    for (thread_uid, id) in &op_addrs {
        let post = post_map.get(id).ok_or_else(|| {
            crate::error::RelayError::Internal(format!("post {id} not loaded"))
        })?;
        op_posts.insert(*thread_uid, post.clone());
    }

    let mut last_3 = HashMap::with_capacity(reply_addrs_by_thread.len());
    for (thread_uid, addrs) in &reply_addrs_by_thread {
        let mut posts = Vec::with_capacity(addrs.len());
        for id in addrs {
            let post = post_map.get(id).ok_or_else(|| {
                crate::error::RelayError::Internal(format!("post {id} not loaded"))
            })?;
            posts.push(post.clone());
        }
        last_3.insert(*thread_uid, posts);
    }

    let deleted_threads: HashSet<Address> =
        threads.iter().filter(|t| t.projection.deleted()).map(|t| t.root.id).collect();
    let preview_posts: Vec<&PostObject> = op_posts
        .iter()
        .filter(|(tid, _)| !deleted_threads.contains(tid))
        .map(|(_, p)| p)
        .chain(
            last_3
                .iter()
                .filter(|(tid, _)| !deleted_threads.contains(tid))
                .flat_map(|(_, posts)| posts.iter()),
        )
        .collect();
    let text_hashes: HashSet<Address> = preview_posts
        .iter()
        .filter(|p| !p.projection.deleted())
        .filter_map(|p| p.projection.text_hash())
        .collect();
    let text = fetch_content(&state.seaweed, ContentKind::Text, text_hashes).await;

    let mut plain_text_hashes = HashSet::new();
    if let Some(h) = board.projection.description_hash() {
        plain_text_hashes.insert(h);
    }
    for thread in &threads {
        if thread.projection.deleted() {
            continue;
        }
        if let Some(h) = thread.projection.topic_hash() {
            plain_text_hashes.insert(h);
        }
    }
    for post in preview_posts.iter().filter(|p| !p.projection.deleted()) {
        if let Some(h) = post.projection.name_hash() {
            plain_text_hashes.insert(h);
        }
    }
    let plain_text = fetch_content(&state.seaweed, ContentKind::PlainText, plain_text_hashes).await;

    let media_hashes: HashSet<Address> = preview_posts
        .iter()
        .filter(|p| !p.projection.deleted())
        .flat_map(|p| p.projection.media_hashes().iter().copied())
        .collect();
    let media_meta = fetch_media_meta(&state.seaweed, media_hashes).await;

    let next_cursor = if i > 1 { Some(i) } else { None };

    let moderators = Moderators {
        forum_admin: Some(state.forum.projection.admin()),
        forum_mods: list_mods(&state.upstream, state.forum.projection.mods().id).await?,
        board_mods: list_mods(&state.upstream, board.projection.mods().id).await?,
        thread_mods: Vec::new(),
        thread_admin: None,
    };

    let response = BoardView {
        board,
        threads,
        op_posts,
        last_3,
        text,
        plain_text,
        media_meta,
        next_cursor,
        moderators,
    };

    let bytes = bcs::to_bytes(&response)
        .map_err(|e| RelayError::Internal(format!("bcs encode BoardView: {e}")))?;
    state.cache.store(key, &bytes).await;
    Ok(bytes)
}
