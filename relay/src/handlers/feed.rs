use crate::app_state::AppState;
use crate::error::RelayError;
use serde::{Deserialize, Serialize};
use sui_sdk_types::Address;

const LIMIT: u64 = 20;

#[derive(Serialize)]
pub(crate) struct FeedView {
    pub(crate) items: Vec<Vec<u8>>,
    pub(crate) next_cursor: Option<u64>,
}

#[derive(Deserialize)]
pub(crate) struct FeedQuery {
    pub(crate) cursor: Option<u64>,
    pub(crate) counter: u64,
}

pub(crate) async fn fetch(
    state: &AppState,
    feed_id: Address,
    query: FeedQuery,
) -> Result<Vec<u8>, RelayError> {
    let end = query.cursor.unwrap_or(query.counter + 1);
    let start = if end > LIMIT { end - LIMIT } else { 1 };

    let mut items = state
        .upstream
        .fetch_feed_raw(feed_id, start, end)
        .await?;

    items.reverse();

    let next_cursor = if start > 1 { Some(start) } else { None };

    let response = FeedView { items, next_cursor };

    bcs::to_bytes(&response).map_err(|e| RelayError::Internal(format!("bcs encode FeedPage: {e}")))
}
