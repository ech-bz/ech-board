use crate::app_state::AppState;
use crate::error::RelayError;
use crate::handlers::client_ip;
use actix_web::{HttpRequest, web};
use aws_sdk_kms::primitives::Blob;

pub(crate) async fn pk(state: web::Data<AppState>) -> Result<Vec<u8>, RelayError> {
    let output = state
        .kms
        .get_public_key()
        .key_id(&state.kms_moderator)
        .send()
        .await
        .map_err(|e| RelayError::Internal(format!("kms get_public_key: {e}")))?;

    output
        .public_key()
        .map(|p| p.as_ref().to_vec())
        .ok_or_else(|| RelayError::Internal("kms: no public key returned".into()))
}

pub(crate) async fn ip(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<Vec<u8>, RelayError> {
    let client_ip =
        client_ip(&req).ok_or_else(|| RelayError::SponsorBuild("no client IP".into()))?;

    let output = state
        .kms
        .generate_mac()
        .key_id(&state.kms_hmac)
        .message(Blob::new(client_ip.as_bytes()))
        .mac_algorithm(aws_sdk_kms::types::MacAlgorithmSpec::HmacSha256)
        .send()
        .await
        .map_err(|e| RelayError::Internal(format!("kms generate_mac: {e}")))?;

    let mac = output
        .mac()
        .ok_or_else(|| RelayError::Internal("kms: no mac returned".into()))?;

    bcs::to_bytes(mac.as_ref()).map_err(|e| RelayError::SponsorBuild(format!("bcs encode: {e}")))
}
