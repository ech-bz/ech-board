mod app_state;
mod captcha;
mod config;
mod error;
mod sponsor;
mod upstream;

use app_state::AppState;
use actix_multipart::form::{bytes::Bytes as MultipartBytes, text::Text, MultipartForm};
use actix_web::{get, post, web, App, HttpRequest, HttpResponse, HttpServer};
use sui_sdk_types::Address;
use sui_sdk_types::TransactionKind;

#[derive(Debug, MultipartForm)]
struct SendForm {
    intent: MultipartBytes,
    captcha: Text<String>,
}

#[derive(serde::Serialize)]
struct SendResponse {
    accepted_by: Vec<String>,
    digest: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
struct RelayIntent {
    sender: Address,
    transaction_kind: TransactionKind,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cfg = config::load().map_err(std::io::Error::other)?;
    let bind_addr = cfg.server.bind.clone();
    let state = AppState::from_config(cfg).await?;

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .service(send)
            .service(healthz)
    })
    .bind(bind_addr)?
    .run()
    .await
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
