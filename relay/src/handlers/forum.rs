use std::collections::{HashMap, HashSet};

use super::fetch_content;
use super::{BoardObject, ForumObject, load_board, load_forum};
use crate::app_state::AppState;
use crate::error::RelayError;
use crate::types::ContentKind;
use serde::Serialize;
use sui_sdk_types::Address;

#[derive(Serialize)]
pub(crate) struct ForumView {
    pub(crate) forum: ForumObject,
    pub(crate) boards: Vec<BoardObject>,
    pub(crate) plain_text: HashMap<Address, Vec<u8>>,
}

pub(crate) async fn fetch(state: &AppState) -> Result<Vec<u8>, RelayError> {
    let forum_uid = state.forum.id;
    let forum_obj = load_forum(&state.upstream, forum_uid).await?;

    let boards_table_id = forum_obj.projection.boards.id;
    let fields = state.upstream.list_dynamic_fields(boards_table_id).await?;

    let mut child_ids = Vec::with_capacity(fields.len());
    for (_name_bytes, _child_id, value_bytes) in &fields {
        let Some(value) = value_bytes else {
            continue;
        };
        let Ok(addr) = bcs::from_bytes::<Address>(value) else {
            continue;
        };
        child_ids.push(addr);
    }

    let mut boards = Vec::with_capacity(child_ids.len());
    for id in child_ids {
        boards.push(load_board(&state.upstream, id).await?);
    }

    let mut plain_text_hashes = HashSet::new();
    for board in &boards {
        if let Some(h) = board.projection.description_hash {
            plain_text_hashes.insert(h);
        }
    }
    let plain_text = fetch_content(&state.seaweed, ContentKind::PlainText, plain_text_hashes).await;

    let response = ForumView {
        forum: forum_obj,
        boards,
        plain_text,
    };

    bcs::to_bytes(&response).map_err(|e| RelayError::Internal(format!("bcs encode ForumView: {e}")))
}
