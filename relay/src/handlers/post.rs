use std::collections::{HashMap, HashSet};

use crate::app_state::AppState;
use crate::error::RelayError;
use serde::Serialize;
use sui_sdk_types::Address;

use super::fetch_content;
use super::fetch_media_meta;
use super::{BoardObject, Moderators, PostObject, ThreadObject, list_mods, load_board, load_post, load_thread};
use crate::types::{ContentKind, MediaMeta};

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
    let post = load_post(&state.upstream, post_uid).await?;
    if post.projection.deleted {
        return Err(RelayError::NotFound("post deleted".into()));
    }
    let thread = load_thread(&state.upstream, post.projection.thread).await?;
    if thread.projection.deleted {
        return Err(RelayError::NotFound("thread deleted".into()));
    }
    let board = load_board(&state.upstream, thread.projection.board).await?;
    if board.projection.deleted {
        return Err(RelayError::NotFound("board deleted".into()));
    }

    let mut text_hashes = HashSet::new();
    if let Some(h) = post.projection.text_hash {
        text_hashes.insert(h);
    }
    let text = fetch_content(&state.seaweed, ContentKind::Text, text_hashes).await;

    let media_hashes: HashSet<Address> = post.projection.media_hashes.iter().copied().collect();
    let media_meta = fetch_media_meta(&state.seaweed, media_hashes).await;

    let moderators = Moderators {
        forum_mods: list_mods(&state.upstream, state.forum.projection.mods.id).await?,
        board_mods: list_mods(&state.upstream, board.projection.mods.id).await?,
        thread_mods: list_mods(&state.upstream, thread.projection.mods.id).await?,
        forum_admin: Some(state.forum.projection.admin),
        thread_admin: thread.projection.admin,
    };

    let response = PostView {
        post,
        thread,
        board,
        text,
        media_meta,
        moderators,
    };

    bcs::to_bytes(&response).map_err(|e| RelayError::Internal(format!("bcs encode PostView: {e}")))
}
