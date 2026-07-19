module forum::forum_apply;

use forum::bans;
use forum::board::{Self, Board};
use forum::empty;
use forum::error;
use forum::forum::Forum;
use forum::post::{Self, Post};
use forum::sender;
use forum::thread::{Self, Thread};
use std::ascii;
use sui::bcs;
use sui::clock::Clock;

public(package) fun apply(self: &mut Forum, ctx: &mut TxContext, clock: &Clock, event: vector<u8>) {
    self.push(event);

    let mut event = bcs::new(event);
    let tag = event.peel_vec_u8();
    let sender = sender::new(event.peel_u256(), event.peel_u256());
    let addr = sender.addr();
    let uid = event.peel_vec_u8();

    match (tag) {
        b"add_moderator" => {
            let moderator = event.peel_address();
            assert!(addr == self.admin(), error::not_authorized());
            self.mods_mut().add(moderator, empty::new());
        },
        b"del_moderator" => {
            let moderator = event.peel_address();
            assert!(addr == self.admin(), error::not_authorized());
            self.mods_mut().remove(moderator);
        },
        b"new_board" => {
            let slug = ascii::string(event.peel_vec_u8());
            let max_media = event.peel_u64();
            let bump_limit = event.peel_u64();
            let desc_hash = event.peel_option!(|b| b.peel_u256());
            assert!(
                addr == self.admin()
                    || self.mods().contains(addr),
                error::not_authorized(),
            );
            assert!(
                slug.as_bytes().all!(|c| (*c >= 0x30 && *c <= 0x39) || (*c >= 0x61 && *c <= 0x7a)),
                error::board_slug_invalid(),
            );
            assert!(slug.length() >= 1 && slug.length() <= 16, error::board_slug_invalid());
            let mut board = board::new(ctx, sender, uid, slug);
            self.boards_mut().add(slug, board.id());
            board.apply(
                ctx,
                clock,
                self,
                board::set_max_media(sender, uid, max_media),
            );
            board.apply(
                ctx,
                clock,
                self,
                board::set_bump_limit(sender, uid, bump_limit),
            );
            board.apply(
                ctx,
                clock,
                self,
                board::set_description(sender, uid, desc_hash),
            );
            board.share();
        },
        b"set_timestamp_precision" => {
            let precision = event.peel_u64();
            assert!(
                addr == self.admin()
                    || self.mods().contains(addr),
                error::not_authorized(),
            );
            *self.timestamp_precision_mut() = precision;
        },
        b"ban" => {
            let key = bans::key(event.peel_address(), event.peel_u8(), event.peel_u256());
            let value = bans::value(event.peel_u256(), event.peel_u64());
            assert!(
                addr == self.admin()
                    || self.mods().contains(addr),
                error::not_authorized(),
            );
            self.bans_mut().ban(key, value);
        },
        b"unban" => {
            let key = bans::key(event.peel_address(), event.peel_u8(), event.peel_u256());
            assert!(
                addr == self.admin()
                    || self.mods().contains(addr),
                error::not_authorized(),
            );
            self.bans_mut().unban(key);
        },
        _ => abort,
    };

    assert!(event.into_remainder_bytes().is_empty());
}

public(package) fun apply_post(
    self: &mut Forum,
    clock: &Clock,
    board: &Board,
    thread: &Thread,
    post: &mut Post,
    event: vector<u8>,
) {
    self.push(event);

    assert!(self.boards().contains(*board.slug()), error::cross_reference_mismatch());
    assert!(board.id() == thread.board(), error::cross_reference_mismatch());
    assert!(thread.id() == post.thread(), error::cross_reference_mismatch());

    let mut event = bcs::new(event);
    let tag = event.peel_vec_u8();
    let sender = sender::new(event.peel_u256(), event.peel_u256());
    let addr = sender.addr();
    let uid = event.peel_vec_u8();

    match (tag) {
        b"ban" => {
            let key = bans::key(event.peel_address(), event.peel_u8(), event.peel_u256());
            let value = bans::value(event.peel_u256(), event.peel_u64());
            assert!(
                addr == self.admin()
                    || self.mods().contains(addr),
                error::not_authorized(),
            );
            self.bans_mut().ban(key, value);
            post.apply(
                clock,
                self,
                board,
                thread,
                post::set_banned(sender, uid, option::some(key)),
            );
        },
        b"unban" => {
            let key = bans::key(event.peel_address(), event.peel_u8(), event.peel_u256());
            assert!(post.banned().borrow() == key, error::cross_reference_mismatch());
            assert!(
                addr == self.admin()
                    || self.mods().contains(addr),
                error::not_authorized(),
            );
            self.bans_mut().unban(key);
            post.apply(
                clock,
                self,
                board,
                thread,
                post::set_banned(sender, uid, option::none()),
            );
        },
        _ => abort,
    };

    assert!(event.into_remainder_bytes().is_empty());
}
