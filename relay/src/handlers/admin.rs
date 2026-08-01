use crate::app_state::AppState;
use crate::error::RelayError;
use crate::types::{Intent, IntentObject, Request};
use actix_web::{HttpResponse, web};
use blake2::Digest;
use blake2::digest::consts::U32;
use sui_sdk_types::Address;
use sui_sdk_types::TypeTag;

type Blake2b = blake2::Blake2b<U32>;

use super::nonce::NonceInfo;
use super::{nonce, send};

fn shard_id(sharded_counter: &Address, sender: &Address) -> Address {
    let mut buf = vec![0u8];
    buf.extend_from_slice(sender.as_ref() as &[u8]);
    let addr = Address::new(Blake2b::digest(&buf).into());
    let hash = Blake2b::digest(&bcs::to_bytes(&addr).unwrap());
    let val = u64::from_be_bytes(hash[24..].try_into().unwrap());
    let index = val % 512;
    sharded_counter.derive_object_id(&TypeTag::U64, &index.to_le_bytes())
}

async fn build_intent(
    state: &AppState,
    event: &str,
    moderator: Address,
) -> Result<Intent, RelayError> {
    let sponsor_pk = state.sponsor.sponsor_public_key();
    let nonce_bytes = nonce::fetch(state, &sponsor_pk).await?;
    let nonce: NonceInfo = bcs::from_bytes(&nonce_bytes)
        .map_err(|e| RelayError::SponsorBuild(format!("nonce decode: {e}")))?;

    let nonce_shard_id = shard_id(&state.forum.projection.nonce_shards, &sponsor_pk);
    let forum_id = state.forum.id;

    let mut payload = Vec::new();
    bcs::serialize_into(&mut payload, event)
        .map_err(|e| RelayError::SponsorBuild(format!("event tag encode: {e}")))?;
    bcs::serialize_into(&mut payload, &moderator)
        .map_err(|e| RelayError::SponsorBuild(format!("moderator encode: {e}")))?;

    Ok(Intent {
        module: "main".into(),
        function: "forum_apply_intent_uid".into(),
        nonce: nonce.nonce,
        objects: vec![
            IntentObject {
                id: Address::from_hex("0x6").unwrap(),
                mutable: false,
            },
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
        requests: vec![Request::Uid],
        public_key: sponsor_pk,
        tweak: Address::ZERO,
    })
}

async fn moderator_action(
    state: web::Data<AppState>,
    moderator: Address,
    event: &str,
) -> Result<HttpResponse, RelayError> {
    let intent = build_intent(&state, event, moderator).await?;
    let signature = state.sponsor.sign_blake2b(
        &bcs::to_bytes(&intent)
            .map_err(|e| RelayError::SponsorBuild(format!("failed to encode intent: {e}")))?,
    );
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(
            send::handle_send(
                &state,
                intent,
                signature,
                "0.0.0.0",
                None,
                None,
                None,
                vec![],
            )
            .await?,
        ))
}

pub(crate) async fn add_moderator(
    state: web::Data<AppState>,
    moderator: Address,
) -> Result<HttpResponse, RelayError> {
    moderator_action(state, moderator, "add_moderator").await
}

pub(crate) async fn del_moderator(
    state: web::Data<AppState>,
    moderator: Address,
) -> Result<HttpResponse, RelayError> {
    moderator_action(state, moderator, "del_moderator").await
}
