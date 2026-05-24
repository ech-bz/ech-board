use crate::captcha::CaptchaVerifier;
use crate::config;
use crate::error;
use crate::sponsor::SponsorService;
use crate::upstream::UpstreamSender;
use crate::{RelayIntent, SendForm, SendResponse};
use futures::StreamExt;
use rand::Rng;
use std::str::FromStr;
use std::time::Duration;
use sui_rpc::proto::sui::rpc::v2::ListOwnedObjectsRequest;
use sui_sdk_types::Address;
use sui_sdk_types::GasPayment;
use sui_sdk_types::Input;
use sui_sdk_types::ObjectReference;
use sui_sdk_types::StructTag;
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
        let sponsor_gas_objects =
            Self::fetch_sponsor_gas_objects(&sui_client, sponsor.sponsor_address())
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
        })
    }

    async fn fetch_sponsor_gas_objects(
        client: &sui_rpc::Client,
        sponsor: Address,
    ) -> Result<Vec<Address>, error::RelayError> {
        let request = ListOwnedObjectsRequest::default()
            .with_owner(&sponsor)
            .with_object_type(&StructTag::gas_coin());
        let mut stream = Box::pin(client.list_owned_objects(request));
        let mut ids = Vec::new();
        while let Some(object_res) = stream.next().await {
            let object = object_res.map_err(|e| {
                error::RelayError::SponsorBuild(format!("failed to list sponsor coins: {e}"))
            })?;
            let id = Address::from_str(object.object_id()).map_err(|e| {
                error::RelayError::SponsorBuild(format!("invalid sponsor coin id: {e}"))
            })?;
            ids.push(id);
        }
        if ids.is_empty() {
            return Err(error::RelayError::SponsorBuild(
                "sponsor has no SUI gas objects".to_string(),
            ));
        }
        eprintln!("relay sponsor gas pool loaded size={}", ids.len());
        Ok(ids)
    }

    async fn build_transaction_from_intent(
        &self,
        intent: &RelayIntent,
        sponsor_gas_object_id: Address,
    ) -> Result<Transaction, error::RelayError> {
        let mut kind = intent.transaction_kind.clone();
        if let TransactionKind::ProgrammableTransaction(pt) = &mut kind {
            for input in &mut pt.inputs {
                match input {
                    Input::ImmutableOrOwned(obj_ref) => {
                        let latest = self.upstream.latest_object_ref(obj_ref.object_id()).await?;
                        *obj_ref = latest;
                    }
                    Input::Receiving(obj_ref) => {
                        let latest = self.upstream.latest_object_ref(obj_ref.object_id()).await?;
                        *obj_ref = latest;
                    }
                    Input::Pure(_) | Input::Shared(_) | Input::FundsWithdrawal(_) => {}
                    _ => {}
                }
            }
        }
        let gas_ref: ObjectReference = self
            .upstream
            .latest_object_ref(&sponsor_gas_object_id)
            .await?;
        Ok(Transaction {
            kind,
            sender: self.sponsor.sponsor_address(),
            gas_payment: GasPayment {
                objects: vec![gas_ref],
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
        let intent: RelayIntent = bcs::from_bytes(&intent_bytes).map_err(|e| {
            error::RelayError::SponsorBuild(format!("failed to decode intent: {e}"))
        })?;

        let mut attempt = 0u64;
        loop {
            attempt += 1;
            let gas_object_id = self.sponsor_gas_objects
                [rand::thread_rng().gen_range(0..self.sponsor_gas_objects.len())];
            let tx = self
                .build_transaction_from_intent(&intent, gas_object_id)
                .await?;
            let signed = self.sponsor.sign_as_sender(tx);
            match self.upstream.broadcast_signed(&signed).await {
                Ok(result) => {
                    eprintln!("relay send success attempts={}", attempt);
                    return Ok(SendResponse {
                        accepted_by: result.accepted_by,
                        digest: result.digest,
                    });
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
