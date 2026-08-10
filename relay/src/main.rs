mod app_state;
mod captcha;
mod config;
mod error;
mod geoip;
mod handlers;
mod import;
mod seaweed;
mod sponsor;
mod thumbnail;
mod tripcode;
mod types;
mod upstream;

use actix_cors::Cors;
use actix_multipart::form::{MultipartForm, MultipartFormConfig};
use actix_web::web::PayloadConfig;
use actix_web::{App, HttpRequest, HttpResponse, HttpServer, get, post, put, web};
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
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Migrate a board from the old backend dump
    Import {
        #[arg(long)]
        board: String,
        #[arg(long)]
        dump: PathBuf,
        #[arg(long)]
        file_base: String,
        #[arg(long)]
        file_key: String,
        #[arg(long)]
        state: PathBuf,
    },
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cli = Cli::parse();
    let cfg = config::load(&cli.config).map_err(std::io::Error::other)?;
    let bind_addr = cfg.server.bind.clone();
    let admin_bind = cfg.server.admin_bind.clone();
    let admin_key_config = cfg.sponsor.private_key_base64.clone();
    let state = AppState::from_config(cfg).await?;

    if let Some(Command::Import {
        board,
        dump,
        file_base,
        file_key,
        state: state_path,
    }) = cli.command
    {
        let admin_key = admin_key_config;
        import::run(
            &state,
            import::ImportOptions {
                board,
                dump,
                admin_key,
                file_base,
                file_key,
                state_path,
            },
        )
        .await
        .map_err(std::io::Error::other)?;
        return Ok(());
    }

    let state = web::Data::new(state);

    let public_state = state.clone();
    let public_server = HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .wrap(cors)
            .app_data(PayloadConfig::new(200 * 1024 * 1024))
            .app_data(MultipartFormConfig::default().total_limit(200 * 1024 * 1024))
            .app_data(public_state.clone())
            .service(send)
            .service(nonce_handler)
            .service(forum_handler)
            .service(board_handler)
            .service(resolve_post_handler)
            .service(resolve_thread_handler)
            .service(thread_handler)
            .service(post_handler)
            .service(content_handler)
            .service(reaction_content_handler)
            .service(reaction_put_handler)
            .service(post_reactions_handler)
            .service(thread_reactions_handler)
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
    req: HttpRequest,
    path: web::Path<(Address, Address, Address, types::ContentKind, Address)>,
) -> Result<HttpResponse, actix_web::Error> {
    let (board_uid, thread_uid, post_uid, kind, hash) = path.into_inner();
    handlers::content::fetch(state, req, board_uid, thread_uid, post_uid, kind, hash).await
}

#[get("/content/{board_uid}/reaction/{hash}")]
async fn reaction_content_handler(
    state: web::Data<AppState>,
    path: web::Path<(Address, Address)>,
) -> Result<HttpResponse, actix_web::Error> {
    let (board_uid, hash) = path.into_inner();
    handlers::content::reaction_fetch(state, board_uid, hash).await
}

#[put("/content/{board_uid}/reaction/{hash}")]
async fn reaction_put_handler(
    state: web::Data<AppState>,
    path: web::Path<(Address, Address)>,
    body: web::Bytes,
) -> Result<HttpResponse, actix_web::Error> {
    let (_, hash) = path.into_inner();
    handlers::content::reaction_put(state, hash, body.to_vec()).await
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

#[get("/board/{uid}/thread/{number}")]
async fn resolve_thread_handler(
    state: web::Data<AppState>,
    path: web::Path<(Address, u64)>,
) -> Result<HttpResponse, error::RelayError> {
    let (uid, number) = path.into_inner();
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(handlers::board::resolve_thread(&state, uid, number).await?))
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

#[get("/post/{uid}/reactions")]
async fn post_reactions_handler(
    state: web::Data<AppState>,
    path: web::Path<Address>,
    query: web::Query<handlers::reactions::ReactionsQuery>,
) -> Result<HttpResponse, error::RelayError> {
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(handlers::reactions::fetch(&state, path.into_inner(), query.pk).await?))
}

#[post("/thread/{uid}/reactions")]
async fn thread_reactions_handler(
    state: web::Data<AppState>,
    _path: web::Path<Address>,
    body: web::Bytes,
) -> Result<HttpResponse, error::RelayError> {
    let queries: Vec<(Address, Address)> = bcs::from_bytes(&body)
        .map_err(|e| error::RelayError::Internal(format!("bcs decode reactions query: {e}")))?;
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(handlers::reactions::fetch_thread(&state, queries).await?))
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

    if form.intent.is_empty() {
        return Err(error::RelayError::SponsorBuild("no intents provided".into()));
    }
    if form.intent.len() != form.signature.len() {
        return Err(error::RelayError::SponsorBuild(
            "intent/signature count mismatch".into(),
        ));
    }
    let intents: Vec<(types::IntentV2, Vec<u8>)> = form
        .intent
        .iter()
        .zip(form.signature.iter())
        .map(|(i, s)| {
            let intent: types::IntentV2 = bcs::from_bytes(&i.data).map_err(|e| {
                error::RelayError::SponsorBuild(format!("failed to decode intent: {e}"))
            })?;
            Ok((intent, s.data.to_vec()))
        })
        .collect::<Result<Vec<_>, error::RelayError>>()?;

    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(
            handlers::send::handle_send(
                &state,
                intents,
                &remote_ip,
                Some(form.captcha.as_str()),
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
