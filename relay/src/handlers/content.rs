use crate::app_state::AppState;
use crate::types::{ContentKind, FileType};
use actix_web::{HttpResponse, web};
use sui_sdk_types::Address;

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
    if board.projection.deleted {
        return Ok(HttpResponse::NotFound().finish());
    }

    let thread = load_thread(&state.upstream, thread_uid)
        .await
        .map_err(actix_web::Error::from)?;
    if thread.projection.deleted {
        return Ok(HttpResponse::NotFound().finish());
    }

    let post = load_post(&state.upstream, post_uid)
        .await
        .map_err(actix_web::Error::from)?;
    if post.projection.deleted {
        return Ok(HttpResponse::NotFound().finish());
    }
    if !post.projection.media_hashes.contains(&hash) {
        return Ok(HttpResponse::NotFound().finish());
    }
    if post.projection.banned_media.contains(&hash) {
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
