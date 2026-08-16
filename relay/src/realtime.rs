use crate::app_state::AppState;
use crate::error::RelayError;
use crate::types::{Entity, EntityKind, EntityRoot, Feed};
use actix_web::{HttpResponse, get, web};
use futures::stream::{self, StreamExt};
use serde::Serialize;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;
use sui_rpc::field::{FieldMask, FieldMaskUtil};
use sui_rpc::proto::sui::rpc::v2::argument::ArgumentKind;
use sui_rpc::proto::sui::rpc::v2::changed_object::{IdOperation, OutputObjectState};
use sui_rpc::proto::sui::rpc::v2::command::Command as CommandKind;
use sui_rpc::proto::sui::rpc::v2::transaction_kind::Data;
use sui_rpc::proto::sui::rpc::v2::{Object, SubscribeCheckpointsRequest, SubscribeCheckpointsResponse, Transaction};
use sui_sdk_types::Address;
use sui_sdk_types::TypeTag;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Batch {
    pub(crate) forum: Option<Address>,
    pub(crate) board: Option<Address>,
    pub(crate) thread: Option<Address>,
    pub(crate) post: Option<Address>,
    pub(crate) envelopes: Vec<Envelope>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Envelope {
    pub(crate) kind: EntityKind,
    pub(crate) target: EntityRoot,
    pub(crate) event: String,
}

#[derive(serde::Deserialize)]
struct FeedEntryBytes {
    #[allow(dead_code)]
    id: Address,
    value: Vec<u8>,
}

fn classify(object_type: &str) -> Option<EntityKind> {
    if object_type.ends_with("::forum::Forum") {
        Some(EntityKind::Forum)
    } else if object_type.ends_with("::board::Board") {
        Some(EntityKind::Board)
    } else if object_type.ends_with("::thread::Thread") {
        Some(EntityKind::Thread)
    } else if object_type.ends_with("::post::Post") {
        Some(EntityKind::Post)
    } else {
        None
    }
}

fn entity_count(function: &str) -> usize {
    if function == "forum_apply_intent_uid" {
        1
    } else if function.starts_with("board_apply_intent_uid") {
        2
    } else if function.starts_with("board_apply_thread_intent_uid")
        || function.starts_with("thread_apply_intent_uid")
    {
        3
    } else if function.starts_with("forum_apply_post_intent_uid")
        || function.starts_with("board_apply_post_intent_uid")
        || function.starts_with("thread_apply_post_intent_uid")
        || function.starts_with("post_apply_intent_uid")
    {
        4
    } else {
        0
    }
}

struct CallEntities {
    forum: Option<Address>,
    board: Option<Address>,
    thread: Option<Address>,
    post: Option<Address>,
}

fn parse_call_entities(tx: &Transaction) -> Option<CallEntities> {
    let data = tx.kind.as_ref()?.data.as_ref()?;
    let Data::ProgrammableTransaction(pt) = data else {
        return None;
    };
    let CommandKind::MoveCall(call) = pt.commands.first()?.command.as_ref()? else {
        return None;
    };
    let count = entity_count(call.function.as_deref()?);
    if count == 0 {
        return None;
    }
    let mut ids: Vec<Address> = Vec::new();
    for arg in &call.arguments {
        if arg.kind != Some(ArgumentKind::Input as i32) {
            continue;
        }
        let Some(idx) = arg.input else { continue };
        let Some(input) = pt.inputs.get(idx as usize) else { continue };
        let Some(id_str) = input.object_id.as_deref() else { continue };
        ids.push(Address::from_str(id_str).ok()?);
    }
    if ids.len() < count {
        return None;
    }
    let tail = &ids[ids.len() - count..];
    Some(CallEntities {
        forum: tail.first().copied(),
        board: tail.get(1).copied(),
        thread: tail.get(2).copied(),
        post: tail.get(3).copied(),
    })
}

pub(crate) async fn run_observer(url: String, tx: broadcast::Sender<Batch>) {
    let mut last_seen: HashMap<Address, u64> = HashMap::new();
    loop {
        if let Err(e) = observe_once(&url, &tx, &mut last_seen).await {
            eprintln!("realtime observer: {e}; reconnecting");
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn observe_once(
    url: &str,
    tx: &broadcast::Sender<Batch>,
    last_seen: &mut HashMap<Address, u64>,
) -> Result<(), RelayError> {
    let mut client = sui_rpc::Client::new(url)
        .map_err(|e| RelayError::Internal(format!("observer client init: {e}")))?;
    let mut sub = client.subscription_client();
    let mut request = SubscribeCheckpointsRequest::default();
    request.read_mask = Some(FieldMask::from_str(
        "transactions.effects.status,transactions.effects.changed_objects,transactions.transaction.kind,objects.objects.object_id,objects.objects.object_type,objects.objects.contents",
    ));
    let stream = sub
        .subscribe_checkpoints(request)
        .await
        .map_err(|e| RelayError::Internal(format!("observer subscribe: {e}")))?
        .into_inner();
    let mut stream = Box::pin(stream);

    while let Some(response) = stream.next().await {
        let response: SubscribeCheckpointsResponse = response
            .map_err(|e| RelayError::Internal(format!("observer stream: {e}")))?;
        let Some(checkpoint) = response.checkpoint else {
            continue;
        };
        let objects = checkpoint.objects;
        let transactions = checkpoint.transactions;
        let obj_map: HashMap<Address, &Object> = objects
            .as_ref()
            .map(|set| {
                set.objects
                    .iter()
                    .filter_map(|o| {
                        let id = o.object_id.as_deref().and_then(|s| Address::from_str(s).ok())?;
                        Some((id, o))
                    })
                    .collect()
            })
            .unwrap_or_default();

        eprintln!(
            "realtime observer: txs={} objects={}",
            transactions.len(),
            obj_map.len(),
        );

        for tx_data in transactions {
            let Some(effects) = tx_data.effects else { continue };
            let success = effects
                .status
                .as_ref()
                .and_then(|s| s.success)
                .unwrap_or(false);
            if !success {
                continue;
            }

            let Some(call) = tx_data.transaction.as_ref().and_then(parse_call_entities) else {
                continue;
            };

            let mut roots: Vec<(Address, EntityKind, Address, u64, u16, bool)> = Vec::new();
            for co in &effects.changed_objects {
                let op = co.id_operation.unwrap_or(IdOperation::Unknown as i32);
                let out = co.output_state.unwrap_or(OutputObjectState::Unknown as i32);
                let created = op == IdOperation::Created as i32;
                let written = out == OutputObjectState::ObjectWrite as i32;
                if !created && !written {
                    continue;
                }
                let Some(oid) = co.object_id.as_deref() else {
                    continue;
                };
                let Ok(id) = Address::from_str(oid) else {
                    continue;
                };
                let Some(obj) = obj_map.get(&id) else {
                    continue;
                };
                let Some(kind) = obj.object_type.as_deref().and_then(classify) else {
                    continue;
                };
                let object: EntityRoot = match obj.contents().deserialize() {
                    Ok(object) => object,
                    Err(_) => continue,
                };
                if object.entity.version == 0 || object.entity.version > 4 {
                    continue;
                }
                roots.push((
                    id,
                    kind,
                    object.entity.feed.id,
                    object.entity.feed.counter,
                    object.entity.version,
                    object.genesis,
                ));
            }
            if roots.is_empty() {
                continue;
            }

            let mut forum = call.forum;
            let mut board = call.board;
            let mut thread = call.thread;
            let mut post = call.post;
            for &(id, kind, _, _, _, _) in &roots {
                match kind {
                    EntityKind::Forum => forum = forum.or(Some(id)),
                    EntityKind::Board => board = board.or(Some(id)),
                    EntityKind::Thread => thread = thread.or(Some(id)),
                    EntityKind::Post => post = post.or(Some(id)),
                }
            }

            let mut envelopes: Vec<Envelope> = Vec::new();
            for &(id, kind, feed, counter, version, genesis) in &roots {
                let prev = last_seen.get(&id).copied();
                let (start, end) = match prev {
                    None => (1, counter + 1),
                    Some(p) if counter > p => (p + 1, counter + 1),
                    _ => continue,
                };
                last_seen.insert(id, counter);

                for counter in start..end {
                    let entry_id = feed.derive_object_id(&TypeTag::U64, &counter.to_le_bytes());
                    let Some(obj) = obj_map.get(&entry_id) else {
                        eprintln!(
                            "realtime observer: feed entry {entry_id} (feed {feed} counter {counter}) missing from checkpoint objects",
                        );
                        continue;
                    };
                    let entry: FeedEntryBytes = match obj.contents().deserialize() {
                        Ok(entry) => entry,
                        Err(e) => {
                            eprintln!("realtime observer: feed entry decode {entry_id}: {e}");
                            continue;
                        }
                    };
                    envelopes.push(Envelope {
                        kind,
                        target: EntityRoot {
                            id,
                            entity: Entity {
                                feed: Feed { id: feed, counter },
                                version,
                            },
                            genesis,
                        },
                        event: hex::encode(&entry.value),
                    });
                }
            }

            if envelopes.is_empty() {
                continue;
            }

            let _ = tx.send(Batch {
                forum,
                board,
                thread,
                post,
                envelopes,
            });
        }
    }
    Ok(())
}

enum ScopeKind {
    Forum,
    Board,
    Thread,
}

fn scoped(batch: &Batch, scope: &ScopeKind, id: &str) -> bool {
    match scope {
        ScopeKind::Forum => {
            batch.forum.map(|a| a.to_string()).as_deref() == Some(id) && batch.thread.is_none()
        }
        ScopeKind::Board => batch.board.map(|a| a.to_string()).as_deref() == Some(id),
        ScopeKind::Thread => batch.thread.map(|a| a.to_string()).as_deref() == Some(id),
    }
}

async fn sse_scope(
    state: web::Data<AppState>,
    scope: ScopeKind,
    id: String,
) -> Result<HttpResponse, actix_web::Error> {
    let rx = state.sse_tx.subscribe();
    let events = stream::unfold((rx, scope, id), |(mut rx, scope, id)| async move {
        loop {
            match rx.recv().await {
                Ok(ev) => {
                    if !scoped(&ev, &scope, &id) {
                        continue;
                    }
                    let line = serde_json::to_string(&ev).unwrap_or_default();
                    let item = web::Bytes::from(format!("data: {line}\n\n"));
                    return Some((Ok::<web::Bytes, actix_web::Error>(item), (rx, scope, id)));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    });

    Ok(HttpResponse::Ok()
        .content_type("text/event-stream")
        .insert_header(("Cache-Control", "no-cache"))
        .streaming(events))
}

#[get("/sse/forum/{id}")]
pub(crate) async fn sse_forum_handler(
    state: web::Data<AppState>,
    path: web::Path<Address>,
) -> Result<HttpResponse, actix_web::Error> {
    sse_scope(state, ScopeKind::Forum, path.into_inner().to_string()).await
}

#[get("/sse/board/{id}")]
pub(crate) async fn sse_board_handler(
    state: web::Data<AppState>,
    path: web::Path<Address>,
) -> Result<HttpResponse, actix_web::Error> {
    sse_scope(state, ScopeKind::Board, path.into_inner().to_string()).await
}

#[get("/sse/thread/{id}")]
pub(crate) async fn sse_thread_handler(
    state: web::Data<AppState>,
    path: web::Path<Address>,
) -> Result<HttpResponse, actix_web::Error> {
    sse_scope(state, ScopeKind::Thread, path.into_inner().to_string()).await
}
