use std::collections::{HashMap, HashSet};

use crate::app_state::AppState;
use crate::error::RelayError;
use serde::Serialize;
use sui_sdk_types::Address;

use super::fetch_content;
use crate::types::ContentKind;
use super::{BoardObject, PostObject, ThreadObject};

const LIMIT: u64 = 20;

#[derive(Serialize)]
pub(crate) struct BoardView {
    pub(crate) board: BoardObject,
    pub(crate) threads: Vec<ThreadObject>,
    pub(crate) last_3: HashMap<Address, Vec<PostObject>>,
    pub(crate) text: HashMap<Address, Vec<u8>>,
    pub(crate) plain_text: HashMap<Address, Vec<u8>>,
    pub(crate) next_cursor: Option<u64>,
}

pub(crate) async fn fetch(
    state: &AppState,
    board_uid: Address,
    cursor: Option<u64>,
) -> Result<Vec<u8>, RelayError> {
    let board = state.upstream.fetch_objects([board_uid]).await?[0]
        .as_ref()
        .ok_or_else(|| RelayError::Internal("board not found".into()))?
        .contents()
        .deserialize::<BoardObject>()
        .map_err(|e| RelayError::Internal(format!("bcs decode BoardObject: {e}")))?;

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

    let thread_objects = state.upstream.fetch_objects(&thread_addrs).await?;

    let mut threads = Vec::with_capacity(thread_objects.len());
    let mut post_addrs_by_thread: Vec<(Address, Vec<Address>)> =
        Vec::with_capacity(thread_objects.len());

    for obj in thread_objects.iter().flatten() {
        let thread = obj
            .contents()
            .deserialize::<ThreadObject>()
            .map_err(|e| RelayError::Internal(format!("bcs decode ThreadObject: {e}")))?;
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

    let post_objects = state.upstream.fetch_objects(&all_post_ids).await?;

    let mut pi = 0;
    let mut last_3 = HashMap::with_capacity(post_addrs_by_thread.len());
    for (thread_uid, addrs) in &post_addrs_by_thread {
        let take = addrs.len();
        let mut posts = Vec::with_capacity(take);
        for obj in post_objects[pi..pi + take].iter().flatten() {
            let post = obj
                .contents()
                .deserialize::<PostObject>()
                .map_err(|e| RelayError::Internal(format!("bcs decode PostObject: {e}")))?;
            posts.push(post);
        }
        last_3.insert(*thread_uid, posts);
        pi += take;
    }

    let text_hashes: HashSet<Address> = last_3
        .values()
        .flat_map(|posts| posts.iter())
        .filter_map(|p| p.projection.text_hash)
        .collect();
    let text = fetch_content(&state.seaweed, ContentKind::Text, text_hashes).await;

    let mut plain_text_hashes = HashSet::new();
    if let Some(h) = board.projection.description_hash {
        plain_text_hashes.insert(h);
    }
    for thread in &threads {
        if let Some(h) = thread.projection.topic_hash {
            plain_text_hashes.insert(h);
        }
    }
    let plain_text = fetch_content(&state.seaweed, ContentKind::PlainText, plain_text_hashes).await;

    let next_cursor = if start > 1 { Some(start) } else { None };

    let response = BoardView {
        board,
        threads,
        last_3,
        text,
        plain_text,
        next_cursor,
    };

    bcs::to_bytes(&response).map_err(|e| RelayError::Internal(format!("bcs encode BoardView: {e}")))
}
