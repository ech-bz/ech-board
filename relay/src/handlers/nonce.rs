use crate::app_state::AppState;
use crate::error::RelayError;
use crate::handlers::Shard;
use blake2::Digest;
use blake2::digest::consts::U32;
use serde::{Deserialize, Serialize};
use sui_sdk_types::Address;
use sui_sdk_types::TypeTag;

type Blake2b = blake2::Blake2b<U32>;

const SHARD_COUNT: u64 = 512;

#[derive(Deserialize)]
struct FieldValue {
    #[allow(dead_code)]
    id: Address,
    #[allow(dead_code)]
    name: Address,
    value: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct NonceInfo {
    pub(crate) nonce: u64,
}

pub(crate) async fn fetch(state: &AppState, sender: &Address) -> Result<Vec<u8>, RelayError> {
    let mut buf = vec![0u8];
    buf.extend_from_slice(sender.as_ref() as &[u8]);
    let addr = Address::new(Blake2b::digest(&buf).into());
    let hash = Blake2b::digest(
        &bcs::to_bytes(&addr)
            .map_err(|e| RelayError::Internal(format!("bcs encode sender: {e}")))?,
    );
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&hash[24..]);
    let shard_index = u64::from_be_bytes(buf) % SHARD_COUNT;

    let shard_id = state
        .forum
        .projection
        .nonce_shards
        .derive_object_id(&TypeTag::U64, &shard_index.to_le_bytes());

    let shard = state.upstream.fetch_objects([shard_id]).await?[0]
        .as_ref()
        .ok_or_else(|| RelayError::Internal("nonce shard not found".into()))?
        .contents()
        .deserialize::<Shard>()
        .map_err(|e| RelayError::Internal(format!("bcs decode Shard: {e}")))?;

    let address_tag: TypeTag = "address"
        .parse()
        .map_err(|e| RelayError::Internal(format!("invalid address type tag: {e}")))?;

    let field_id = shard
        .counters
        .id
        .derive_dynamic_child_id(&address_tag, hash.as_slice());

    let nonce = match &state.upstream.fetch_objects([field_id]).await?[0] {
        None => 0,
        Some(obj) => obj
            .contents()
            .deserialize::<FieldValue>()
            .map(|f| f.value)
            .unwrap_or(0),
    };

    let response = NonceInfo { nonce };

    bcs::to_bytes(&response).map_err(|e| RelayError::Internal(format!("bcs encode NonceInfo: {e}")))
}
