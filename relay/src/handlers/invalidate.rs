use crate::app_state::AppState;
use crate::cache::{CACHE_NS, Invalidation};
use crate::handlers::send::split_event;
use crate::types::IntentV2;
use std::future::Future;
use std::pin::Pin;
use sui_sdk_types::Address;

fn object_indexes(fn_name: &str) -> (Option<usize>, Option<usize>, Option<usize>, Option<usize>) {
    match fn_name {
        "forum_apply_intent_uid" => (Some(2), None, None, None),
        "forum_apply_post_intent_uid" => (Some(2), Some(3), Some(4), Some(5)),
        "board_apply_intent_uid"
        | "board_apply_intent_uid_geo"
        | "board_apply_intent_uid_tripcode"
        | "board_apply_intent_uid_geo_tripcode"
        | "board_apply_intent_uid_captcha"
        | "board_apply_intent_uid_geo_captcha"
        | "board_apply_intent_uid_tripcode_captcha"
        | "board_apply_intent_uid_geo_tripcode_captcha" => (None, Some(3), None, None),
        "board_apply_thread_intent_uid"
        | "board_apply_thread_intent_uid_geo"
        | "board_apply_thread_intent_uid_tripcode"
        | "board_apply_thread_intent_uid_geo_tripcode"
        | "board_apply_thread_intent_uid_captcha"
        | "board_apply_thread_intent_uid_geo_captcha"
        | "board_apply_thread_intent_uid_tripcode_captcha"
        | "board_apply_thread_intent_uid_geo_tripcode_captcha" => (None, Some(3), Some(4), None),
        "board_apply_post_intent_uid" => (None, Some(3), Some(4), Some(5)),
        "thread_apply_intent_uid" => (None, Some(2), Some(3), None),
        "thread_apply_post_intent_uid" => (None, Some(3), Some(4), Some(5)),
        "post_apply_intent_uid" | "post_apply_intent_uid_ip32" => (None, Some(3), Some(4), Some(5)),
        _ => (None, None, None, None),
    }
}

fn obj_at(objects: &[crate::types::IntentObject], idx: Option<usize>) -> Option<Address> {
    idx.and_then(|i| objects.get(i)).map(|o| o.id)
}

#[cfg(test)]
mod tests {
    use super::object_indexes;

    #[test]
    fn object_indexes_layout() {
        let cases = [
            ("forum_apply_intent_uid", Some(2), None, None, None),
            ("forum_apply_post_intent_uid", Some(2), Some(3), Some(4), Some(5)),
            ("board_apply_intent_uid_captcha", None, Some(3), None, None),
            ("board_apply_thread_intent_uid_captcha", None, Some(3), Some(4), None),
            ("board_apply_post_intent_uid", None, Some(3), Some(4), Some(5)),
            ("thread_apply_intent_uid", None, Some(2), Some(3), None),
            ("thread_apply_post_intent_uid", None, Some(3), Some(4), Some(5)),
            ("post_apply_intent_uid", None, Some(3), Some(4), Some(5)),
            ("post_apply_intent_uid_ip32", None, Some(3), Some(4), Some(5)),
        ];
        for (f, fo, bo, th, po) in cases {
            assert_eq!(object_indexes(f), (fo, bo, th, po), "fn: {f}");
        }
    }
}

pub(crate) async fn apply(state: &AppState, intents: &[(IntentV2, Vec<u8>)]) {
    let mut flush = false;
    let mut gens: Vec<String> = Vec::new();
    let mut dels: Vec<String> = Vec::new();
    let mut patterns: Vec<String> = Vec::new();
    let mut scopes: Vec<String> = Vec::new();

    for (intent, _) in intents {
        let Ok((event, _)) = split_event(&intent.payload) else {
            continue;
        };
        let (forum_idx, board_idx, thread_idx, post_idx) = object_indexes(&intent.function);
        let forum = obj_at(&intent.objects, forum_idx);
        let board = obj_at(&intent.objects, board_idx);
        let thread = obj_at(&intent.objects, thread_idx);
        let post = obj_at(&intent.objects, post_idx);

        match event {
            "new_post_v2" | "new_post_migrate_v2" | "new_thread_v2" | "new_thread_migrate_v2" => {
                dels.push(format!("{CACHE_NS}:forum"));
                scopes.push(format!("{CACHE_NS}:forum"));
            }
            "set_reaction_v2" | "vote_v2" => {
                if let (Some(b), Some(t), Some(p)) = (board, thread, post) {
                    gens.push(format!("gen:thread:{t}"));
                    gens.push(format!("gen:board:{b}"));
                    dels.push(format!("{CACHE_NS}:post:{p}"));
                    dels.push(format!("{CACHE_NS}:reactions:{p}:{}", intent.public_key));
                    scopes.push(format!("{CACHE_NS}:post:{p}"));
                    scopes.push(format!("{CACHE_NS}:reactions:{p}:{}", intent.public_key));
                }
            }
            "set_topic" => {
                if let Some(t) = thread {
                    patterns.push(format!("{CACHE_NS}:thread:{t}:*"));
                    scopes.push(format!("{CACHE_NS}:thread:{t}"));
                }
                if let Some(b) = board {
                    patterns.push(format!("{CACHE_NS}:board:{b}:*"));
                    scopes.push(format!("{CACHE_NS}:board:{b}"));
                }
            }
            "post_set_text" | "post_set_deleted" | "ban_media" | "unban_media" => {
                if let (Some(b), Some(t), Some(p)) = (board, thread, post) {
                    dels.push(format!("{CACHE_NS}:post:{p}"));
                    patterns.push(format!("{CACHE_NS}:thread:{t}:*"));
                    patterns.push(format!("{CACHE_NS}:board:{b}:*"));
                    scopes.push(format!("{CACHE_NS}:post:{p}"));
                    scopes.push(format!("{CACHE_NS}:thread:{t}"));
                    scopes.push(format!("{CACHE_NS}:board:{b}"));
                }
            }
            "ban" | "unban" => {
                let level = match intent.function.as_str() {
                    "forum_apply_post_intent_uid" => forum,
                    "board_apply_post_intent_uid" => board,
                    "thread_apply_post_intent_uid" => thread,
                    _ => None,
                };
                if let Some(l) = level {
                    patterns.push(format!("{CACHE_NS}:bans:{l}:*"));
                    scopes.push(format!("{CACHE_NS}:bans:{l}"));
                }
            }
            "set_closed"
            | "set_admin"
            | "add_moderator"
            | "del_moderator"
            | "set_deleted"
            | "upgrade"
            | "new_board"
            | "set_description"
            | "set_pinned"
            | "set_max_media"
            | "set_bump_limit"
            | "set_ignore_forum_bans"
            | "set_reactions"
            | "set_timestamp_precision" => {
                flush = true;
            }
            _ => {}
        }
    }

    if flush {
        state.cache.l1_flush();
        if let Err(e) = state.cache.l2_flush().await {
            eprintln!("cache flush: {e}");
        }
        let _ = state
            .cache
            .publish(&Invalidation {
                flush: true,
                scopes: Vec::new(),
            })
            .await;
    } else {
        let mut futs: Vec<Pin<Box<dyn Future<Output = ()> + Send + '_>>> = Vec::new();
        for g in &gens {
            let key = g.clone();
            futs.push(Box::pin(async move {
                if let Err(e) = state.cache.l2_incr(&key).await {
                    eprintln!("cache incr {key}: {e}");
                }
            }));
        }
        if !dels.is_empty() {
            let keys = dels.clone();
            futs.push(Box::pin(async move {
                if let Err(e) = state.cache.l2_del(&keys).await {
                    eprintln!("cache del {keys:?}: {e}");
                }
            }));
        }
        for p in &patterns {
            let pat = p.clone();
            futs.push(Box::pin(async move {
                if let Err(e) = state.cache.l2_pattern_del(&pat).await {
                    eprintln!("cache pattern del {pat}: {e}");
                }
            }));
        }
        for scope in &scopes {
            state.cache.l1_invalidate_prefix(scope);
        }
        if !scopes.is_empty() {
            let inv = Invalidation {
                flush: false,
                scopes,
            };
            futs.push(Box::pin(async move {
                if let Err(e) = state.cache.publish(&inv).await {
                    eprintln!("cache publish: {e}");
                }
            }));
        }
        futures::future::join_all(futs).await;
    }
}
