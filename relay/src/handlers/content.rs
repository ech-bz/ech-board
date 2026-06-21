use crate::app_state::AppState;
use crate::error::RelayError;
use crate::types::{ContentKind, FileType};
use actix_web::{HttpResponse, web};
use sui_sdk_types::Address;

use super::{BoardObject, PostObject, ThreadObject};

pub(crate) async fn fetch(
    state: web::Data<AppState>,
    board_uid: Address,
    thread_uid: Address,
    post_uid: Address,
    kind: ContentKind,
    hash: Address,
) -> Result<HttpResponse, actix_web::Error> {
    let objects = state
        .upstream
        .fetch_objects([board_uid, thread_uid, post_uid])
        .await
        .map_err(|e| actix_web::Error::from(RelayError::Internal(format!("content fetch: {e}"))))?;

    let board = match &objects[0] {
        Some(obj) => obj.contents().deserialize::<BoardObject>().map_err(|e| {
            actix_web::Error::from(RelayError::Internal(format!("bcs decode BoardObject: {e}")))
        })?,
        None => return Ok(HttpResponse::NotFound().finish()),
    };
    if board.projection.deleted {
        return Ok(HttpResponse::NotFound().finish());
    }

    let thread = match &objects[1] {
        Some(obj) => obj.contents().deserialize::<ThreadObject>().map_err(|e| {
            actix_web::Error::from(RelayError::Internal(format!(
                "bcs decode ThreadObject: {e}"
            )))
        })?,
        None => return Ok(HttpResponse::NotFound().finish()),
    };
    if thread.projection.deleted || thread.projection.closed {
        return Ok(HttpResponse::NotFound().finish());
    }

    let post = match &objects[2] {
        Some(obj) => obj.contents().deserialize::<PostObject>().map_err(|e| {
            actix_web::Error::from(RelayError::Internal(format!("bcs decode PostObject: {e}")))
        })?,
        None => return Ok(HttpResponse::NotFound().finish()),
    };
    if post.projection.deleted {
        return Ok(HttpResponse::NotFound().finish());
    }
    if !post.projection.media_hashes.contains(&hash) {
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
