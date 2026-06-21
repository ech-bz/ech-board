use std::collections::{HashMap, HashSet};

use crate::app_state::AppState;
use crate::error::RelayError;
use serde::Serialize;
use sui_sdk_types::Address;

use super::fetch_text;
use super::{PostObject, ThreadObject};

#[derive(Serialize)]
pub(crate) struct ThreadView {
    pub(crate) thread: ThreadObject,
    pub(crate) posts: Vec<PostObject>,
    pub(crate) content: HashMap<Address, Vec<u8>>,
}

pub(crate) async fn fetch(state: &AppState, thread_uid: Address) -> Result<Vec<u8>, RelayError> {
    let thread = state.upstream.fetch_objects([thread_uid]).await?[0]
        .as_ref()
        .ok_or_else(|| RelayError::Internal("thread not found".into()))?
        .contents()
        .deserialize::<ThreadObject>()
        .map_err(|e| RelayError::Internal(format!("bcs decode ThreadObject: {e}")))?;

    let fields = state
        .upstream
        .list_dynamic_fields(thread.projection.posts.id)
        .await?;

    let post_ids: Vec<Address> = fields
        .iter()
        .filter_map(|(_, _, value_bytes)| {
            value_bytes
                .as_ref()
                .and_then(|v| bcs::from_bytes::<Address>(v).ok())
        })
        .collect();

    let mut posts: Vec<PostObject> = state
        .upstream
        .fetch_objects(&post_ids)
        .await?
        .into_iter()
        .flatten()
        .map(|obj| {
            obj.contents()
                .deserialize::<PostObject>()
                .map_err(|e| RelayError::Internal(format!("bcs decode PostObject: {e}")))
        })
        .collect::<Result<Vec<_>, _>>()?;

    posts.sort_by_key(|p| p.projection.number);

    let text_hashes: HashSet<Address> = posts
        .iter()
        .filter_map(|p| p.projection.text_hash)
        .collect();

    let content = fetch_text(&state.seaweed, text_hashes).await;

    let response = ThreadView {
        thread,
        posts,
        content,
    };

    bcs::to_bytes(&response)
        .map_err(|e| RelayError::Internal(format!("bcs encode ThreadView: {e}")))
}
