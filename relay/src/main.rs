mod app_state;
mod captcha;
mod config;
mod error;
mod geoip;
mod handlers;
mod seaweed;
mod sponsor;
mod thumbnail;
mod tripcode;
mod types;
mod upstream;

use actix_cors::Cors;
use actix_multipart::form::MultipartForm;
use actix_web::web::PayloadConfig;
use actix_web::{App, HttpRequest, HttpResponse, HttpServer, get, post, web};
use app_state::AppState;
use clap::Parser;
use std::path::PathBuf;
use sui_sdk_types::Address;
use types::DecryptRequest;
use types::SendForm;

#[derive(Parser)]
#[command(name = "ech-board-relay")]
struct Cli {
    #[arg(short, long)]
    config: PathBuf,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cli = Cli::parse();
    let cfg = config::load(&cli.config).map_err(std::io::Error::other)?;
    let bind_addr = cfg.server.bind.clone();
    let admin_bind = cfg.server.admin_bind.clone();
    let state = web::Data::new(AppState::from_config(cfg).await?);

    let public_state = state.clone();
    let public_server = HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .wrap(cors)
            .app_data(PayloadConfig::new(200 * 1024 * 1024))
            .app_data(public_state.clone())
            .service(send)
            .service(nonce_handler)
            .service(forum_handler)
            .service(board_handler)
            .service(resolve_post_handler)
            .service(thread_handler)
            .service(post_handler)
            .service(content_handler)
            .service(feed_handler)
            .service(bans_handler)
            .service(healthz)
            .service(decrypt_handler)
    })
    .bind(&bind_addr)?
    .run();

    let admin_state = state.clone();
    let admin_server = HttpServer::new(move || {
        App::new()
            .app_data(admin_state.clone())
            .service(add_moderator)
            .service(del_moderator)
    })
    .bind(&admin_bind)?
    .run();

    futures::try_join!(public_server, admin_server)?;
    Ok(())
}

#[get("/nonce/{sender}")]
async fn nonce_handler(
    state: web::Data<AppState>,
    path: web::Path<Address>,
) -> Result<HttpResponse, error::RelayError> {
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(handlers::nonce::fetch(&state, &path.into_inner()).await?))
}

#[get("/healthz")]
async fn healthz() -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[get("/content/{board_uid}/{thread_uid}/{post_uid}/{kind}/{hash}")]
async fn content_handler(
    state: web::Data<AppState>,
    path: web::Path<(Address, Address, Address, types::ContentKind, Address)>,
) -> Result<HttpResponse, actix_web::Error> {
    let (board_uid, thread_uid, post_uid, kind, hash) = path.into_inner();
    handlers::content::fetch(state, board_uid, thread_uid, post_uid, kind, hash).await
}

#[get("/forum")]
async fn forum_handler(state: web::Data<AppState>) -> Result<HttpResponse, error::RelayError> {
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(handlers::forum::fetch(&state).await?))
}

#[get("/board/{uid}")]
async fn board_handler(
    state: web::Data<AppState>,
    path: web::Path<Address>,
    query: web::Query<handlers::Pagination>,
) -> Result<HttpResponse, error::RelayError> {
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(handlers::board::fetch(&state, path.into_inner(), query.cursor).await?))
}

#[get("/board/{uid}/post/{number}")]
async fn resolve_post_handler(
    state: web::Data<AppState>,
    path: web::Path<(Address, u64)>,
) -> Result<HttpResponse, error::RelayError> {
    let (uid, number) = path.into_inner();
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(handlers::board::resolve_post(&state, uid, number).await?))
}

#[get("/thread/{uid}")]
async fn thread_handler(
    state: web::Data<AppState>,
    path: web::Path<Address>,
) -> Result<HttpResponse, error::RelayError> {
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(handlers::thread::fetch(&state, path.into_inner()).await?))
}

#[get("/post/{uid}")]
async fn post_handler(
    state: web::Data<AppState>,
    path: web::Path<Address>,
) -> Result<HttpResponse, error::RelayError> {
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(handlers::post::fetch(&state, path.into_inner()).await?))
}

#[get("/feed/{uid}")]
async fn feed_handler(
    state: web::Data<AppState>,
    path: web::Path<Address>,
    query: web::Query<handlers::feed::FeedQuery>,
) -> Result<HttpResponse, error::RelayError> {
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(handlers::feed::fetch(&state, path.into_inner(), query.into_inner()).await?))
}

#[get("/bans/{uid}")]
async fn bans_handler(
    state: web::Data<AppState>,
    path: web::Path<Address>,
    query: web::Query<handlers::Pagination>,
) -> Result<HttpResponse, error::RelayError> {
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(handlers::bans::fetch(&state, path.into_inner(), query.cursor).await?))
}

#[post("/decrypt")]
async fn decrypt_handler(
    state: web::Data<AppState>,
    body: web::Json<DecryptRequest>,
) -> Result<HttpResponse, error::RelayError> {
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(handlers::decrypt::handle(&state, body.into_inner()).await?))
}

#[post("/send")]
async fn send(
    req: HttpRequest,
    state: web::Data<AppState>,
    MultipartForm(form): MultipartForm<SendForm>,
) -> Result<HttpResponse, error::RelayError> {
    let remote_ip = handlers::client_ip(&req)
        .ok_or_else(|| error::RelayError::SponsorBuild("no client IP".into()))?;
    state
        .captcha
        .verify(form.captcha.as_str(), remote_ip.as_str())
        .await?;

    let intent: types::Intent = bcs::from_bytes(&form.intent.data)
        .map_err(|e| error::RelayError::SponsorBuild(format!("failed to decode intent: {e}")))?;

    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(
            handlers::send::handle_send(
                &state,
                intent,
                form.signature.data.to_vec(),
                &remote_ip,
                form.text,
                form.description.map(|t| t.into_inner()),
                form.topic.map(|t| t.into_inner()),
                form.reason.map(|t| t.into_inner()),
                form.name.map(|t| t.into_inner()),
                form.tripcode.map(|t| t.into_inner()),
                form.media,
            )
            .await?,
        ))
}

#[post("/add_moderator")]
async fn add_moderator(
    state: web::Data<AppState>,
    body: web::Json<Address>,
) -> Result<HttpResponse, error::RelayError> {
    handlers::admin::add_moderator(state, body.into_inner()).await
}

#[post("/del_moderator")]
async fn del_moderator(
    state: web::Data<AppState>,
    body: web::Json<Address>,
) -> Result<HttpResponse, error::RelayError> {
    handlers::admin::del_moderator(state, body.into_inner()).await
}
