use super::{BoardObject, ForumObject};
use crate::app_state::AppState;
use crate::error::RelayError;
use serde::Serialize;
use sui_sdk_types::Address;

#[derive(Debug, Serialize)]
pub(crate) struct BoardSlug {
    pub(crate) slug: String,
    pub(crate) id: Address,
}

#[derive(Serialize)]
pub(crate) struct ForumView {
    pub(crate) forum: ForumObject,
    pub(crate) boards: Vec<BoardSlug>,
}

pub(crate) async fn fetch(state: &AppState) -> Result<Vec<u8>, RelayError> {
    let forum_uid = state.forum.id;
    let forum_obj = state.upstream.fetch_objects([forum_uid]).await?[0]
        .as_ref()
        .ok_or_else(|| RelayError::Internal("forum not found".into()))?
        .contents()
        .deserialize::<ForumObject>()
        .map_err(|e| RelayError::Internal(format!("bcs decode ForumObject: {e}")))?;

    let boards_table_id = forum_obj.projection.boards.id;
    eprintln!("forum: listing dynamic fields of boards table {}", boards_table_id);

    let fields = state
        .upstream
        .list_dynamic_fields(boards_table_id)
        .await?;

    eprintln!("forum: list_dynamic_fields returned {} entries", fields.len());

    let mut child_ids = Vec::with_capacity(fields.len());
    for (_name_bytes, _child_id, value_bytes) in &fields {
        let Some(value) = value_bytes else {
            eprintln!("forum: skipping entry with no value");
            continue;
        };
        let Ok(addr) = bcs::from_bytes::<Address>(value) else {
            eprintln!("forum: skipping entry with invalid address");
            continue;
        };
        child_ids.push(addr);
    }

    let board_objects = state.upstream.fetch_objects(child_ids).await?;

    let mut slugs = Vec::with_capacity(board_objects.len());
    for obj in board_objects.into_iter().flatten() {
        match obj.contents().deserialize::<BoardObject>() {
            Ok(board) => {
                let slug = board.projection.slug.clone();
                let id = board.id;
                eprintln!("forum: board slug={} id={}", slug, id);
                slugs.push(BoardSlug { slug, id });
            }
            Err(e) => {
                eprintln!("forum: failed to decode BoardObject: {e}");
            }
        }
    }

    let response = ForumView {
        forum: forum_obj,
        boards: slugs,
    };

    bcs::to_bytes(&response).map_err(|e| RelayError::Internal(format!("bcs encode ForumView: {e}")))
}
