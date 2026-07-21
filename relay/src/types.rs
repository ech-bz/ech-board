use actix_multipart::form::{
    MultipartForm, bytes::Bytes as MultipartBytes, tempfile::TempFile, text::Text,
};
use serde::{Deserialize, Serialize};
use sui_sdk_types::Address;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ContentKind {
    Text,
    Media,
    Thumbnail,
    PlainText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileType {
    Jpeg,
    Png,
    WebP,
    Mp4,
    WebM,
}

impl FileType {
    pub(crate) fn detect(bytes: &[u8]) -> Option<Self> {
        match bytes {
            [0xff, 0xd8, ..] => Some(Self::Jpeg),
            [0x89, 0x50, 0x4e, 0x47, ..] => Some(Self::Png),
            [0x52, 0x49, 0x46, 0x46, ..] => Some(Self::WebP),
            [_, _, _, _, b'f', b't', b'y', b'p', ..] => Some(Self::Mp4),
            [0x1a, 0x45, 0xdf, 0xa3, ..] => Some(Self::WebM),
            _ => None,
        }
    }

    pub(crate) fn mime(&self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::WebP => "image/webp",
            Self::Mp4 => "video/mp4",
            Self::WebM => "video/webm",
        }
    }
}

#[derive(Debug, MultipartForm)]
pub(crate) struct SendForm {
    pub(crate) intent: MultipartBytes,
    pub(crate) signature: MultipartBytes,
    pub(crate) captcha: Text<String>,
    pub(crate) text: Option<MultipartBytes>,
    pub(crate) description: Option<Text<String>>,
    pub(crate) topic: Option<Text<String>>,
    pub(crate) media: Vec<TempFile>,
}

#[derive(Serialize)]
pub(crate) struct SendResponse {
    pub(crate) accepted_by: Vec<String>,
    pub(crate) digest: String,
    pub(crate) events: Vec<RelayEvent>,
}

#[derive(Serialize, Clone, Debug)]
pub(crate) struct RelayEvent {
    pub(crate) package_id: String,
    pub(crate) module: String,
    pub(crate) sender: String,
    pub(crate) event_type: String,
    pub(crate) contents: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Intent {
    pub(crate) module: String,
    pub(crate) function: String,
    pub(crate) nonce: u64,
    pub(crate) objects: Vec<IntentObject>,
    pub(crate) payload: Vec<u8>,
    pub(crate) public_key: Address,
    pub(crate) tweak: Address,
    pub(crate) uid: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct IntentObject {
    pub(crate) id: Address,
    pub(crate) mutable: bool,
}

pub(crate) const MAX_TEXT_SIZE: usize = 65536;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum PostPart {
    Plain(String),
    Bold(String),
    Italic(String),
    Code(String),
    ReplyTo(Address, Address),
    Secret {
        data_nonce: [u8; 12],
        data_ct: Vec<u8>,
        encrypted_keys: Vec<EncryptedKey>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct EncryptedKey {
    pub(crate) nonce: [u8; 12],
    pub(crate) ct: [u8; 32],
    pub(crate) tag: [u8; 16],
}
