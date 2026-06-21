mod app_state;
mod captcha;
mod config;
mod error;
mod registry;
mod shards;
mod sponsor;
mod types;
mod upstream;

use actix_multipart::form::MultipartForm;
use actix_web::{get, post, web, App, HttpRequest, HttpResponse, HttpServer};
use app_state::AppState;
use clap::Parser;
use std::path::PathBuf;
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
    let state = AppState::from_config(cfg).await?;

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .service(send)
            .service(shards_handler)
            .service(registry_handler)
            .service(healthz)
    })
    .bind(bind_addr)?
    .run()
    .await
}

#[get("/board_slug_registry")]
async fn registry_handler(state: web::Data<AppState>) -> Result<HttpResponse, error::RelayError> {
    let bcs_bytes = registry::get_registry_cached(
        &state.graphql_client,
        &state.registry_cache,
        &state.graphql_url,
        &state.forum_package_id.to_string(),
    )
    .await?;
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(bcs_bytes))
}

#[get("/shards")]
async fn shards_handler(state: web::Data<AppState>) -> Result<HttpResponse, error::RelayError> {
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(
            shards::get_shards_cached(
                &state.graphql_client,
                &state.shards_cache,
                &state.graphql_url,
                &state.forum_package_id.to_string(),
            )
            .await?,
        ))
}

#[get("/healthz")]
async fn healthz() -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[post("/send")]
async fn send(
    req: HttpRequest,
    state: web::Data<AppState>,
    MultipartForm(form): MultipartForm<SendForm>,
) -> Result<HttpResponse, error::RelayError> {
    let result = state.handle_send(&req, form).await?;
    Ok(HttpResponse::Ok().json(result))
}
