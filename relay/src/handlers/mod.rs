pub(crate) mod admin;
pub(crate) mod bans;
pub(crate) mod board;
pub(crate) mod content;
pub(crate) mod decrypt;
pub(crate) mod feed;
pub(crate) mod forum;
pub(crate) mod invalidate;
pub(crate) mod nonce;
pub(crate) mod post;
pub(crate) mod reactions;
pub(crate) mod send;
pub(crate) mod thread;

use crate::seaweed::SeaweedClient;
use crate::types::{ContentKind, EntityRoot, MediaMeta, Table};
use actix_web::HttpRequest;
use futures::{StreamExt, TryStreamExt};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use sui_sdk_types::Address;

#[derive(Deserialize)]
pub(crate) struct Pagination {
    pub(crate) cursor: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct Moderators {
    pub(super) forum_admin: Option<Address>,
    pub(super) forum_mods: Vec<Address>,
    pub(super) board_mods: Vec<Address>,
    pub(super) thread_mods: Vec<Address>,
    pub(super) thread_admin: Option<Address>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Shard {
    pub(super) id: Address,
    pub(super) shards: u64,
    pub(super) index: u64,
    pub(super) counters: Table,
}

pub(super) struct DynamicFields {
    values: HashMap<Vec<u8>, Vec<u8>>,
}

impl DynamicFields {
    pub(super) async fn load(
        upstream: &crate::upstream::UpstreamSender,
        parent: Address,
    ) -> Result<Self, crate::error::RelayError> {
        let mut values = HashMap::new();
        for (encoded_name, _, value) in upstream.list_dynamic_fields(parent).await? {
            let name = bcs::from_bytes::<Vec<u8>>(&encoded_name).unwrap_or(encoded_name);
            if let Some(value) = value {
                values.insert(name, value);
            }
        }
        Ok(Self { values })
    }

    pub(super) fn get<T: DeserializeOwned>(
        &self,
        name: &[u8],
    ) -> Result<T, crate::error::RelayError> {
        let value = self.values.get(name).ok_or_else(|| {
            crate::error::RelayError::Internal(format!(
                "dynamic field '{}' not found",
                String::from_utf8_lossy(name),
            ))
        })?;
        bcs::from_bytes(value).map_err(|e| {
            crate::error::RelayError::Internal(format!(
                "dynamic field '{}' decode: {e}",
                String::from_utf8_lossy(name),
            ))
        })
    }
}

pub(super) async fn load_roots_fields(
    upstream: &crate::upstream::UpstreamSender,
    ids: &[Address],
) -> Result<(Vec<EntityRoot>, HashMap<Address, DynamicFields>), crate::error::RelayError> {
    let (objects, fields) = tokio::join!(
        upstream.fetch_objects(ids),
        futures::stream::iter(ids.iter().copied().map(|id| async move {
            let fields = DynamicFields::load(upstream, id).await?;
            Ok::<_, crate::error::RelayError>((id, fields))
        }))
        .buffer_unordered(64)
        .try_collect::<Vec<_>>(),
    );
    let roots = ids
        .iter()
        .zip(objects?.into_iter())
        .map(|(id, object)| {
            object
                .as_ref()
                .ok_or_else(|| {
                    crate::error::RelayError::Internal(format!("object {id} not found"))
                })?
                .contents()
                .deserialize::<EntityRoot>()
                .map_err(|e| {
                    crate::error::RelayError::Internal(format!("entity root {id} decode: {e}"))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((roots, fields?.into_iter().collect()))
}

pub(super) async fn load_root_fields(
    upstream: &crate::upstream::UpstreamSender,
    id: Address,
) -> Result<(EntityRoot, DynamicFields), crate::error::RelayError> {
    let (mut roots, mut fields) = load_roots_fields(upstream, &[id]).await?;
    let root = roots
        .pop()
        .ok_or_else(|| crate::error::RelayError::Internal(format!("object {id} not found")))?;
    let fields = fields
        .remove(&id)
        .ok_or_else(|| crate::error::RelayError::Internal(format!("fields {id} missing")))?;
    Ok((root, fields))
}

pub(super) async fn list_mods(
    upstream: &crate::upstream::UpstreamSender,
    mods_table_id: Address,
) -> Result<Vec<Address>, crate::error::RelayError> {
    let fields = upstream.list_dynamic_fields(mods_table_id).await?;
    let mut mods = Vec::with_capacity(fields.len());
    for (name_bytes, _, value) in fields {
        if value.is_none() {
            continue;
        }
        if let Ok(addr) = bcs::from_bytes::<Address>(&name_bytes) {
            mods.push(addr);
        }
    }
    Ok(mods)
}

pub(super) async fn fetch_content(
    seaweed: &SeaweedClient,
    kind: ContentKind,
    hashes: HashSet<Address>,
) -> HashMap<Address, Vec<u8>> {
    futures::stream::iter(hashes.into_iter().map(|addr| async move {
        for attempt in 0..3 {
            match seaweed.get(kind, &addr).await {
                Ok(Some(data)) => return Some((addr, data)),
                Ok(None) => return None,
                Err(_) if attempt < 2 => {
                    tokio::time::sleep(Duration::from_millis(50 * (attempt + 1))).await;
                }
                Err(_) => return None,
            }
        }
        None
    }))
    .buffer_unordered(32)
    .collect::<Vec<_>>()
    .await
    .into_iter()
    .flatten()
    .collect()
}

pub(super) async fn fetch_media_meta(
    seaweed: &SeaweedClient,
    hashes: HashSet<Address>,
) -> HashMap<Address, MediaMeta> {
    fetch_content(seaweed, ContentKind::MediaMeta, hashes)
        .await
        .into_iter()
        .filter_map(|(h, data)| bcs::from_bytes::<MediaMeta>(&data).ok().map(|m| (h, m)))
        .collect()
}

pub(super) fn client_ip(req: &HttpRequest) -> Option<String> {
    req.connection_info()
        .realip_remote_addr()
        .map(|s| s.to_string())
}
