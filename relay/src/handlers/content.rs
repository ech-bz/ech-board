use crate::app_state::AppState;
use crate::types::{ContentKind, FileType};
use actix_web::{HttpResponse, web};
use blake2::Digest;
use blake2::digest::consts::U32;
use sui_sdk_types::Address;

type Blake2b = blake2::Blake2b<U32>;

use super::{load_board, load_post, load_thread};

pub(crate) async fn fetch(
    state: web::Data<AppState>,
    board_uid: Address,
    thread_uid: Address,
    post_uid: Address,
    kind: ContentKind,
    hash: Address,
) -> Result<HttpResponse, actix_web::Error> {
    let board = load_board(&state.upstream, board_uid)
        .await
        .map_err(actix_web::Error::from)?;
    if board.projection.deleted() {
        return Ok(HttpResponse::NotFound().finish());
    }

    let thread = load_thread(&state.upstream, thread_uid)
        .await
        .map_err(actix_web::Error::from)?;
    if thread.projection.deleted() {
        return Ok(HttpResponse::NotFound().finish());
    }

    let post = load_post(&state.upstream, post_uid)
        .await
        .map_err(actix_web::Error::from)?;
    if post.projection.deleted() {
        return Ok(HttpResponse::NotFound().finish());
    }
    if !post.projection.media_hashes().contains(&hash) {
        return Ok(HttpResponse::NotFound().finish());
    }
    if post.projection.banned_media().contains(&hash) {
        return Ok(HttpResponse::NotFound().finish());
    }

    match state.seaweed.get(kind, &hash).await {
        Ok(Some(data)) => Ok(HttpResponse::Ok()
            .insert_header(("Cache-Control", "public, max-age=31536000, immutable"))
            .insert_header((
                "Content-Type",
                FileType::detect(&data).map_or("application/octet-stream", |f| f.mime()),
            ))
            .body(data)),
        Ok(None) => Ok(HttpResponse::NotFound().finish()),
        Err(e) => Ok(HttpResponse::InternalServerError().body(e.to_string())),
    }
}

pub(crate) async fn reaction_fetch(
    state: web::Data<AppState>,
    board_uid: Address,
    hash: Address,
) -> Result<HttpResponse, actix_web::Error> {
    let board = load_board(&state.upstream, board_uid)
        .await
        .map_err(actix_web::Error::from)?;
    if board.projection.deleted() {
        return Ok(HttpResponse::NotFound().finish());
    }
    if !board.projection.reactions().contains(&hash) {
        return Ok(HttpResponse::NotFound().finish());
    }

    match state.seaweed.get(ContentKind::Reaction, &hash).await {
        Ok(Some(data)) => Ok(HttpResponse::Ok()
            .insert_header(("Cache-Control", "public, max-age=31536000, immutable"))
            .insert_header((
                "Content-Type",
                FileType::detect(&data).map_or("application/octet-stream", |f| f.mime()),
            ))
            .body(data)),
        Ok(None) => Ok(HttpResponse::NotFound().finish()),
        Err(e) => Ok(HttpResponse::InternalServerError().body(e.to_string())),
    }
}

pub(crate) async fn reaction_put(
    state: web::Data<AppState>,
    hash: Address,
    data: Vec<u8>,
) -> Result<HttpResponse, actix_web::Error> {
    if Blake2b::digest(&data).as_slice() != hash.as_bytes() {
        return Ok(HttpResponse::BadRequest().body("content hash mismatch"));
    }
    if FileType::detect(&data).is_none() {
        return Ok(HttpResponse::BadRequest().body("unsupported file type"));
    }

    state
        .seaweed
        .put(ContentKind::Reaction, &hash, &data)
        .await
        .map_err(actix_web::Error::from)?;
    Ok(HttpResponse::Ok().finish())
}
