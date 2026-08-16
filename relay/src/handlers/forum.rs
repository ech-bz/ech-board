use std::collections::{HashMap, HashSet};

use super::fetch_content;
use super::load_root_fields;
use super::board::{BoardObject, load_board};
use super::{Moderators, list_mods};
use crate::app_state::AppState;
use crate::cache::CACHE_NS;
use crate::error::RelayError;
use crate::types::{Bans, ContentKind, EntityRoot, Table};
use serde::{Deserialize, Serialize};
use sui_sdk_types::Address;

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct ForumObject {
    pub(crate) root: EntityRoot,
    pub(crate) projection: ForumProjection,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) enum ForumProjection {
    V1 {
        nonce_shards: Address,
        admin: Address,
        mods: Table,
        bans: Bans,
        boards: Table,
        timestamp_precision_ms: u64,
    },
}

impl ForumProjection {
    pub fn nonce_shards(&self) -> Address {
        match self {
            ForumProjection::V1 { nonce_shards, .. } => *nonce_shards,
        }
    }

    pub fn admin(&self) -> Address {
        match self {
            ForumProjection::V1 { admin, .. } => *admin,
        }
    }

    pub fn mods(&self) -> &Table {
        match self {
            ForumProjection::V1 { mods, .. } => mods,
        }
    }

    pub fn bans(&self) -> &Bans {
        match self {
            ForumProjection::V1 { bans, .. } => bans,
        }
    }

    pub fn boards(&self) -> &Table {
        match self {
            ForumProjection::V1 { boards, .. } => boards,
        }
    }

    #[allow(dead_code)]
    pub fn timestamp_precision_ms(&self) -> u64 {
        match self {
            ForumProjection::V1 { timestamp_precision_ms, .. } => *timestamp_precision_ms,
        }
    }
}

pub(crate) async fn load_forum(
    upstream: &crate::upstream::UpstreamSender,
    id: Address,
) -> Result<ForumObject, RelayError> {
    let (root, fields) = load_root_fields(upstream, id).await?;
    let version = root.entity.version;
    let projection = match version {
        1 => ForumProjection::V1 {
            nonce_shards: fields.get(b"nonce_shards")?,
            admin: fields.get(b"admin")?,
            mods: fields.get(b"moderators")?,
            bans: fields.get(b"bans")?,
            boards: fields.get(b"boards")?,
            timestamp_precision_ms: fields.get(b"timestamp_precision")?,
        },
        _ => return Err(RelayError::Internal(format!(
            "forum version {version} not supported"
        ))),
    };
    Ok(ForumObject {
        root,
        projection,
    })
}

#[derive(Serialize)]
pub(crate) struct ForumView {
    pub(crate) forum: ForumObject,
    pub(crate) boards: Vec<BoardObject>,
    pub(crate) plain_text: HashMap<Address, Vec<u8>>,
    pub(crate) moderators: Moderators,
}

pub(crate) async fn fetch(state: &AppState) -> Result<Vec<u8>, RelayError> {
    let forum_uid = state.forum.root.id;
    state
        .cache
        .get_or_build(format!("{CACHE_NS}:forum"), async {
            let forum_obj = load_forum(&state.upstream, forum_uid).await?;

            let boards_table_id = forum_obj.projection.boards().id;
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
                if let Some(h) = board.projection.description_hash() && !board.projection.deleted()
                {
                    plain_text_hashes.insert(h);
                }
            }
            let plain_text =
                fetch_content(&state.seaweed, ContentKind::PlainText, plain_text_hashes).await;

            let moderators = Moderators {
                forum_admin: Some(forum_obj.projection.admin()),
                forum_mods: list_mods(&state.upstream, forum_obj.projection.mods().id).await?,
                board_mods: Vec::new(),
                thread_mods: Vec::new(),
                thread_admin: None,
            };

            let response = ForumView {
                forum: forum_obj,
                boards,
                plain_text,
                moderators,
            };

            bcs::to_bytes(&response)
                .map_err(|e| RelayError::Internal(format!("bcs encode ForumView: {e}")))
        })
        .await
}
