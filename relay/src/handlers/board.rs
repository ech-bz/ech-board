use std::collections::{HashMap, HashSet};

use crate::app_state::AppState;
use crate::error::RelayError;
use serde::{Deserialize, Serialize};
use sui_sdk_types::{Address, TypeTag};

use super::fetch_content;
use super::{BoardObject, Moderators, PostObject, ThreadObject, list_mods, load_board, load_post, load_thread};
use crate::types::ContentKind;

const LIMIT: u64 = 20;

pub(crate) async fn resolve_post(
    state: &AppState,
    board_uid: Address,
    number: u64,
) -> Result<Vec<u8>, RelayError> {
    let board = load_board(&state.upstream, board_uid).await?;
    if board.projection.deleted {
        return Err(RelayError::NotFound("board deleted".into()));
    }
    let child_id = board
        .projection
        .posts
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
    Ok(entry.value.as_bytes().to_vec())
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
    pub(crate) next_cursor: Option<u64>,
    pub(crate) moderators: Moderators,
}

pub(crate) async fn fetch(
    state: &AppState,
    board_uid: Address,
    cursor: Option<u64>,
) -> Result<Vec<u8>, RelayError> {
    let board = load_board(&state.upstream, board_uid).await?;

    if board.projection.deleted {
        return Err(RelayError::NotFound("board deleted".into()));
    }

    let end = cursor.unwrap_or(board.projection.bumps.counter + 1);
    let start = if end > LIMIT { end - LIMIT } else { 1 };

    let bump_addrs = state
        .upstream
        .fetch_feed(board.projection.bumps.id, start, end)
        .await?;

    let mut seen = HashSet::new();
    let mut thread_addrs: Vec<Address> = Vec::new();
    for addr in bump_addrs.into_iter().rev() {
        if seen.insert(addr) {
            thread_addrs.push(addr);
        }
    }

    let mut threads = Vec::with_capacity(thread_addrs.len());
    let mut post_addrs_by_thread: Vec<(Address, Vec<Address>)> =
        Vec::with_capacity(thread_addrs.len());

    for thread_id in thread_addrs {
        let thread = load_thread(&state.upstream, thread_id).await?;
        let thread_uid = thread.id;
        let mut post_addrs = vec![thread.projection.op];
        post_addrs.extend_from_slice(&thread.projection.last_3);
        post_addrs_by_thread.push((thread_uid, post_addrs));
        threads.push(thread);
    }

    let all_post_ids: Vec<_> = post_addrs_by_thread
        .iter()
        .flat_map(|(_, addrs)| addrs.iter().copied())
        .collect();

    let mut post_objects = Vec::with_capacity(all_post_ids.len());
    for id in all_post_ids {
        post_objects.push(load_post(&state.upstream, id).await.ok());
    }

    let mut pi = 0;
    let mut last_3 = HashMap::with_capacity(post_addrs_by_thread.len());
    for (thread_uid, addrs) in &post_addrs_by_thread {
        let take = addrs.len();
        let mut posts = Vec::with_capacity(take);
        for post in post_objects[pi..pi + take].iter().flatten() {
            posts.push(post.clone());
        }
        last_3.insert(*thread_uid, posts);
        pi += take;
    }

    let deleted_threads: HashSet<Address> =
        threads.iter().filter(|t| t.projection.deleted).map(|t| t.id).collect();
    let text_hashes: HashSet<Address> = last_3
        .iter()
        .filter(|(tid, _)| !deleted_threads.contains(tid))
        .flat_map(|(_, posts)| posts.iter())
        .filter(|p| !p.projection.deleted)
        .filter_map(|p| p.projection.text_hash)
        .collect();
    let text = fetch_content(&state.seaweed, ContentKind::Text, text_hashes).await;

    let mut plain_text_hashes = HashSet::new();
    if let Some(h) = board.projection.description_hash {
        plain_text_hashes.insert(h);
    }
    for thread in &threads {
        if thread.projection.deleted {
            continue;
        }
        if let Some(h) = thread.projection.topic_hash {
            plain_text_hashes.insert(h);
        }
    }
    for (tid, posts) in &last_3 {
        if deleted_threads.contains(tid) {
            continue;
        }
        for post in posts.iter().filter(|p| !p.projection.deleted) {
            if let Some(h) = post.projection.name_hash {
                plain_text_hashes.insert(h);
            }
        }
    }
    let plain_text = fetch_content(&state.seaweed, ContentKind::PlainText, plain_text_hashes).await;

    let next_cursor = if start > 1 { Some(start) } else { None };

    let moderators = Moderators {
        forum_mods: list_mods(&state.upstream, state.forum.projection.mods.id).await?,
        board_mods: list_mods(&state.upstream, board.projection.mods.id).await?,
        thread_mods: Vec::new(),
        forum_admin: Some(state.forum.projection.admin),
        thread_admin: None,
    };

    let response = BoardView {
        board,
        threads,
        last_3,
        text,
        plain_text,
        next_cursor,
        moderators,
    };

    bcs::to_bytes(&response).map_err(|e| RelayError::Internal(format!("bcs encode BoardView: {e}")))
}
