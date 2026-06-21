use crate::error::RelayError;
use graphql_client::GraphQLQuery;
use std::sync::{Arc, RwLock};

type SuiAddress = String;
type MoveTypeSignature = serde_json::Value;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "queries/schema.graphql",
    query_path = "queries/get_board_slug_registry.graphql",
    response_derives = "Debug"
)]
#[allow(dead_code)]
struct GetBoardSlugRegistry;

#[derive(Debug, Default, Clone)]
pub(crate) struct RegistryCache {
    inner: Arc<RwLock<Option<Vec<u8>>>>,
}

impl RegistryCache {
    pub(crate) fn get(&self) -> Option<Vec<u8>> {
        self.inner.read().unwrap().clone()
    }

    pub(crate) fn set(&self, data: Vec<u8>) {
        *self.inner.write().unwrap() = Some(data);
    }
}

pub(crate) async fn fetch_registry(
    client: &reqwest::Client,
    graphql_url: &str,
    package_id: &str,
) -> Result<Vec<u8>, RelayError> {
    let response = client
        .post(graphql_url)
        .json(&GetBoardSlugRegistry::build_query(
            get_board_slug_registry::Variables {
                package_id: package_id.to_string(),
            },
        ))
        .send()
        .await
        .map_err(|e| RelayError::GraphqlRequest(e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .map_err(|e| RelayError::GraphqlRequest(e))?;
        return Err(RelayError::GraphqlResponse(format!(
            "HTTP {status}: {body}"
        )));
    }

    let nodes = response
        .json::<graphql_client::Response<get_board_slug_registry::ResponseData>>()
        .await
        .map_err(|e| RelayError::GraphqlRequest(e))?
        .data
        .ok_or_else(|| RelayError::GraphqlResponse("graphql response has no data".into()))?
        .package
        .and_then(|p| p.previous_transaction)
        .and_then(|t| t.effects)
        .and_then(|e| e.object_changes)
        .map(|oc| oc.nodes)
        .unwrap_or_default();

    for node in nodes {
        let is_registry = node
            .output_state
            .as_ref()
            .and_then(|os| os.as_move_object.as_ref())
            .and_then(|mo| mo.contents.as_ref())
            .and_then(|c| c.type_.as_ref())
            .and_then(|t| t.signature.get("datatype"))
            .and_then(|dt| {
                let module = dt.get("module")?.as_str()?;
                let type_ = dt.get("type")?.as_str()?;
                (module == "forum" && type_ == "BoardSlugRegistry").then_some(())
            })
            .is_some();
        if is_registry {
            return hex::decode(node.address.trim_start_matches("0x"))
                .map_err(|e| RelayError::GraphqlResponse(format!("invalid hex address: {e}")));
        }
    }

    Err(RelayError::GraphqlResponse(
        "BoardSlugRegistry not found".into(),
    ))
}

pub(crate) async fn get_registry_cached(
    client: &reqwest::Client,
    cache: &RegistryCache,
    graphql_url: &str,
    package_id: &str,
) -> Result<Vec<u8>, RelayError> {
    if let Some(cached) = cache.get() {
        return Ok(cached);
    }

    let address = fetch_registry(client, graphql_url, package_id).await?;
    cache.set(address.clone());
    Ok(address)
}
