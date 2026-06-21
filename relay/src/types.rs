use actix_multipart::form::{bytes::Bytes as MultipartBytes, text::Text, MultipartForm};
use sui_sdk_types::Address;

#[derive(Debug, MultipartForm)]
pub(crate) struct SendForm {
    pub(crate) intent: MultipartBytes,
    pub(crate) signature: MultipartBytes,
    pub(crate) captcha: Text<String>,
}

#[derive(serde::Serialize)]
pub(crate) struct SendResponse {
    pub(crate) accepted_by: Vec<String>,
    pub(crate) digest: String,
    pub(crate) events: Vec<RelayEvent>,
}

#[derive(serde::Serialize, Clone, Debug)]
pub(crate) struct RelayEvent {
    pub(crate) package_id: String,
    pub(crate) module: String,
    pub(crate) sender: String,
    pub(crate) event_type: String,
    pub(crate) contents: Vec<u8>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub(crate) struct Intent {
    pub(crate) module: String,
    pub(crate) function: String,
    pub(crate) nonce: u64,
    pub(crate) objects: Vec<IntentObject>,
    pub(crate) payload: Vec<u8>,
    pub(crate) public_key: Vec<u8>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub(crate) struct IntentObject {
    pub(crate) id: Address,
    pub(crate) mutable: bool,
}
