use std::collections::{HashMap, HashSet};

use crate::app_state::AppState;
use crate::cache::CACHE_NS;
use crate::error::RelayError;
use serde::Serialize;
use sui_sdk_types::Address;

use super::fetch_content;
use super::fetch_media_meta;
use super::{Moderators, PostObject, ThreadObject, list_mods, load_posts_and_board, load_thread};
use crate::types::{ContentKind, MediaMeta};

#[derive(Serialize)]
pub(crate) struct ThreadView {
    pub(crate) thread: ThreadObject,
    pub(crate) posts: Vec<PostObject>,
    pub(crate) text: HashMap<Address, Vec<u8>>,
    pub(crate) plain_text: HashMap<Address, Vec<u8>>,
    pub(crate) media_meta: HashMap<Address, MediaMeta>,
    pub(crate) moderators: Moderators,
}

pub(crate) async fn fetch(state: &AppState, thread_uid: Address) -> Result<Vec<u8>, RelayError> {
    let thread = load_thread(&state.upstream, thread_uid).await?;
    if thread.projection.deleted() {
        return Err(RelayError::NotFound("thread deleted".into()));
    }
    let pcounter = thread.projection.posts().counter;
    let rgen = state
        .cache
        .gen_get(&format!("gen:thread:{thread_uid}"))
        .await;
    let key = format!("{CACHE_NS}:thread:{thread_uid}:{pcounter}:{rgen}");
    state
        .cache
        .get_or_build(key, async {
            let post_ids = state
                .upstream
                .fetch_feed(
                    thread.projection.posts().id,
                    1,
                    thread.projection.posts().counter + 1,
                )
                .await?;
            let (mut posts, board) =
                load_posts_and_board(&state.upstream, post_ids, thread.projection.board()).await?;

            if board.projection.deleted() {
                return Err(RelayError::NotFound("board deleted".into()));
            }

            posts.sort_by_key(|p| p.projection.number());

            let text_hashes: HashSet<Address> = posts
                .iter()
                .filter(|p| !p.projection.deleted())
                .filter_map(|p| p.projection.text_hash())
                .collect();

            let mut plain_text_hashes = HashSet::new();
            if let Some(h) = thread.projection.topic_hash() {
                plain_text_hashes.insert(h);
            }
            for post in posts.iter().filter(|p| !p.projection.deleted()) {
                if let Some(h) = post.projection.name_hash() {
                    plain_text_hashes.insert(h);
                }
            }

            let media_hashes: HashSet<Address> = posts
                .iter()
                .filter(|p| !p.projection.deleted())
                .flat_map(|p| p.projection.media_hashes().iter().copied())
                .collect();

            let (text, plain_text, media_meta) = tokio::join!(
                fetch_content(&state.seaweed, ContentKind::Text, text_hashes),
                fetch_content(&state.seaweed, ContentKind::PlainText, plain_text_hashes),
                fetch_media_meta(&state.seaweed, media_hashes),
            );

            let (forum_mods, board_mods, thread_mods) = tokio::join!(
                list_mods(&state.upstream, state.forum.projection.mods().id),
                list_mods(&state.upstream, board.projection.mods().id),
                list_mods(&state.upstream, thread.projection.mods().id),
            );
            let moderators = Moderators {
                forum_admin: Some(state.forum.projection.admin()),
                forum_mods: forum_mods?,
                board_mods: board_mods?,
                thread_mods: thread_mods?,
                thread_admin: *thread.projection.admin(),
            };

            let response = ThreadView {
                thread,
                posts,
                text,
                plain_text,
                media_meta,
                moderators,
            };

            bcs::to_bytes(&response)
                .map_err(|e| RelayError::Internal(format!("bcs encode ThreadView: {e}")))
        })
        .await
}
