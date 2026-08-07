module forum::board_apply;

use forum::bans;
use forum::board::{Self, Board};
use forum::empty;
use forum::entity;
use forum::error;
use forum::event;
use forum::forum::{Self, Forum};
use forum::post::{Self, Post};
use forum::responses;
use forum::sender::{Self, Sender};
use forum::thread::{Self, Thread};
use sui::bcs;
use sui::clock::{Self, Clock};
use sui::vec_set;

public(package) fun apply(
    self: &mut Board,
    ctx: &mut TxContext,
    clock: &Clock,
    forum: &Forum,
    event: vector<u8>,
) {
    self.push(event);

    assert!(forum.boards().contains(*self.slug()), error::cross_reference_mismatch());

    let mut event = bcs::new(event);
    event::peel_version(&mut event);
    let responses = responses::peel(&mut event);
    let sender = sender::peel(&mut event);
    let addr = sender.addr();
    let tag = event.peel_vec_u8();
    if (tag != b"upgrade" && !self.check_version()) {
        abort error::entity_version_unsupported()
    };
    match (tag) {
        b"upgrade" => {
            self.do_upgrade(ctx);
        },
        b"add_moderator" => {
            let moderator = event.peel_address();
            assert!(
                addr == forum.admin()
                    || forum.mods().contains(addr),
                error::not_authorized(),
            );
            self.mods_mut().add(moderator, empty::new());
        },
        b"del_moderator" => {
            let moderator = event.peel_address();
            assert!(
                addr == forum.admin()
                    || forum.mods().contains(addr),
                error::not_authorized(),
            );
            self.mods_mut().remove(moderator);
        },
        b"set_max_media" => {
            let max_media = event.peel_u64();
            assert!(
                addr == forum.admin()
                    || forum.mods().contains(addr),
                error::not_authorized(),
            );
            *self.max_media_mut() = max_media;
        },
        b"set_bump_limit" => {
            let bump_limit = event.peel_u64();
            assert!(
                addr == forum.admin()
                    || forum.mods().contains(addr),
                error::not_authorized(),
            );
            *self.bump_limit_mut() = bump_limit;
        },
        b"set_closed" => {
            let closed = event.peel_bool();
            assert!(
                addr == forum.admin()
                    || forum.mods().contains(addr),
                error::not_authorized(),
            );
            assert!(*self.closed() != closed);
            assert!(!*self.deleted());
            *self.closed_mut() = closed;
        },
        b"set_deleted" => {
            let deleted = event.peel_bool();
            assert!(
                addr == forum.admin()
                    || forum.mods().contains(addr),
                error::not_authorized(),
            );
            assert!(*self.deleted() != deleted);
            assert!(*self.closed());
            *self.deleted_mut() = deleted;
        },
        b"new_thread_v2" => {
            let topic_hash = event.peel_option!(|b| b.peel_u256());
            let text_hash = event.peel_option!(|b| b.peel_u256());
            let media_hashes = event.peel_vec!(|b| b.peel_u256());
            let name_hash = event.peel_option!(|b| b.peel_u256());
            let vote_keys = event.peel_vec!(|b| b.peel_u256());
            let multi_vote = event.peel_bool();
            assert!(
                self.max_media() == 0 || media_hashes.length() > 0,
                error::post_requires_media(),
            );
            let number = self.posts().length() + 1;
            let mut thread = thread::new(
                ctx,
                copy responses,
                sender,
                self.id(),
                number,
                topic_hash,
            );
            self.threads_mut().add(number, thread.id());
            let new_post = board::new_post_v2(
                responses,
                sender,
                thread.id(),
                text_hash,
                media_hashes,
                name_hash,
                vote_keys,
                multi_vote,
            );
            self.apply_thread(ctx, clock, forum, &mut thread, new_post);
            thread.share();
        },
        b"new_thread_migrate_v2" => {
            let timestamp_ms = event.peel_u64();
            let topic_hash = event.peel_option!(|b| b.peel_u256());
            let text_hash = event.peel_option!(|b| b.peel_u256());
            let media_hashes = event.peel_vec!(|b| b.peel_u256());
            let name_hash = event.peel_option!(|b| b.peel_u256());
            let vote_keys = event.peel_vec!(|b| b.peel_u256());
            let multi_vote = event.peel_bool();
            assert!(
                self.max_media() == 0 || media_hashes.length() > 0,
                error::post_requires_media(),
            );
            let number = self.posts().length() + 1;
            let mut thread = thread::new(
                ctx,
                copy responses,
                sender,
                self.id(),
                number,
                topic_hash,
            );
            self.threads_mut().add(number, thread.id());
            let new_post = board::new_post_migrate_v2(
                responses,
                sender,
                timestamp_ms,
                thread.id(),
                text_hash,
                media_hashes,
                name_hash,
                vote_keys,
                multi_vote,
            );
            self.apply_thread(ctx, clock, forum, &mut thread, new_post);
            thread.share();
        },
        b"set_description" => {
            let desc_hash = event.peel_option!(|b| b.peel_u256());
            assert!(
                addr == forum.admin()
                    || forum.mods().contains(addr)
                    || self.mods().contains(addr),
                error::not_authorized(),
            );
            *self.desc_hash_mut() = desc_hash;
        },
        b"set_ignore_forum_bans" => {
            let ignore = event.peel_bool();
            assert!(
                addr == forum.admin()
                    || forum.mods().contains(addr),
                error::not_authorized(),
            );
            *self.ignore_forum_bans_mut() = ignore;
        },
        b"set_reactions" => {
            let reactions = event.peel_vec!(|b| b.peel_u256());
            assert!(
                addr == forum.admin()
                    || forum.mods().contains(addr)
                    || self.mods().contains(addr),
                error::not_authorized(),
            );
            *self.reactions_mut() = vec_set::from_keys(reactions);
        },
        b"set_pinned" => {
            let pinned = event.peel_vec!(|b| b.peel_address());
            assert!(
                addr == forum.admin()
                    || forum.mods().contains(addr)
                    || self.mods().contains(addr),
                error::not_authorized(),
            );
            *self.pinned_mut() = pinned;
        },
        _ => abort,
    };

    assert!(event.into_remainder_bytes().is_empty());
}

public(package) fun apply_thread(
    self: &mut Board,
    ctx: &mut TxContext,
    clock: &Clock,
    forum: &Forum,
    thread: &mut Thread,
    event: vector<u8>,
) {
    self.push(event);

    assert!(forum.boards().contains(*self.slug()), error::cross_reference_mismatch());
    assert!(self.id() == thread.board(), error::cross_reference_mismatch());

    let mut event = bcs::new(event);
    event::peel_version(&mut event);
    let responses = responses::peel(&mut event);
    let sender = sender::peel(&mut event);
    let addr = sender.addr();
    let tag = event.peel_vec_u8();
    if (tag != b"upgrade" && !self.check_version()) {
        abort error::entity_version_unsupported()
    };
    match (tag) {
        b"upgrade" => {
            self.do_upgrade(ctx);
        },
        b"new_post_v2" => {
            let thread_id = event.peel_address();
            let text_hash = event.peel_option!(|b| b.peel_u256());
            let media_hashes = event.peel_vec!(|b| b.peel_u256());
            let name_hash = event.peel_option!(|b| b.peel_u256());
            let vote_keys = event.peel_vec!(|b| b.peel_u256());
            let multi_vote = event.peel_bool();
            assert!(thread.id() == thread_id, error::cross_reference_mismatch());
            assert!(media_hashes.length() <= *self.max_media(), error::media_limit_exceeded());
            assert!(media_hashes.length() > 0 || text_hash.is_some(), error::post_empty());
            assert!(vote_keys.length() <= 16, error::vote_options_limit());
            assert!(
                !*self.closed() || (addr == forum.admin()
                    || forum.mods().contains(addr)
                    || self.mods().contains(addr)),
                error::board_closed(),
            );
            let number = self.posts().length() + 1;
            let ts = clock.timestamp_ms();
            let precision = forum.timestamp_precision();
            let timestamp_ms = if (*precision > 0) ts - ts % *precision else ts;
            let post = post::new(
                ctx,
                copy responses,
                sender,
                thread.id(),
                number,
                timestamp_ms,
                name_hash,
                text_hash,
                media_hashes,
                vote_keys,
                multi_vote,
            );
            self.posts_mut().add(number, post.id());
            if (thread.posts().next() <= *self.bump_limit() && !self.pinned().contains(&thread.id())) {
                self.bumps_mut().push(thread.id()).share();
            };
            thread.apply(ctx, forum, self, thread::new_post(responses, sender, post.id()));
            post.share();
        },
        b"new_post_migrate_v2" => {
            let timestamp_ms = event.peel_u64();
            let thread_id = event.peel_address();
            let text_hash = event.peel_option!(|b| b.peel_u256());
            let media_hashes = event.peel_vec!(|b| b.peel_u256());
            let name_hash = event.peel_option!(|b| b.peel_u256());
            let vote_keys = event.peel_vec!(|b| b.peel_u256());
            let multi_vote = event.peel_bool();
            assert!(thread.id() == thread_id, error::cross_reference_mismatch());
            assert!(media_hashes.length() <= *self.max_media(), error::media_limit_exceeded());
            assert!(media_hashes.length() > 0 || text_hash.is_some(), error::post_empty());
            assert!(vote_keys.length() <= 16, error::vote_options_limit());
            assert!(
                !*self.closed() || (addr == forum.admin()
                    || forum.mods().contains(addr)
                    || self.mods().contains(addr)),
                error::board_closed(),
            );
            let number = self.posts().length() + 1;
            let post = post::new(
                ctx,
                copy responses,
                sender,
                thread.id(),
                number,
                timestamp_ms,
                name_hash,
                text_hash,
                media_hashes,
                vote_keys,
                multi_vote,
            );
            self.posts_mut().add(number, post.id());
            if (thread.posts().next() <= *self.bump_limit() && !self.pinned().contains(&thread.id())) {
                self.bumps_mut().push(thread.id()).share();
            };
            thread.apply(ctx, forum, self, thread::new_post(responses, sender, post.id()));
            post.share();
        },
        _ => abort,
    };

    assert!(event.into_remainder_bytes().is_empty());
}

public(package) fun apply_post(
    self: &mut Board,
    ctx: &mut TxContext,
    clock: &Clock,
    forum: &Forum,
    thread: &Thread,
    post: &mut Post,
    event: vector<u8>,
) {
    self.push(event);

    assert!(forum.boards().contains(*self.slug()), error::cross_reference_mismatch());
    assert!(self.id() == thread.board(), error::cross_reference_mismatch());
    assert!(thread.id() == post.thread(), error::cross_reference_mismatch());

    let mut event = bcs::new(event);
    event::peel_version(&mut event);
    let responses = responses::peel(&mut event);
    let sender = sender::peel(&mut event);
    let addr = sender.addr();
    let tag = event.peel_vec_u8();
    if (tag != b"upgrade" && !self.check_version()) {
        abort error::entity_version_unsupported()
    };
    match (tag) {
        b"upgrade" => {
            self.do_upgrade(ctx);
        },
        b"ban" => {
            let key = bans::key(event.peel_address(), event.peel_u8(), event.peel_u256());
            let value = bans::value(event.peel_u256(), event.peel_u64());
            assert!(
                addr == forum.admin()
                    || forum.mods().contains(addr)
                    || self.mods().contains(addr),
                error::not_authorized(),
            );
            self.bans_mut().ban(key, value);
            post.apply(
                ctx,
                clock,
                forum,
                self,
                thread,
                post::set_banned(responses, sender, option::some(key)),
            );
        },
        b"unban" => {
            let key = bans::key(event.peel_address(), event.peel_u8(), event.peel_u256());
            assert!(post.banned().borrow() == key, error::cross_reference_mismatch());
            assert!(
                addr == forum.admin()
                    || forum.mods().contains(addr)
                    || self.mods().contains(addr),
                error::not_authorized(),
            );
            self.bans_mut().unban(key);
            post.apply(
                ctx,
                clock,
                forum,
                self,
                thread,
                post::set_banned(responses, sender, option::none()),
            );
        },
        _ => abort,
    };

    assert!(event.into_remainder_bytes().is_empty());
}
