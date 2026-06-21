use crate::captcha::CaptchaVerifier;
use crate::config;
use crate::error;
use crate::sponsor::SponsorService;
use crate::types::{Intent, SendForm, SendResponse};
use crate::upstream::UpstreamSender;
use futures::StreamExt;
use rand::seq::SliceRandom;
use std::str::FromStr;
use std::time::Duration;
use sui_rpc::field::{FieldMask, FieldMaskUtil};
use sui_rpc::proto::sui::rpc::v2::GetObjectRequest;
use sui_rpc::proto::sui::rpc::v2::ListOwnedObjectsRequest;
use sui_sdk_types::hash::Hasher;
use sui_sdk_types::Address;
use sui_sdk_types::GasPayment;
use sui_sdk_types::Identifier;
use sui_sdk_types::Input;
use sui_sdk_types::MoveCall;
use sui_sdk_types::ProgrammableTransaction;
use sui_sdk_types::Transaction;
use sui_sdk_types::TransactionKind;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) captcha: CaptchaVerifier,
    pub(crate) upstream: UpstreamSender,
    pub(crate) sponsor: SponsorService,
    pub(crate) sponsor_gas_objects: Vec<Address>,
    pub(crate) sponsor_gas_budget: u64,
    pub(crate) sponsor_gas_price: u64,
    pub(crate) forum_package_id: Address,
    pub(crate) shards: Vec<Address>,
}

impl AppState {
    pub(crate) async fn from_config(cfg: config::AppConfig) -> Result<Self, std::io::Error> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(cfg.upstream.request_timeout_ms))
            .build()
            .map_err(std::io::Error::other)?;

        let sponsor_cfg = cfg.sponsor.clone();
        let sponsor = SponsorService::new(cfg.sponsor).map_err(std::io::Error::other)?;
        let upstream_rpc_url = cfg.upstream.submit_url.clone();
        let sui_client = sui_rpc::Client::new(upstream_rpc_url.as_str()).map_err(|e| {
            std::io::Error::other(format!("failed to init upstream rpc client: {e}"))
        })?;

        let forum_package_id = Address::from_str(&cfg.forum_package_id)
            .map_err(|e| std::io::Error::other(format!("invalid forum_package_id: {e}")))?;

        let sponsor_gas_objects = Self::fetch_gas_objects(&sui_client, sponsor.sponsor_address())
            .await
            .map_err(std::io::Error::other)?;

        let auth_registry_id =
            Self::fetch_auth_registry_id(&client, &cfg.upstream.submit_url, forum_package_id)
                .await
                .map_err(std::io::Error::other)?;

        let shards = Self::fetch_registry_shards(&sui_client, auth_registry_id)
            .await
            .map_err(std::io::Error::other)?;

        Ok(Self {
            captcha: CaptchaVerifier::new(client, cfg.captcha),
            upstream: UpstreamSender::new(
                sui_client,
                cfg.upstream.submit_url,
                cfg.upstream.request_timeout_ms,
            ),
            sponsor,
            sponsor_gas_objects,
            sponsor_gas_budget: sponsor_cfg.gas_budget,
            sponsor_gas_price: sponsor_cfg.gas_price,
            forum_package_id,
            shards,
        })
    }

    async fn fetch_gas_objects(
        client: &sui_rpc::Client,
        sponsor: Address,
    ) -> Result<Vec<Address>, error::RelayError> {
        let request = ListOwnedObjectsRequest::default().with_owner(&sponsor);
        let mut stream = Box::pin(client.list_owned_objects(request));
        let mut gas_ids = Vec::new();
        while let Some(object_res) = stream.next().await {
            let object = object_res.map_err(|e| {
                error::RelayError::SponsorBuild(format!("failed to list sponsor objects: {e}"))
            })?;
            let obj_type = object.object_type.as_deref().unwrap_or("");
            if obj_type.contains("Coin") && obj_type.contains("sui::SUI") {
                let id = Address::from_str(object.object_id()).map_err(|e| {
                    error::RelayError::SponsorBuild(format!("invalid coin id: {e}"))
                })?;
                gas_ids.push(id);
            }
        }
        if gas_ids.is_empty() {
            return Err(error::RelayError::SponsorBuild(
                "sponsor has no SUI gas objects".to_string(),
            ));
        }
        eprintln!("relay sponsor gas pool loaded size={}", gas_ids.len());
        Ok(gas_ids)
    }

    async fn fetch_auth_registry_id(
        client: &reqwest::Client,
        submit_url: &str,
        forum_package_id: Address,
    ) -> Result<Address, error::RelayError> {
        let rpc_url = format!("{}/", submit_url.trim_end_matches('/'));
        let event_type = format!("{}::intent::IntentGateRegistryCreated", forum_package_id);
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "suix_queryEvents",
            "params": [{"MoveEventType": event_type}]
        });
        let resp = client
            .post(&rpc_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| error::RelayError::SponsorBuild(format!("rpc query events: {e}")))?;
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| error::RelayError::SponsorBuild(format!("rpc parse response: {e}")))?;
        let registry_id = json["result"]["data"][0]["parsedJson"]["registry_id"]
            .as_str()
            .ok_or_else(|| {
                error::RelayError::SponsorBuild(
                    "IntentGateRegistryCreated event not found".to_string(),
                )
            })?;
        Address::from_str(registry_id).map_err(|e| {
            error::RelayError::SponsorBuild(format!("invalid registry_id from event: {e}"))
        })
    }

    fn shard_index_from_public_key(public_key: &[u8]) -> u64 {
        let digest = Hasher::digest(public_key);
        let bytes: &[u8] = digest.as_ref();
        let first = bytes[0] as u64;
        let second = (bytes[1] as u64) << 8;
        (first + second) % 1024
    }

    fn auth_shard_id(&self, public_key: &[u8]) -> Address {
        let shard_index = Self::shard_index_from_public_key(public_key) as usize;
        self.shards[shard_index]
    }

    async fn fetch_registry_shards(
        client: &sui_rpc::Client,
        registry_id: Address,
    ) -> Result<Vec<Address>, error::RelayError> {
        let mut rpc = client.clone();
        let mut request = GetObjectRequest::new(&registry_id);
        request.read_mask = Some(FieldMask::from_str("contents"));
        let response =
            rpc.ledger_client().get_object(request).await.map_err(|e| {
                error::RelayError::SponsorBuild(format!("get_object registry: {e}"))
            })?;
        let object = response.into_inner().object.ok_or_else(|| {
            error::RelayError::SponsorBuild("registry object not found".to_string())
        })?;
        let bcs_bytes = object
            .contents
            .as_ref()
            .and_then(|c| c.value.as_deref())
            .ok_or_else(|| {
                error::RelayError::SponsorBuild("registry object missing BCS contents".to_string())
            })?;
        let registry: RegistryBcs = bcs::from_bytes(bcs_bytes).map_err(|e| {
            error::RelayError::SponsorBuild(format!("failed to deserialize registry: {e}"))
        })?;
        let max_key = registry
            .shards
            .contents
            .iter()
            .map(|e| e.key)
            .max()
            .unwrap_or(0) as usize;
        let mut shards = vec![Address::ZERO; max_key + 1];
        for entry in registry.shards.contents {
            shards[entry.key as usize] = Address::new(entry.value);
        }
        Ok(shards)
    }

    async fn build_transaction_from_intent(
        &self,
        intent_raw: &[u8],
        signature_bytes: &[u8],
        sponsor_gas_object_id: Address,
    ) -> Result<Transaction, error::RelayError> {
        let intent: Intent = bcs::from_bytes(intent_raw).map_err(|e| {
            error::RelayError::SponsorBuild(format!("failed to decode intent: {e}"))
        })?;

        let mut inputs = vec![
            Input::Shared(
                self.upstream
                    .shared_input(&self.auth_shard_id(&intent.public_key), true)
                    .await?,
            ),
            Input::Pure(bcs::to_bytes(&intent_raw.to_vec()).map_err(|e| {
                error::RelayError::SponsorBuild(format!("failed to encode intent bytes: {e}"))
            })?),
            Input::Pure(bcs::to_bytes(&signature_bytes.to_vec()).map_err(|e| {
                error::RelayError::SponsorBuild(format!("failed to encode signature: {e}"))
            })?),
        ];
        for object in &intent.objects {
            inputs.push(Input::Shared(
                self.upstream
                    .shared_input(&object.id, object.mutable)
                    .await?,
            ));
        }

        let commands = vec![sui_sdk_types::Command::MoveCall(MoveCall {
            package: self.forum_package_id,
            module: Identifier::from_str(intent.module.as_str()).map_err(|e| {
                error::RelayError::SponsorBuild(format!("failed to parse module name: {e}"))
            })?,
            function: Identifier::from_str(intent.function.as_str()).map_err(|e| {
                error::RelayError::SponsorBuild(format!("failed to parse function name: {e}"))
            })?,
            type_arguments: vec![],
            arguments: (0u16..inputs.len() as u16)
                .map(sui_sdk_types::Argument::Input)
                .collect(),
        })];

        Ok(Transaction {
            kind: TransactionKind::ProgrammableTransaction(ProgrammableTransaction {
                inputs,
                commands,
            }),
            sender: self.sponsor.sponsor_address(),
            gas_payment: GasPayment {
                objects: vec![self.upstream.object_ref(&sponsor_gas_object_id).await?],
                owner: self.sponsor.sponsor_address(),
                price: self.sponsor_gas_price,
                budget: self.sponsor_gas_budget,
            },
            expiration: sui_sdk_types::TransactionExpiration::None,
        })
    }

    pub(crate) async fn handle_send(
        &self,
        req: &actix_web::HttpRequest,
        form: SendForm,
    ) -> Result<SendResponse, error::RelayError> {
        let remote_ip = req.peer_addr().map(|a| a.ip().to_string());
        self.captcha
            .verify(form.captcha.as_str(), remote_ip.as_deref())
            .await?;

        let intent_bytes = form.intent.data.to_vec();
        let signature_bytes = form.signature.data.to_vec();

        let mut attempt = 0u64;
        loop {
            attempt += 1;
            let gas_object_id = *self
                .sponsor_gas_objects
                .choose(&mut rand::thread_rng())
                .expect("sponsor gas pool is guaranteed non-empty");
            let tx = self
                .build_transaction_from_intent(&intent_bytes, &signature_bytes, gas_object_id)
                .await?;
            let signed = self.sponsor.sign_as_sender(tx);
            match self.upstream.broadcast_signed(&signed).await {
                Ok(result) => {
                    eprintln!("relay send success attempts={}", attempt);
                    return Ok(result);
                }
                Err(err) => {
                    let retryable = err.is_retryable_upstream();
                    eprintln!(
                        "relay send retry attempt={} retryable={} error={}",
                        attempt, retryable, err
                    );
                    if !retryable {
                        return Err(err);
                    }
                }
            }
        }
    }
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct RegistryBcs {
    id: [u8; 32],
    shards: VecMapBcs,
}

#[derive(serde::Deserialize)]
struct VecMapBcs {
    contents: Vec<EntryBcs>,
}

#[derive(serde::Deserialize)]
struct EntryBcs {
    key: u64,
    value: [u8; 32],
}
