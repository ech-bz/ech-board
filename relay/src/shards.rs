use crate::error::RelayError;
use graphql_client::GraphQLQuery;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, RwLock};

type SuiAddress = String;
type MoveTypeSignature = serde_json::Value;
type JSON = serde_json::Value;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "queries/schema.graphql",
    query_path = "queries/get_shards.graphql",
    response_derives = "Debug"
)]
#[allow(dead_code)]
struct GetShards;

#[derive(Debug, Default, Clone)]
pub(crate) struct ShardsCache {
    inner: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl ShardsCache {
    pub(crate) fn get(&self, package_id: &str) -> Option<Vec<u8>> {
        self.inner.read().unwrap().get(package_id).cloned()
    }

    pub(crate) fn set(&self, package_id: String, data: Vec<u8>) {
        self.inner.write().unwrap().insert(package_id, data);
    }
}

const PAGE_SIZE: i64 = 128;

pub(crate) async fn fetch_shards(
    client: &reqwest::Client,
    graphql_url: &str,
    package_id: &str,
) -> Result<Vec<Vec<u8>>, RelayError> {
    let mut shards: BTreeMap<u64, Vec<u8>> = BTreeMap::new();
    let mut cursor: Option<String> = None;

    loop {
        let response = client
            .post(graphql_url)
            .json(&GetShards::build_query(get_shards::Variables {
                package_id: package_id.to_string(),
                first: Some(PAGE_SIZE),
                after: cursor.clone(),
            }))
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

        let page = response
            .json::<graphql_client::Response<get_shards::ResponseData>>()
            .await
            .map_err(|e| RelayError::GraphqlRequest(e))?
            .data
            .ok_or_else(|| RelayError::GraphqlResponse("graphql response has no data".into()))?
            .package
            .and_then(|p| p.previous_transaction)
            .and_then(|t| t.effects)
            .and_then(|e| e.object_changes);

        let (nodes, page_info) = match page {
            Some(p) => (p.nodes, p.page_info),
            None => break,
        };

        for node in nodes {
            let contents = match node
                .output_state
                .as_ref()
                .and_then(|os| os.as_move_object.as_ref())
                .and_then(|mo| mo.contents.as_ref())
            {
                Some(c) => c,
                None => continue,
            };

            let is_shard = contents
                .type_
                .as_ref()
                .and_then(|t| t.signature.get("datatype"))
                .and_then(|dt| {
                    let module = dt.get("module")?.as_str()?;
                    let type_ = dt.get("type")?.as_str()?;
                    (module == "nonce" && type_ == "NonceGateShard").then_some(())
                })
                .is_some();
            if !is_shard {
                continue;
            }

            let index = contents
                .extract
                .as_ref()
                .and_then(|e| e.json.as_ref())
                .and_then(|j| j.as_str())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);

            shards.insert(
                index,
                hex::decode(node.address.trim_start_matches("0x")).map_err(|e| {
                    RelayError::GraphqlResponse(format!("invalid hex address: {e}"))
                })?,
            );
        }

        if !page_info.has_next_page {
            break;
        }
        cursor = page_info.end_cursor;
    }

    Ok(shards.into_values().collect())
}

pub(crate) async fn get_shards_cached(
    client: &reqwest::Client,
    cache: &ShardsCache,
    graphql_url: &str,
    package_id: &str,
) -> Result<Vec<u8>, RelayError> {
    if let Some(cached) = cache.get(package_id) {
        return Ok(cached);
    }

    let shards = fetch_shards(client, graphql_url, package_id).await?;
    let bcs_bytes = bcs::to_bytes(&shards)
        .map_err(|e| RelayError::GraphqlResponse(format!("bcs encode: {e}")))?;
    cache.set(package_id.to_string(), bcs_bytes.clone());
    Ok(bcs_bytes)
}
