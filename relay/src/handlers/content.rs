use crate::app_state::AppState;
use crate::types::{ContentKind, FileType, MediaMeta};
use actix_web::{HttpRequest, HttpResponse, http::StatusCode, web};
use blake2::Digest;
use blake2::digest::consts::U32;
use sui_sdk_types::Address;

type Blake2b = blake2::Blake2b<U32>;

use crate::error::RelayError;

use super::board::{decode_board, load_board};
use super::load_roots_fields;
use super::post::decode_post;
use super::thread::decode_thread;

pub(crate) async fn fetch(
    state: web::Data<AppState>,
    req: HttpRequest,
    board_uid: Address,
    thread_uid: Address,
    post_uid: Address,
    kind: ContentKind,
    hash: Address,
) -> Result<HttpResponse, actix_web::Error> {
    let (roots, mut fields) =
        load_roots_fields(&state.upstream, &[board_uid, thread_uid, post_uid]).await?;
    let mut roots = roots.into_iter();
    let board_root = roots
        .next()
        .ok_or_else(|| RelayError::Internal("board root missing".to_string()))?;
    let thread_root = roots
        .next()
        .ok_or_else(|| RelayError::Internal("thread root missing".to_string()))?;
    let post_root = roots
        .next()
        .ok_or_else(|| RelayError::Internal("post root missing".to_string()))?;
    let board_fields = fields
        .remove(&board_uid)
        .ok_or_else(|| RelayError::Internal("board fields missing".to_string()))?;
    let thread_fields = fields
        .remove(&thread_uid)
        .ok_or_else(|| RelayError::Internal("thread fields missing".to_string()))?;
    let post_fields = fields
        .remove(&post_uid)
        .ok_or_else(|| RelayError::Internal("post fields missing".to_string()))?;
    let board = decode_board(board_root, board_fields)?;
    let thread = decode_thread(thread_root, thread_fields)?;
    let post = decode_post(post_root, post_fields)?;
    if board.projection.deleted() {
        return Ok(HttpResponse::NotFound().finish());
    }
    if thread.projection.deleted() {
        return Ok(HttpResponse::NotFound().finish());
    }
    if post.projection.deleted() {
        return Ok(HttpResponse::NotFound().finish());
    }
    if kind == ContentKind::Text {
        if post.projection.text_hash() != Some(hash) {
            return Ok(HttpResponse::NotFound().finish());
        }
    } else {
        if !post.projection.media_hashes().contains(&hash)
            || post.projection.banned_media().contains(&hash)
        {
            return Ok(HttpResponse::NotFound().finish());
        }
    }

    if kind == ContentKind::Media {
        return stream_media(&state, req, &hash).await;
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

async fn stream_media(
    state: &AppState,
    req: HttpRequest,
    hash: &Address,
) -> Result<HttpResponse, actix_web::Error> {
    let mime = match state.seaweed.get(ContentKind::MediaMeta, hash).await {
        Ok(Some(meta)) => bcs::from_bytes::<MediaMeta>(&meta)
            .map_err(|e| {
                actix_web::Error::from(crate::error::RelayError::Internal(format!(
                    "media meta bcs: {e}"
                )))
            })?
            .mime,
        Ok(None) => return Ok(HttpResponse::NotFound().finish()),
        Err(e) => return Ok(HttpResponse::InternalServerError().body(e.to_string())),
    };

    let range = req
        .headers()
        .get("Range")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    match state.seaweed.open(ContentKind::Media, hash, range.as_deref()).await {
        Ok(Some(resp)) => {
            let status = if resp.status() == reqwest::StatusCode::PARTIAL_CONTENT {
                StatusCode::PARTIAL_CONTENT
            } else {
                StatusCode::OK
            };
            let mut builder = HttpResponse::build(status);
            builder
                .insert_header(("Cache-Control", "public, max-age=31536000, immutable"))
                .insert_header(("Accept-Ranges", "bytes"))
                .insert_header(("Content-Type", mime.as_str()));
            if let Some(len) = resp.content_length() {
                builder.insert_header(("Content-Length", len.to_string()));
            }
            if let Some(cr) = resp.headers().get("content-range") {
                if let Ok(s) = cr.to_str() {
                    builder.insert_header(("Content-Range", s));
                }
            }
            Ok(builder.streaming(resp.bytes_stream()))
        }
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
