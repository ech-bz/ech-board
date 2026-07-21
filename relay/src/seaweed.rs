use crate::error::RelayError;
use crate::types::ContentKind;
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::Serialize;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use sui_sdk_types::Address;

#[derive(Clone)]
pub(crate) struct SeaweedClient {
    client: reqwest::Client,
    filer_url: String,
    jwt_key: jsonwebtoken::EncodingKey,
}

#[derive(Serialize)]
struct Claims {
    sub: String,
    iat: u64,
}

impl SeaweedClient {
    pub(crate) fn new(filer_url: String, jwt_signing_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("reqwest client build");
        Self {
            client,
            filer_url: filer_url.trim_end_matches('/').to_string(),
            jwt_key: EncodingKey::from_secret(jwt_signing_key.as_bytes()),
        }
    }

    fn token(&self) -> Result<String, RelayError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| RelayError::Internal(format!("seaweed jwt time: {e}")))?
            .as_secs();
        let claims = Claims {
            sub: "relay".into(),
            iat: now,
        };
        encode(&Header::default(), &claims, &self.jwt_key)
            .map_err(|e| RelayError::Internal(format!("seaweed jwt encode: {e}")))
    }

    fn auth_header(&self) -> Result<String, RelayError> {
        Ok(format!("BEARER {}", self.token()?))
    }

    fn path(&self, kind: ContentKind, hash: &Address) -> String {
        let kind_str = match kind {
            ContentKind::Text => "text",
            ContentKind::Media => "media",
            ContentKind::Thumbnail => "thumb",
            ContentKind::PlainText => "plaintext",
        };
        let hex = hex::encode(hash.as_bytes());
        format!("{}/{}/{}/{}", kind_str, &hex[0..2], &hex[2..4], hex)
    }

    fn url(&self, kind: ContentKind, hash: &Address) -> String {
        format!("{}/{}", self.filer_url, self.path(kind, hash))
    }

    pub(crate) async fn put(
        &self,
        kind: ContentKind,
        hash: &Address,
        data: &[u8],
    ) -> Result<(), RelayError> {
        let url = self.url(kind, hash);
        let resp = self
            .client
            .put(&url)
            .header("Authorization", &self.auth_header()?)
            .body(data.to_vec())
            .send()
            .await
            .map_err(|e| RelayError::Internal(format!("seaweed put: {e}")))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(RelayError::Internal(format!("seaweed put HTTP {}", status)));
        }
        Ok(())
    }

    pub(crate) async fn get(
        &self,
        kind: ContentKind,
        hash: &Address,
    ) -> Result<Option<Vec<u8>>, RelayError> {
        let url = self.url(kind, hash);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", &self.auth_header()?)
            .send()
            .await
            .map_err(|e| RelayError::Internal(format!("seaweed get: {e}")))?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let status = resp.status();
        if !status.is_success() {
            return Err(RelayError::Internal(format!("seaweed get HTTP {}", status)));
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| RelayError::Internal(format!("seaweed get body: {e}")))?;
        Ok(Some(bytes.to_vec()))
    }

    pub(crate) async fn delete(&self, kind: ContentKind, hash: &Address) -> Result<(), RelayError> {
        let url = self.url(kind, hash);
        let resp = self
            .client
            .delete(&url)
            .header("Authorization", &self.auth_header()?)
            .send()
            .await
            .map_err(|e| RelayError::Internal(format!("seaweed delete: {e}")))?;
        let status = resp.status();
        if !status.is_success() && status != reqwest::StatusCode::NOT_FOUND {
            return Err(RelayError::Internal(format!(
                "seaweed delete HTTP {}",
                status
            )));
        }
        Ok(())
    }
}
