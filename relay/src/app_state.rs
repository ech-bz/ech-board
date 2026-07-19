use crate::captcha::CaptchaVerifier;
use crate::config;
use crate::handlers::ForumObject;
use crate::seaweed::SeaweedClient;
use crate::sponsor::SponsorService;
use crate::upstream::UpstreamSender;
use aws_sdk_kms::Client as KmsClient;
use std::str::FromStr;
use std::time::Duration;
use sui_sdk_types::Address;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) captcha: CaptchaVerifier,
    pub(crate) upstream: UpstreamSender,
    pub(crate) sponsor: SponsorService,
    pub(crate) sponsor_gas_objects: Vec<Address>,
    pub(crate) sponsor_gas_budget: u64,
    pub(crate) sponsor_gas_price: u64,
    pub(crate) forum: ForumObject,
    pub(crate) package_id: Address,
    pub(crate) seaweed: SeaweedClient,
    pub(crate) kms: KmsClient,
    pub(crate) kms_hmac: String,
    pub(crate) kms_moderator: String,
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

        let package_id = Address::from_str(&cfg.forum_package_id)
            .map_err(|e| std::io::Error::other(format!("invalid forum_package_id: {e}")))?;
        let forum_registry = Address::from_str(&cfg.forum_registry)
            .map_err(|e| std::io::Error::other(format!("invalid forum_registry: {e}")))?;

        let upstream = UpstreamSender::new(
            sui_client,
            cfg.upstream.submit_url.clone(),
            cfg.upstream.request_timeout_ms,
        );

        let sponsor_gas_objects = upstream
            .list_gas_objects(sponsor.sponsor_address())
            .await
            .map_err(std::io::Error::other)?;

        let forum = upstream
            .fetch_objects([forum_registry])
            .await
            .map_err(|e| std::io::Error::other(format!("failed to fetch forum object: {e}")))?
            .into_iter()
            .flatten()
            .next()
            .ok_or_else(|| std::io::Error::other("forum object not found"))?
            .contents()
            .deserialize::<ForumObject>()
            .map_err(|e| std::io::Error::other(format!("bcs decode ForumObject: {e}")))?;

        Ok(Self {
            captcha: CaptchaVerifier::new(client.clone(), cfg.captcha),
            upstream,
            sponsor,
            sponsor_gas_objects,
            sponsor_gas_budget: sponsor_cfg.gas_budget,
            sponsor_gas_price: sponsor_cfg.gas_price,
            forum,
            package_id,
            seaweed: SeaweedClient::new(cfg.seaweed_filer_url, cfg.seaweed_jwt_signing_key),
            kms: KmsClient::from_conf(
                aws_sdk_kms::Config::builder()
                    .region(aws_sdk_kms::config::Region::new(cfg.kms_region.clone()))
                    .credentials_provider(
                        aws_sdk_kms::config::Credentials::new(
                            cfg.aws_access_key_id.clone(),
                            cfg.aws_secret_access_key.clone(),
                            None,
                            None,
                            "ech-board-relay",
                        ),
                    )
                    .build(),
            ),
            kms_hmac: cfg.kms_hmac,
            kms_moderator: cfg.kms_moderator,
        })
    }
}
