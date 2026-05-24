use crate::config::SponsorConfig;
use crate::error::RelayError;
use base64::Engine;
use ed25519_dalek::Signer;
use sui_sdk_types::Address;
use sui_sdk_types::Ed25519PublicKey;
use sui_sdk_types::Ed25519Signature;
use sui_sdk_types::SimpleSignature;
use sui_sdk_types::SignedTransaction;
use sui_sdk_types::SignatureScheme;
use sui_sdk_types::Transaction;

#[derive(Clone)]
pub struct SponsorService {
    signing_key: ed25519_dalek::SigningKey,
    sponsor_address: Address,
}

impl SponsorService {
    pub fn new(cfg: SponsorConfig) -> Result<Self, RelayError> {
        let key_bytes = base64::engine::general_purpose::STANDARD
            .decode(cfg.private_key_base64.as_bytes())
            .map_err(|e| RelayError::SponsorConfig(format!("invalid sponsor key base64: {e}")))?;
        if key_bytes.len() != 33 {
            return Err(RelayError::SponsorConfig(
                "sponsor key must be 33 bytes (scheme flag + 32-byte secret)".to_string(),
            ));
        }
        if key_bytes[0] != SignatureScheme::Ed25519.to_u8() {
            return Err(RelayError::SponsorConfig(
                "sponsor key scheme must be ed25519".to_string(),
            ));
        }

        let mut secret = [0u8; 32];
        secret.copy_from_slice(&key_bytes[1..]);
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret);
        let public_key = Ed25519PublicKey::new(signing_key.verifying_key().to_bytes());
        let sponsor_address = public_key.derive_address();

        Ok(Self {
            signing_key,
            sponsor_address,
        })
    }

    pub fn sponsor_address(&self) -> Address {
        self.sponsor_address
    }

    pub fn sign_as_sender(&self, tx: Transaction) -> SignedTransaction {
        let digest = tx.signing_digest();
        let sig = self.signing_key.sign(&digest).to_bytes();
        let signer_sig = sui_sdk_types::UserSignature::Simple(SimpleSignature::Ed25519 {
            signature: Ed25519Signature::new(sig),
            public_key: Ed25519PublicKey::new(self.signing_key.verifying_key().to_bytes()),
        });
        SignedTransaction {
            transaction: tx,
            signatures: vec![signer_sig],
        }
    }
}
