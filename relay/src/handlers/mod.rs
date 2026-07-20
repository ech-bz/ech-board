pub(crate) mod admin;
pub(crate) mod board;
pub(crate) mod content;
pub(crate) mod feed;
pub(crate) mod forum;
pub(crate) mod nonce;
pub(crate) mod send;
pub(crate) mod thread;
pub(crate) mod uid;

use crate::seaweed::SeaweedClient;
use crate::types::ContentKind;
use actix_web::HttpRequest;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use sui_sdk_types::Address;

#[derive(Deserialize)]
pub(crate) struct Pagination {
    pub(crate) cursor: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct Table {
    pub(super) id: Address,
    pub(super) size: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct Feed {
    pub(super) id: Address,
    pub(super) counter: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct ForumObject {
    pub(super) id: Address,
    pub(super) feed: Feed,
    pub(super) projection: ForumProjection,
    pub(super) genesis: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct ForumProjection {
    pub(super) nonce_shards: Address,
    pub(super) admin: Address,
    pub(super) mods: Table,
    pub(super) bans: Table,
    pub(super) boards: Table,
    pub(super) timestamp_precision_ms: u64,
}

#[derive(Serialize, Deserialize)]
pub(super) struct BoardObject {
    pub(super) id: Address,
    pub(super) feed: Feed,
    pub(super) projection: BoardProjection,
    pub(super) genesis: bool,
}

#[derive(Serialize, Deserialize)]
pub(super) struct BoardProjection {
    pub(super) slug: String,
    pub(super) description: String,
    pub(super) max_media: u64,
    pub(super) bump_limit: u64,
    pub(super) closed: bool,
    pub(super) deleted: bool,
    pub(super) mods: Table,
    pub(super) bans: Table,
    pub(super) threads: Table,
    pub(super) posts: Table,
    pub(super) bumps: Feed,
}

#[derive(Serialize, Deserialize)]
pub(super) struct ThreadObject {
    pub(super) id: Address,
    pub(super) feed: Feed,
    pub(super) projection: ThreadProjection,
    pub(super) genesis: bool,
}

#[derive(Serialize, Deserialize)]
pub(super) struct ThreadProjection {
    pub(super) board_slug: String,
    pub(super) number: u64,
    pub(super) op: Address,
    pub(super) subject: String,
    pub(super) closed: bool,
    pub(super) deleted: bool,
    pub(super) pinned: bool,
    pub(super) admin: Option<Address>,
    pub(super) mods: Table,
    pub(super) bans: Table,
    pub(super) posts: Table,
    pub(super) last_3: Vec<Address>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct PostObject {
    pub(super) id: Address,
    pub(super) feed: Feed,
    pub(super) projection: PostProjection,
    pub(super) genesis: bool,
}

#[derive(Serialize, Deserialize)]
pub(super) struct PostProjection {
    pub(super) board_slug: String,
    pub(super) thread: u64,
    pub(super) number: u64,
    pub(super) author: Address,
    pub(super) tweak: Address,
    pub(super) deleted: bool,
    pub(super) text_hash: Option<Address>,
    pub(super) media_hashes: Vec<Address>,
    pub(super) created_at_ms: u64,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Shard {
    pub(super) id: Address,
    pub(super) shards: u64,
    pub(super) index: u64,
    pub(super) counters: Table,
}

pub(super) async fn fetch_text(
    seaweed: &SeaweedClient,
    hashes: HashSet<Address>,
) -> HashMap<Address, Vec<u8>> {
    futures::stream::iter(hashes.into_iter().map(|addr| async move {
        match seaweed.get(ContentKind::Text, &addr).await {
            Ok(Some(data)) => Some((addr, data)),
            _ => None,
        }
    }))
    .buffer_unordered(32)
    .collect::<Vec<_>>()
    .await
    .into_iter()
    .flatten()
    .collect()
}

pub(super) fn client_ip(req: &HttpRequest) -> Option<String> {
    req.connection_info().realip_remote_addr().map(|s| s.to_string())
}