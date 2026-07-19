use crate::app_state::AppState;
use crate::error::RelayError;
use crate::types::{Intent, IntentObject};
use actix_web::{HttpResponse, web};
use sui_sdk_types::Address;

use super::nonce::NonceInfo;
use super::{nonce, send};

async fn build_intent(
    state: &AppState,
    opcode: u8,
    moderator: Address,
) -> Result<Intent, RelayError> {
    let sponsor_addr = state.sponsor.sponsor_address();
    let nonce_bytes = nonce::fetch(state, &sponsor_addr).await?;
    let nonce: NonceInfo = bcs::from_bytes(&nonce_bytes)
        .map_err(|e| RelayError::SponsorBuild(format!("nonce decode: {e}")))?;

    let nonce_shard_id = state.forum.projection.nonce_shards;
    let forum_id = state.forum.id;

    let mut payload = Vec::new();
    bcs::serialize_into(&mut payload, &opcode).unwrap();
    bcs::serialize_into(&mut payload, &moderator).unwrap();

    Ok(Intent {
        module: "forum".into(),
        function: "apply_forum_intent".into(),
        nonce: nonce.nonce,
        objects: vec![
            IntentObject {
                id: nonce_shard_id,
                mutable: true,
            },
            IntentObject {
                id: forum_id,
                mutable: true,
            },
        ],
        payload,
        public_key: sponsor_addr,
        tweak: Address::ZERO,
        uid: vec![],
    })
}

async fn moderator_action(
    state: web::Data<AppState>,
    moderator: Address,
    opcode: u8,
) -> Result<HttpResponse, RelayError> {
    let intent = build_intent(&state, opcode, moderator).await?;
    let signature = state.sponsor.sign_blake2b(
        &bcs::to_bytes(&intent)
            .map_err(|e| RelayError::SponsorBuild(format!("failed to encode intent: {e}")))?,
    );
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(send::handle_send(&state, intent, signature, None, vec![]).await?))
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
