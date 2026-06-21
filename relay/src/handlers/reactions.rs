use crate::app_state::AppState;
use crate::cache::CACHE_NS;
use crate::error::RelayError;
use crate::handlers::DynamicFields;
use crate::types::{Registry, Sender};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use sui_sdk_types::Address;

#[derive(Deserialize)]
#[allow(dead_code)]
struct UserEntry2 {
    sender: Sender,
    options: Vec<Address>,
}

#[derive(Serialize, Deserialize)]
struct PostReactionsView {
    reaction: Option<Address>,
}

async fn find_reaction(
    state: &AppState,
    post_uid: Address,
    pk: Address,
) -> Result<Option<Address>, RelayError> {
    let fields = DynamicFields::load(&state.upstream, post_uid).await?;
    let Some(reacted) = fields.get::<Registry>(b"reacted").ok() else {
        return Ok(None);
    };

    let pk_bytes = pk.as_bytes().to_vec();
    let mut entry_id: Option<u64> = None;
    for (name, _, value) in state.upstream.list_dynamic_fields(reacted.index.id).await? {
        if name == pk_bytes {
            if let Some(v) = value {
                entry_id = bcs::from_bytes::<u64>(&v).ok();
            }
            break;
        }
    }

    let Some(id) = entry_id else { return Ok(None) };

    let key = bcs::to_bytes(&id)
        .map_err(|e| RelayError::Internal(format!("bcs encode entry id: {e}")))?;
    let mut found: Option<Address> = None;
    for (name, _, value) in state.upstream.list_dynamic_fields(reacted.entries.id).await? {
        if name == key {
            if let Some(v) = value {
                if let Ok(entry) = bcs::from_bytes::<UserEntry2>(&v) {
                    found = entry.options.into_iter().next();
                }
            }
            break;
        }
    }
    Ok(found)
}

async fn cached_find_reaction(
    state: &AppState,
    post_uid: Address,
    pk: Address,
) -> Result<Option<Address>, RelayError> {
    let key = format!("{CACHE_NS}:reactions:{post_uid}:{pk}");
    if let Some(cached) = state.cache.peek(&key).await {
        if let Ok(view) = bcs::from_bytes::<PostReactionsView>(&cached) {
            return Ok(view.reaction);
        }
    }
    let reaction = find_reaction(state, post_uid, pk).await?;
    let bytes = bcs::to_bytes(&PostReactionsView { reaction })
        .map_err(|e| RelayError::Internal(format!("bcs encode PostReactionsView: {e}")))?;
    state.cache.store(key, &bytes).await;
    Ok(reaction)
}

pub(crate) async fn fetch(
    state: &AppState,
    queries: Vec<(Address, Address)>,
) -> Result<Vec<u8>, RelayError> {
    let results = join_all(
        queries
            .iter()
            .map(|(pid, pk)| cached_find_reaction(state, *pid, *pk)),
    )
    .await;

    let mut out = Vec::new();
    for ((pid, _), res) in queries.iter().zip(results.into_iter()) {
        if let Some(r) = res? {
            out.push((*pid, r));
        }
    }

    bcs::to_bytes(&out)
        .map_err(|e| RelayError::Internal(format!("bcs encode thread reactions: {e}")))
}
