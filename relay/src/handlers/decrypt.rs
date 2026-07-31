use crate::app_state::AppState;
use crate::error;
use crate::handlers::{load_board, load_forum, load_post, load_thread};
use crate::types::DecryptRequest;
use aws_sdk_kms::primitives::Blob;
use blake2::Digest;
use blake2::digest::consts::U32;
use ed25519_dalek::Verifier;
use sui_sdk_types::{Address, TypeTag};

type Blake2b = blake2::Blake2b<U32>;

pub(crate) async fn handle(
    state: &AppState,
    req: DecryptRequest,
) -> Result<Vec<u8>, error::RelayError> {
    if req.path.is_empty() || req.path.len() > 4 {
        return Err(error::RelayError::SponsorBuild(
            "path must have 1-4 addresses".into(),
        ));
    }

    let mut msg: Vec<u8> = Vec::new();
    msg.extend_from_slice(&req.uid);
    for addr in &req.path {
        msg.extend_from_slice(addr.as_bytes());
    }
    msg.extend_from_slice(req.pk.as_bytes());
    let digest = Blake2b::digest(&msg);

    let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(
        <&[u8; 32]>::try_from(req.pk.as_bytes())
            .map_err(|_| error::RelayError::SponsorBuild("invalid pk length".into()))?,
    )
    .map_err(|e| error::RelayError::SponsorBuild(format!("invalid pk: {e}")))?;

    let signature = ed25519_dalek::Signature::from_slice(&req.signature)
        .map_err(|e| error::RelayError::SponsorBuild(format!("invalid signature: {e}")))?;

    verifying_key
        .verify(&digest, &signature)
        .map_err(|e| error::RelayError::SponsorBuild(format!("signature verification: {e}")))?;

    let forum = load_forum(&state.upstream, req.path[0])
        .await
        .map_err(|e| error::RelayError::SponsorBuild(format!("path[0] not a forum: {e}")))?;

    let board = if req.path.len() >= 2 {
        Some(
            load_board(&state.upstream, req.path[1])
            .await
            .map_err(|e| error::RelayError::SponsorBuild(format!("path[1] not a board: {e}")))?,
        )
    } else {
        None
    };

    let thread = if req.path.len() >= 3 {
        Some(
            load_thread(&state.upstream, req.path[2])
            .await
            .map_err(|e| error::RelayError::SponsorBuild(format!("path[2] not a thread: {e}")))?,
        )
    } else {
        None
    };

    let _post = if req.path.len() >= 4 {
        Some(
            load_post(&state.upstream, req.path[3])
            .await
            .map_err(|e| error::RelayError::SponsorBuild(format!("path[3] not a post: {e}")))?,
        )
    } else {
        None
    };

    let mut mods_table_ids = vec![forum.projection.mods.id];
    let mut bans_registry = &forum.projection.bans;

    if let Some(ref b) = board {
        mods_table_ids.push(b.projection.mods.id);
        bans_registry = &b.projection.bans;
    }
    if let Some(ref t) = thread {
        mods_table_ids.push(t.projection.mods.id);
        bans_registry = &t.projection.bans;
    }

    let authorized = check_authorization(
        state,
        &mods_table_ids,
        &sender_address(&req.pk),
        &forum.projection.admin,
        thread.as_ref().and_then(|t| t.projection.admin.as_ref()),
    )
    .await?;

    if !authorized {
        return Err(error::RelayError::SponsorBuild(
            "pk not authorized in chain".into(),
        ));
    }

    let decrypted = state
        .kms
        .decrypt()
        .key_id(&state.kms_moderator)
        .ciphertext_blob(Blob::new(req.uid))
        .send()
        .await
        .map_err(|e| error::RelayError::SponsorBuild(format!("kms decrypt: {e}")))?;

    let plaintext = decrypted
        .plaintext()
        .ok_or_else(|| error::RelayError::SponsorBuild("kms decrypt: no plaintext".into()))?;

    let pt = plaintext.as_ref();
    let chunks: [[u8; 32]; 4] = bcs::from_bytes(pt).map_err(|e| {
        error::RelayError::SponsorBuild(format!("bcs decode hmac chunks: {e}"))
    })?;
    let mask_bytes: [u8; 4] = [32, 24, 20, 16];
    let registries = [
        &bans_registry.ip32.entries.id,
        &bans_registry.ip24.entries.id,
        &bans_registry.ip20.entries.id,
        &bans_registry.ip16.entries.id,
    ];
    let mut out = Vec::with_capacity(128);
    for (i, chunk) in chunks.iter().enumerate() {
        let mut buf = registries[i].as_bytes().to_vec();
        buf.push(mask_bytes[i]);
        buf.extend_from_slice(chunk);
        out.extend_from_slice(Blake2b::digest(&buf).as_slice());
    }

    Ok(out)
}

async fn check_authorization(
    state: &AppState,
    mods_table_ids: &[Address],
    pk: &Address,
    forum_admin: &Address,
    thread_admin: Option<&Address>,
) -> Result<bool, error::RelayError> {
    let mut entry_ids: Vec<Address> = Vec::with_capacity(mods_table_ids.len());

    for mods_id in mods_table_ids {
        entry_ids.push(mods_id.derive_dynamic_child_id(&TypeTag::Address, pk.as_bytes()));
    }

    let entries = state.upstream.fetch_objects(&entry_ids).await?;
    if entries.iter().any(|e| e.is_some()) {
        return Ok(true);
    }

    if forum_admin == pk {
        return Ok(true);
    }

    if thread_admin == Some(pk) {
        return Ok(true);
    }

    Ok(false)
}

fn sender_address(pk: &Address) -> Address {
    let mut bytes = Vec::with_capacity(33);
    bytes.push(0);
    bytes.extend_from_slice(pk.as_bytes());
    Address::new(Blake2b::digest(bytes).into())
}
