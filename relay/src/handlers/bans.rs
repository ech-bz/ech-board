use std::collections::{HashMap, HashSet};

use crate::app_state::AppState;
use crate::cache::CACHE_NS;
use crate::error::RelayError;
use crate::handlers::{Bans, DynamicFields, Registry, fetch_content};
use crate::types::ContentKind;
use serde::{Deserialize, Serialize};
use sui_sdk_types::Address;

const LIMIT: u64 = 50;

#[derive(Serialize, Clone)]
pub(crate) struct BanEntry {
    pub(crate) mask: u8,
    pub(crate) ip_hash: Address,
    pub(crate) reason_hash: Address,
    pub(crate) reason: Option<String>,
    pub(crate) expires: u64,
}

#[derive(Serialize)]
pub(crate) struct BansView {
    pub(crate) level: Address,
    pub(crate) bans: Vec<BanEntry>,
    pub(crate) next_cursor: Option<u64>,
}

#[derive(Deserialize)]
struct BanValue {
    reason_hash: Address,
    expires: u64,
}

async fn read_registry(
    upstream: &crate::upstream::UpstreamSender,
    reg: &Registry,
    mask: u8,
) -> Result<Vec<BanEntry>, RelayError> {
    let mut entries: Vec<(u64, BanValue)> = Vec::new();
    for (name, _, value) in upstream.list_dynamic_fields(reg.entries.id).await? {
        let Some(value) = value else { continue };
        let Ok(id) = bcs::from_bytes::<u64>(&name) else { continue };
        let Ok(v) = bcs::from_bytes::<BanValue>(&value) else { continue };
        entries.push((id, v));
    }
    entries.sort_by_key(|(id, _)| *id);

    let mut hashes: HashMap<u64, Vec<Address>> = HashMap::new();
    for (name, _, value) in upstream.list_dynamic_fields(reg.identities.id).await? {
        let Some(value) = value else { continue };
        let Ok(id) = bcs::from_bytes::<u64>(&name) else { continue };
        let Ok(v) = bcs::from_bytes::<Vec<Address>>(&value) else { continue };
        hashes.insert(id, v);
    }

    let mut out = Vec::with_capacity(entries.len());
    for (id, value) in entries {
        let hs = hashes.get(&id).cloned().unwrap_or_default();
        for h in hs {
            out.push(BanEntry {
                mask,
                ip_hash: h,
                reason_hash: value.reason_hash,
                reason: None,
                expires: value.expires,
            });
        }
    }
    Ok(out)
}

pub(crate) async fn fetch(
    state: &AppState,
    uid: Address,
    cursor: Option<u64>,
) -> Result<Vec<u8>, RelayError> {
    let key = format!("{CACHE_NS}:bans:{uid}:{}", cursor.unwrap_or(0));
    state
        .cache
        .get_or_build(key, async {
            let fields = DynamicFields::load(&state.upstream, uid).await?;
            let bans: Bans = fields.get(b"bans")?;

            let mut all: Vec<BanEntry> = Vec::new();
            for (reg, mask) in [
                (&bans.ip32, 32u8),
                (&bans.ip24, 24u8),
                (&bans.ip20, 20u8),
                (&bans.ip16, 16u8),
            ] {
                all.extend(read_registry(&state.upstream, reg, mask).await?);
            }

            let reason_hashes: HashSet<Address> = all.iter().map(|b| b.reason_hash).collect();
            let plain =
                fetch_content(&state.seaweed, ContentKind::PlainText, reason_hashes).await;

            let offset = cursor.unwrap_or(0);
            let start = offset as usize;
            let page: Vec<BanEntry> = all
                .iter()
                .skip(start)
                .take(LIMIT as usize)
                .cloned()
                .map(|mut b| {
                    b.reason = plain
                        .get(&b.reason_hash)
                        .map(|bytes| String::from_utf8_lossy(bytes).into_owned());
                    b
                })
                .collect();
            let next_cursor = if offset + (page.len() as u64) < (all.len() as u64) {
                Some(offset + page.len() as u64)
            } else {
                None
            };

            let response = BansView {
                level: bans.level,
                bans: page,
                next_cursor,
            };

            bcs::to_bytes(&response)
                .map_err(|e| RelayError::Internal(format!("bcs encode BansView: {e}")))
        })
        .await
}
