mod app_state;
mod captcha;
mod config;
mod error;
mod handlers;
mod seaweed;
mod sponsor;
mod thumbnail;
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
            .service(thread_handler)
            .service(content_handler)
            .service(feed_handler)
            .service(healthz)
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

#[get("/thread/{uid}")]
async fn thread_handler(
    state: web::Data<AppState>,
    path: web::Path<Address>,
) -> Result<HttpResponse, error::RelayError> {
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(handlers::thread::fetch(&state, path.into_inner()).await?))
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

#[post("/send")]
async fn send(
    req: HttpRequest,
    state: web::Data<AppState>,
    MultipartForm(form): MultipartForm<SendForm>,
) -> Result<HttpResponse, error::RelayError> {
    let remote_ip = req.connection_info().realip_remote_addr().map(|s| s.to_string());
    state
        .captcha
        .verify(form.captcha.as_str(), remote_ip.as_deref())
        .await?;
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(
            handlers::send::handle_send(
                &state,
                form.intent.data.to_vec(),
                form.signature.data.to_vec(),
                form.text,
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
