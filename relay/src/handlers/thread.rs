use std::collections::{HashMap, HashSet};

use crate::app_state::AppState;
use crate::error::RelayError;
use serde::Serialize;
use sui_sdk_types::Address;

use super::fetch_content;
use crate::types::ContentKind;
use super::{PostObject, ThreadObject, load_post, load_thread};

#[derive(Serialize)]
pub(crate) struct ThreadView {
    pub(crate) thread: ThreadObject,
    pub(crate) posts: Vec<PostObject>,
    pub(crate) text: HashMap<Address, Vec<u8>>,
    pub(crate) plain_text: HashMap<Address, Vec<u8>>,
}

pub(crate) async fn fetch(state: &AppState, thread_uid: Address) -> Result<Vec<u8>, RelayError> {
    let thread = load_thread(&state.upstream, thread_uid).await?;
    let post_ids = state
        .upstream
        .fetch_feed(
            thread.projection.posts.id,
            1,
            thread.projection.posts.counter + 1,
        )
        .await?;
    let mut posts: Vec<PostObject> = Vec::with_capacity(post_ids.len());
    for id in post_ids {
        posts.push(load_post(&state.upstream, id).await?);
    }

    posts.sort_by_key(|p| p.projection.number);

    let text_hashes: HashSet<Address> = posts
        .iter()
        .filter_map(|p| p.projection.text_hash)
        .collect();
    let text = fetch_content(&state.seaweed, ContentKind::Text, text_hashes).await;

    let mut plain_text_hashes = HashSet::new();
    if let Some(h) = thread.projection.topic_hash {
        plain_text_hashes.insert(h);
    }
    let plain_text = fetch_content(&state.seaweed, ContentKind::PlainText, plain_text_hashes).await;

    let response = ThreadView {
        thread,
        posts,
        text,
        plain_text,
    };

    bcs::to_bytes(&response)
        .map_err(|e| RelayError::Internal(format!("bcs encode ThreadView: {e}")))
}
