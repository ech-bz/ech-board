use crate::captcha::CaptchaVerifier;
use crate::config;
use crate::error;
use crate::registry::RegistryCache;
use crate::shards::ShardsCache;
use crate::sponsor::SponsorService;
use crate::types::{Intent, SendForm, SendResponse};
use crate::upstream::UpstreamSender;
use futures::StreamExt;
use rand::seq::SliceRandom;
use std::str::FromStr;
use std::time::Duration;
use sui_rpc::proto::sui::rpc::v2::ListOwnedObjectsRequest;
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
    pub(crate) graphql_client: reqwest::Client,
    pub(crate) graphql_url: String,
    pub(crate) shards_cache: ShardsCache,
    pub(crate) registry_cache: RegistryCache,
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

        Ok(Self {
            captcha: CaptchaVerifier::new(client.clone(), cfg.captcha),
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
            graphql_client: client,
            graphql_url: cfg.graphql_url,
            shards_cache: ShardsCache::default(),
            registry_cache: RegistryCache::default(),
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
