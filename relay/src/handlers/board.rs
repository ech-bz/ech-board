use std::collections::{HashMap, HashSet};

use crate::app_state::AppState;
use crate::error::RelayError;
use serde::{Deserialize, Serialize};
use sui_sdk_types::{Address, TypeTag};

use super::fetch_content;
use super::fetch_media_meta;
use super::{BoardObject, Moderators, PostObject, ThreadObject, list_mods, load_board, load_posts, load_threads};
use crate::types::{ContentKind, MediaMeta};

const PAGE_THREADS: usize = 20;
const BUMP_CHUNK: u64 = 500;

pub(crate) async fn resolve_post(
    state: &AppState,
    board_uid: Address,
    number: u64,
) -> Result<Vec<u8>, RelayError> {
    let key = format!("v:resolvepost:{board_uid}:{number}");
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
    let key = format!("v:resolvethread:{board_uid}:{number}");
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
        "v:board:{board_uid}:{pcounter}:{rgen}:{}",
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
    let mut post_addrs_by_thread: Vec<(Address, Vec<Address>)> =
        Vec::with_capacity(thread_addrs.len());

    for thread_id in thread_addrs {
        let thread = thread_map.get(&thread_id).ok_or_else(|| {
            crate::error::RelayError::Internal(format!("thread {thread_id} not loaded"))
        })?;
        let thread_uid = thread.id;
        let mut post_addrs = vec![thread.projection.op()];
        post_addrs.extend_from_slice(thread.projection.last_3());
        post_addrs_by_thread.push((thread_uid, post_addrs));
        threads.push(thread.clone());
    }

    let all_post_ids: Vec<_> = post_addrs_by_thread
        .iter()
        .flat_map(|(_, addrs)| addrs.iter().copied())
        .collect();

    let post_map = load_posts(&state.upstream, &all_post_ids).await?;

    let mut last_3 = HashMap::with_capacity(post_addrs_by_thread.len());
    for (thread_uid, addrs) in &post_addrs_by_thread {
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
        threads.iter().filter(|t| t.projection.deleted()).map(|t| t.id).collect();
    let text_hashes: HashSet<Address> = last_3
        .iter()
        .filter(|(tid, _)| !deleted_threads.contains(tid))
        .flat_map(|(_, posts)| posts.iter())
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
    for (tid, posts) in &last_3 {
        if deleted_threads.contains(tid) {
            continue;
        }
        for post in posts.iter().filter(|p| !p.projection.deleted()) {
            if let Some(h) = post.projection.name_hash() {
                plain_text_hashes.insert(h);
            }
        }
    }
    let plain_text = fetch_content(&state.seaweed, ContentKind::PlainText, plain_text_hashes).await;

    let media_hashes: HashSet<Address> = last_3
        .iter()
        .filter(|(tid, _)| !deleted_threads.contains(tid))
        .flat_map(|(_, posts)| posts.iter())
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
