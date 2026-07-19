use crate::app_state::AppState;
use crate::error::RelayError;
use actix_web::{HttpResponse, web};
use sui_sdk_types::Address;

use super::nonce::NonceInfo;
use super::{nonce, send};

async fn build_intent(
    state: &AppState,
    opcode: u8,
    moderator: Address,
) -> Result<(Vec<u8>, Vec<u8>), RelayError> {
    let sponsor_addr = state.sponsor.sponsor_address();
    let nonce_bytes = nonce::fetch(state, &sponsor_addr).await?;
    let nonce: NonceInfo = bcs::from_bytes(&nonce_bytes)
        .map_err(|e| RelayError::SponsorBuild(format!("nonce decode: {e}")))?;

    let nonce_shard_id = state.forum.projection.nonce_shards;
    let forum_id = state.forum.id;

    let mut buf = Vec::new();
    bcs::serialize_into(&mut buf, &"forum".as_bytes().to_vec()).unwrap();
    bcs::serialize_into(&mut buf, &"apply_forum_intent".as_bytes().to_vec()).unwrap();
    bcs::serialize_into(&mut buf, &nonce.nonce).unwrap();

    let objects = [(nonce_shard_id, true), (forum_id, true)];
    let obj_count: u64 = objects.len() as u64;
    bcs::serialize_into(&mut buf, &obj_count).unwrap();
    for (id, mutable) in &objects {
        bcs::serialize_into(&mut buf, id).unwrap();
        bcs::serialize_into(&mut buf, mutable).unwrap();
    }

    let mut payload = Vec::new();
    bcs::serialize_into(&mut payload, &opcode).unwrap();
    bcs::serialize_into(&mut payload, &moderator).unwrap();
    bcs::serialize_into(&mut buf, &payload).unwrap();

    bcs::serialize_into(&mut buf, &sponsor_addr).unwrap();
    let tweak = Address::ZERO;
    bcs::serialize_into(&mut buf, &tweak).unwrap();

    let sig = state.sponsor.sign_blake2b(&buf);
    Ok((buf, sig))
}

async fn moderator_action(
    state: web::Data<AppState>,
    moderator: Address,
    opcode: u8,
) -> Result<HttpResponse, RelayError> {
    let (intent_bytes, sig_bytes) = build_intent(&state, opcode, moderator).await?;
    let result = send::handle_send(&state, intent_bytes, sig_bytes, None, vec![]).await?;
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(result))
}

pub(crate) async fn add_moderator(
    state: web::Data<AppState>,
    moderator: Address,
) -> Result<HttpResponse, RelayError> {
    moderator_action(state, moderator, 0).await
}

pub(crate) async fn del_moderator(
    state: web::Data<AppState>,
    moderator: Address,
) -> Result<HttpResponse, RelayError> {
    moderator_action(state, moderator, 1).await
}
