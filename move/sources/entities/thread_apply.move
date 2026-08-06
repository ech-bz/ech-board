module forum::thread_apply;

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

public(package) fun apply(
    self: &mut Thread,
    ctx: &mut TxContext,
    forum: &Forum,
    board: &Board,
    event: vector<u8>,
) {
    self.push(event);

    assert!(forum.boards().contains(*board.slug()), error::cross_reference_mismatch());
    assert!(board.id() == self.board(), error::cross_reference_mismatch());

    let mut event = bcs::new(event);
    event::peel_version(&mut event);
    responses::peel(&mut event);
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
                    || forum.mods().contains(addr)
                    || board.mods().contains(addr)
                    || self.admin().is_some_and!(|a| addr == a),
                error::not_authorized(),
            );
            self.mods_mut().add(moderator, empty::new());
        },
        b"del_moderator" => {
            let moderator = event.peel_address();
            assert!(
                addr == forum.admin()
                    || forum.mods().contains(addr)
                    || board.mods().contains(addr)
                    || self.admin().is_some_and!(|a| addr == a),
                error::not_authorized(),
            );
            self.mods_mut().remove(moderator);
        },
        b"set_closed" => {
            let closed = event.peel_bool();
            assert!(
                addr == forum.admin()
                    || forum.mods().contains(addr)
                    || board.mods().contains(addr)
                    || self.admin().is_some_and!(|a| addr == a)
                    || self.mods().contains(addr),
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
                    || forum.mods().contains(addr)
                    || board.mods().contains(addr)
                    || self.admin().is_some_and!(|a| addr == a)
                    || self.mods().contains(addr),
                error::not_authorized(),
            );
            assert!(*self.deleted() != deleted);
            assert!(*self.closed());
            *self.deleted_mut() = deleted;
        },
        b"set_topic" => {
            let topic_hash = event.peel_option!(|b| b.peel_u256());
            assert!(
                self.genesis()
                    || addr == forum.admin()
                    || forum.mods().contains(addr)
                    || board.mods().contains(addr)
                    || self.admin().is_some_and!(|a| addr == a)
                    || self.mods().contains(addr),
                error::not_authorized(),
            );
            *self.topic_hash_mut() = topic_hash;
        },
        b"set_admin" => {
            let admin = event.peel_option!(|b| b.peel_address());
            assert!(
                addr == forum.admin()
                    || forum.mods().contains(addr)
                    || board.mods().contains(addr)
                    || self.admin().is_some_and!(|a| addr == a),
                error::not_authorized(),
            );
            *self.admin_mut() = admin;
        },
        b"new_post" => {
            let post = event.peel_address();
            assert!(
                !*self.closed() || (addr == forum.admin()
                    || forum.mods().contains(addr)
                    || board.mods().contains(addr)
                    || self.admin().is_some_and!(|a| addr == a)),
                error::thread_closed(),
            );
            self.posts_mut().push(post).share();
            if (self.genesis()) {
                *self.op_mut() = post;
            } else {
                let last3 = self.last3_mut();
                last3.push_back(post);
                if (last3.length() > 3) {
                    last3.remove(0);
                };
            };
        },
        _ => abort,
    };

    assert!(event.into_remainder_bytes().is_empty());
}

public(package) fun apply_post(
    self: &mut Thread,
    ctx: &mut TxContext,
    clock: &Clock,
    forum: &Forum,
    board: &Board,
    post: &mut Post,
    event: vector<u8>,
) {
    self.push(event);

    assert!(forum.boards().contains(*board.slug()), error::cross_reference_mismatch());
    assert!(board.id() == self.board(), error::cross_reference_mismatch());
    assert!(self.id() == post.thread(), error::cross_reference_mismatch());

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
                    || board.mods().contains(addr)
                    || self.admin().is_some_and!(|a| addr == a)
                    || self.mods().contains(addr),
                error::not_authorized(),
            );
            self.bans_mut().ban(key, value);
            post.apply(
                ctx,
                clock,
                forum,
                board,
                self,
                post::set_banned(responses, sender, option::some(key)),
            );
        },
        b"unban" => {
            let key = bans::key(event.peel_address(), event.peel_u8(), event.peel_u256());
            assert!(post.banned().borrow() == key, error::cross_reference_mismatch());
            assert!(
                addr == forum.admin()
                    || forum.mods().contains(addr)
                    || board.mods().contains(addr)
                    || self.admin().is_some_and!(|a| addr == a)
                    || self.mods().contains(addr),
                error::not_authorized(),
            );
            self.bans_mut().unban(key);
            post.apply(
                ctx,
                clock,
                forum,
                board,
                self,
                post::set_banned(responses, sender, option::none()),
            );
        },
        _ => abort,
    };

    assert!(event.into_remainder_bytes().is_empty());
}
