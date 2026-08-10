module forum::post_apply;

use forum::bans::{Self, BanKey, BanValue};
use forum::board::{Self, Board};
use forum::empty::{Self, Empty};
use forum::entity;
use forum::error;
use forum::event;
use forum::forum::{Self, Forum};
use forum::post::{Self, Post};
use forum::responses;
use forum::sender::{Self, Sender};
use forum::thread::{Self, Thread};
use forum::user_entry::Self;
use std::ascii::{Self, String};
use sui::bcs;
use sui::clock::Clock;
use sui::table::{Self, Table};
use sui::vec_set::{Self, VecSet};

public(package) fun apply(
    self: &mut Post,
    ctx: &mut TxContext,
    clock: &Clock,
    forum: &Forum,
    board: &Board,
    thread: &Thread,
    event: vector<u8>,
) {
    self.push(event);

    assert!(forum.boards().contains(*board.slug()), error::cross_reference_mismatch());
    assert!(board.id() == thread.board(), error::cross_reference_mismatch());
    assert!(thread.id() == self.thread(), error::cross_reference_mismatch());

    let mut event = bcs::new(event);
    event::peel_version(&mut event);
    let responses = responses::peel(&mut event);
    let sender = sender::peel(&mut event);
    let addr = sender.addr();

    let can_self_moderate =
        (self.sender() == sender) && (clock.timestamp_ms() - *self.timestamp()) <= 600000;

    let tag = event.peel_vec_u8();
    if (tag != b"upgrade" && !self.check_version()) {
        abort error::entity_version_unsupported()
    };
    match (tag) {
        b"upgrade" => {
            self.do_upgrade(ctx);
        },
        b"set_deleted" => {
            let deleted = event.peel_bool();
            assert!(*self.deleted() != deleted);
            assert!(
                addr == forum.admin()
                    || forum.mods().contains(addr)
                    || board.mods().contains(addr)
                    || thread.admin().is_some_and!(|a| addr == a)
                    || thread.mods().contains(addr)
                    || can_self_moderate
                    || (deleted && self.media_hashes().is_empty() && self.text_hash().is_none()),
                error::not_authorized(),
            );
            *self.deleted_mut() = deleted;
        },
        b"set_text" => {
            let hash = event.peel_option!(|b| b.peel_u256());
            assert!(
                addr == forum.admin()
                    || forum.mods().contains(addr)
                    || board.mods().contains(addr)
                    || thread.admin().is_some_and!(|a| addr == a)
                    || thread.mods().contains(addr)
                    || can_self_moderate,
                error::not_authorized(),
            );
            *self.text_hash_mut() = hash;
            if (self.media_hashes().is_empty() && self.text_hash().is_none()) {
                self.apply(
                    ctx,
                    clock,
                    forum,
                    board,
                    thread,
                    post::set_deleted(responses, sender, true),
                );
            };
        },
        b"ban_media" => {
            let hashes = event.peel_vec!(|b| b.peel_u256());
            assert!(
                addr == forum.admin()
                    || forum.mods().contains(addr)
                    || board.mods().contains(addr)
                    || thread.admin().is_some_and!(|a| addr == a)
                    || thread.mods().contains(addr)
                    || can_self_moderate,
                error::not_authorized(),
            );
            hashes.do!(|hash| {
                assert!(self.media_hashes().contains(&hash), error::media_not_found());
                self.banned_media_mut().insert(hash);
            });
        },
        b"unban_media" => {
            let hashes = event.peel_vec!(|b| b.peel_u256());
            assert!(
                addr == forum.admin()
                    || forum.mods().contains(addr)
                    || board.mods().contains(addr)
                    || thread.admin().is_some_and!(|a| addr == a)
                    || thread.mods().contains(addr),
                error::not_authorized(),
            );
            hashes.do!(|hash| {
                self.banned_media_mut().remove(&hash);
            });
        },
        b"set_reaction" => {
            let ip32_hash = *responses.ip32().borrow();
            let reaction_hash = event.peel_u256();
            assert!(board.reactions().contains(&reaction_hash), error::reaction_not_allowed());
            let reacted_id = self.reacted().find(sender.pk()).or!(self.reacted().find(ip32_hash));
            if (reacted_id.is_some()) {
                let reacted_id = *reacted_id.borrow();
                let entry = *self.reacted().entry(reacted_id);
                let entry_hash = entry.options()[0];
                assert!(entry.sender() == sender, error::not_authorized());
                reaction_dec(self, entry_hash);
                self.reacted_mut().remove(reacted_id);
                if (entry_hash != reaction_hash) {
                    reaction_inc(self, reaction_hash);
                    self
                        .reacted_mut()
                        .add(
                            vector[ip32_hash, sender.pk()],
                            user_entry::new(sender, vector[reaction_hash]),
                        );
                };
            } else {
                reaction_inc(self, reaction_hash);
                self
                    .reacted_mut()
                    .add(
                        vector[ip32_hash, sender.pk()],
                        user_entry::new(sender, vector[reaction_hash]),
                    );
            };
        },
        b"vote_v2" => {
            let ip32_hash = *responses.ip32().borrow();
            let options = event.peel_vec!(|b| b.peel_u256());
            assert!(
                options.length() == 1 || (options.length() != 0 && *self.multi_vote()),
                error::vote_options_mismatch(),
            );
            assert!(self.voted().find(ip32_hash).is_none(), error::already_voted());
            assert!(self.voted().find(sender.pk()).is_none(), error::already_voted());
            let options = vec_set::from_keys(options);
            let keys = *options.keys();
            keys.do!(|option| {
                assert!(self.votes().contains(&option), error::vote_options_mismatch());
                let count = &mut self.votes_mut()[&option];
                *count = *count + 1;
            });
            self
                .voted_mut()
                .add(
                    vector[ip32_hash, sender.pk()],
                    user_entry::new(sender, *options.keys()),
                );
        },
        b"set_banned" => {
            let banned = event.peel_option!(
                |bcs| bans::key(bcs.peel_address(), bcs.peel_u8(), bcs.peel_u256()),
            );
            assert!(self.banned().is_some() != banned.is_some());
            assert!(
                addr == forum.admin()
                    || forum.mods().contains(addr)
                    || board.mods().contains(addr)
                    || thread.admin().is_some_and!(|a| addr == a)
                    || thread.mods().contains(addr),
                error::not_authorized(),
            );
            *self.banned_mut() = banned;
        },
        b"set_mod_note" => {
            let mod_note = event.peel_option!(|b| b.peel_u256());
            assert!(
                addr == forum.admin()
                    || forum.mods().contains(addr)
                    || board.mods().contains(addr)
                    || thread.admin().is_some_and!(|a| addr == a)
                    || thread.mods().contains(addr),
                error::not_authorized(),
            );
            *self.mod_note_mut() = mod_note;
        },
        _ => abort,
    };

    assert!(event.into_remainder_bytes().is_empty());
}

fun reaction_inc(self: &mut Post, hash: u256) {
    let reactions = self.reactions_mut();
    if (reactions.contains(&hash)) {
        let count = &mut reactions[&hash];
        *count = *count + 1;
    } else {
        reactions.insert(hash, 1);
    };
}

fun reaction_dec(self: &mut Post, hash: u256) {
    let reactions = self.reactions_mut();
    let count = &mut reactions[&hash];
    *count = *count - 1;
    if (*count == 0) {
        reactions.remove(&hash);
    }
}
