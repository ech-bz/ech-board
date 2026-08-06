#[test_only]
module forum::post_api_tests;

use forum::board::{Self, Board};
use forum::forum::{Self, Forum};
use forum::post::{Self, Post};
use forum::responses::{Self, Responses};
use forum::sender::{Self, Sender};
use forum::thread::{Self, Thread};
use std::ascii;
use sui::clock::{Self, Clock};
use sui::tx_context;

const ADMIN_PK: u256 = 31;
const BOARD_MOD_PK: u256 = 32;
const AUTHOR_PK: u256 = 33;
const USER_PK: u256 = 34;
const THREAD_MOD_PK: u256 = 35;
const OTHER_USER_PK: u256 = 36;

fun actor(pk: u256): Sender {
    sender::new(pk, 0)
}

fun uid(value: vector<u8>): Responses {
    responses::new(option::some(value), option::none(), option::none(), option::none())
}

fun uid_ip(value: vector<u8>, ip32: u256): Responses {
    responses::new(option::some(value), option::some(ip32), option::none(), option::none())
}

fun fixture(
    ctx: &mut TxContext,
    text_hash: Option<u256>,
    media_hashes: vector<u256>,
    vote_keys: vector<u256>,
): (Forum, Board, Thread, Post, Clock) {
    let admin = actor(ADMIN_PK);
    let mut forum = forum::new(ctx, uid(b"forum"), admin, admin.addr());
    let board = board::new(ctx, uid(b"board"), admin, ascii::string(b"test"));
    forum.boards_mut().add(ascii::string(b"test"), board.id());
    let thread = thread::new(
        ctx,
        uid(b"thread"),
        admin,
        board.id(),
        1,
        option::none(),
    );
    let post = post::new(
        ctx,
        uid(b"post"),
        actor(AUTHOR_PK),
        thread.id(),
        1,
        0,
        option::none(),
        text_hash,
        media_hashes,
        vote_keys,
        false,
    );
    let clock = clock::create_for_testing(ctx);
    (forum, board, thread, post, clock)
}

fun fixture_multi(
    ctx: &mut TxContext,
    vote_keys: vector<u256>,
): (Forum, Board, Thread, Post, Clock) {
    let admin = actor(ADMIN_PK);
    let mut forum = forum::new(ctx, uid(b"forum"), admin, admin.addr());
    let board = board::new(ctx, uid(b"board"), admin, ascii::string(b"test"));
    forum.boards_mut().add(ascii::string(b"test"), board.id());
    let thread = thread::new(
        ctx,
        uid(b"thread"),
        admin,
        board.id(),
        1,
        option::none(),
    );
    let post = post::new(
        ctx,
        uid(b"post"),
        actor(AUTHOR_PK),
        thread.id(),
        1,
        0,
        option::none(),
        option::some(1),
        vector[],
        vote_keys,
        true,
    );
    let clock = clock::create_for_testing(ctx);
    (forum, board, thread, post, clock)
}

fun finish(forum: Forum, board: Board, thread: Thread, post: Post, clock: Clock) {
    clock.destroy_for_testing();
    post.share();
    thread.share();
    board.share();
    forum.share();
}

#[test]
fun post_uid_allowed_events_for_author() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, clock) = fixture(
        &mut ctx,
        option::some(1),
        vector[10, 11],
        vector[],
    );
    let author = actor(AUTHOR_PK);

    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::set_text(uid(b"1"), author, option::some(2)),
    );
    assert!(*post.text_hash() == option::some(2));

    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::ban_media(uid(b"2"), author, vector[10]),
    );
    assert!(post.banned_media().contains(&10));
    assert!(post.media_hashes() == &vector[10, 11]);

    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::set_deleted(uid(b"3"), author, true),
    );
    assert!(*post.deleted());

    finish(forum, board, thread, post, clock);
}

#[test]
fun post_set_text_none_auto_deletes_empty_post() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, clock) = fixture(
        &mut ctx,
        option::some(1),
        vector[],
        vector[],
    );
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::set_text(uid(b"1"), actor(AUTHOR_PK), option::none()),
    );
    assert!(post.text_hash().is_none());
    assert!(*post.deleted());
    finish(forum, board, thread, post, clock);
}

#[test]
fun post_moderator_ban_then_unban_media() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, thread, mut post, mut clock) = fixture(
        &mut ctx,
        option::some(1),
        vector[10, 11],
        vector[],
    );
    let board_mod = actor(BOARD_MOD_PK);
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::add_moderator(uid(b"1"), actor(ADMIN_PK), board_mod.addr()),
    );
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::ban_media(uid(b"2"), board_mod, vector[10]),
    );
    assert!(post.banned_media().contains(&10));
    assert!(post.banned_media().length() == 1);
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::unban_media(uid(b"3"), board_mod, vector[10]),
    );
    assert!(!post.banned_media().contains(&10));
    assert!(post.banned_media().length() == 0);
    assert!(post.media_hashes() == &vector[10, 11]);
    finish(forum, board, thread, post, clock);
}

#[test]
#[expected_failure(abort_code = 14)]
fun post_author_cannot_unban_media() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, clock) = fixture(
        &mut ctx,
        option::none(),
        vector[10],
        vector[],
    );
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::unban_media(uid(b"1"), actor(AUTHOR_PK), vector[10]),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 22)]
fun post_ban_media_rejects_hash_not_in_post() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, clock) = fixture(
        &mut ctx,
        option::none(),
        vector[10],
        vector[],
    );
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::ban_media(uid(b"1"), actor(AUTHOR_PK), vector[999]),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 0)]
fun post_ban_media_rejects_already_banned() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, clock) = fixture(
        &mut ctx,
        option::none(),
        vector[10],
        vector[],
    );
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::ban_media(uid(b"1"), actor(AUTHOR_PK), vector[10]),
    );
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::ban_media(uid(b"2"), actor(AUTHOR_PK), vector[10]),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 1)]
fun post_unban_media_rejects_not_banned() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, thread, mut post, mut clock) = fixture(
        &mut ctx,
        option::none(),
        vector[10],
        vector[],
    );
    let board_mod = actor(BOARD_MOD_PK);
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::add_moderator(uid(b"1"), actor(ADMIN_PK), board_mod.addr()),
    );
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::unban_media(uid(b"2"), board_mod, vector[10]),
    );
    abort
}

#[test]
fun board_moderator_can_edit_after_self_moderation_window() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, thread, mut post, mut clock) = fixture(
        &mut ctx,
        option::some(1),
        vector[10],
        vector[],
    );
    let board_mod = actor(BOARD_MOD_PK);
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::add_moderator(uid(b"1"), actor(ADMIN_PK), board_mod.addr()),
    );
    clock.set_for_testing(600_001);
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::set_text(uid(b"2"), board_mod, option::some(2)),
    );
    assert!(*post.text_hash() == option::some(2));
    finish(forum, board, thread, post, clock);
}

#[test]
fun thread_moderator_can_edit_after_self_moderation_window() {
    let mut ctx = tx_context::dummy();
    let (forum, board, mut thread, mut post, mut clock) = fixture(
        &mut ctx,
        option::some(1),
        vector[],
        vector[],
    );
    let thread_mod = actor(THREAD_MOD_PK);
    thread.apply(
        &mut ctx,
        &forum,
        &board,
        thread::add_moderator(uid(b"1"), actor(ADMIN_PK), thread_mod.addr()),
    );
    clock.set_for_testing(600_001);
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::set_text(uid(b"2"), thread_mod, option::some(2)),
    );
    assert!(*post.text_hash() == option::some(2));
    finish(forum, board, thread, post, clock);
}

#[test]
fun empty_post_can_be_deleted_by_any_user() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, clock) = fixture(
        &mut ctx,
        option::none(),
        vector[],
        vector[],
    );
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::set_deleted(uid(b"1"), actor(USER_PK), true),
    );
    assert!(*post.deleted());
    finish(forum, board, thread, post, clock);
}

#[test]
fun post_reaction_add_toggle_and_change() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, thread, mut post, clock) = fixture(
        &mut ctx,
        option::some(1),
        vector[],
        vector[],
    );
    let admin = actor(ADMIN_PK);
    let user = actor(USER_PK);
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_reactions(uid(b"1"), admin, vector[100, 101]),
    );

    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::set_reaction(uid_ip(b"2", 500), user, 100),
    );
    assert!(post.reactions()[&100] == 1);

    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::set_reaction(uid_ip(b"3", 500), user, 100),
    );
    assert!(!post.reactions().contains(&100));

    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::set_reaction(uid_ip(b"4", 500), user, 100),
    );
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::set_reaction(uid_ip(b"5", 500), user, 101),
    );
    assert!(!post.reactions().contains(&100));
    assert!(post.reactions()[&101] == 1);

    finish(forum, board, thread, post, clock);
}

#[test]
fun post_vote_allowed_event() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, clock) = fixture(
        &mut ctx,
        option::some(1),
        vector[],
        vector[200],
    );
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::vote_v2(uid_ip(b"1", 500), actor(USER_PK), vector[200]),
    );
    assert!(post.votes()[&200] == 1);
    finish(forum, board, thread, post, clock);
}

#[test]
#[expected_failure(abort_code = 14)]
fun post_author_cannot_edit_after_self_moderation_window() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, mut clock) = fixture(
        &mut ctx,
        option::some(1),
        vector[],
        vector[],
    );
    clock.set_for_testing(600_001);
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::set_text(uid(b"1"), actor(AUTHOR_PK), option::some(2)),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 14)]
fun post_other_user_cannot_edit() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, clock) = fixture(
        &mut ctx,
        option::some(1),
        vector[],
        vector[],
    );
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::set_deleted(uid(b"1"), actor(USER_PK), true),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 16)]
fun post_rejects_unconfigured_reaction() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, clock) = fixture(
        &mut ctx,
        option::some(1),
        vector[],
        vector[],
    );
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::set_reaction(uid_ip(b"1", 500), actor(USER_PK), 100),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 20)]
fun post_rejects_unknown_vote_option() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, clock) = fixture(
        &mut ctx,
        option::some(1),
        vector[],
        vector[200],
    );
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::vote_v2(uid_ip(b"1", 500), actor(USER_PK), vector[201]),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 17)]
fun post_rejects_duplicate_vote_by_ip() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, clock) = fixture(
        &mut ctx,
        option::some(1),
        vector[],
        vector[200],
    );
    let user = actor(USER_PK);
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::vote_v2(uid_ip(b"1", 500), user, vector[200]),
    );
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::vote_v2(uid_ip(b"2", 500), user, vector[200]),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 14)]
fun post_rejects_reaction_takeover_by_same_ip() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, thread, mut post, clock) = fixture(
        &mut ctx,
        option::some(1),
        vector[],
        vector[],
    );
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_reactions(uid(b"1"), actor(ADMIN_PK), vector[100]),
    );
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::set_reaction(uid_ip(b"2", 500), actor(USER_PK), 100),
    );
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::set_reaction(uid_ip(b"3", 500), actor(OTHER_USER_PK), 100),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 17)]
fun post_rejects_duplicate_vote_by_sender_on_different_ip() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, clock) = fixture(
        &mut ctx,
        option::some(1),
        vector[],
        vector[200],
    );
    let user = actor(USER_PK);
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::vote_v2(uid_ip(b"1", 500), user, vector[200]),
    );
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::vote_v2(uid_ip(b"2", 501), user, vector[200]),
    );
    abort
}

#[test]
fun post_multi_vote_single_tx_multiple_options() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, clock) = fixture_multi(
        &mut ctx,
        vector[200, 201],
    );
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::vote_v2(uid_ip(b"1", 500), actor(USER_PK), vector[200, 201]),
    );
    assert!(post.votes()[&200] == 1);
    assert!(post.votes()[&201] == 1);
    finish(forum, board, thread, post, clock);
}

#[test]
#[expected_failure(abort_code = 17)]
fun post_multi_vote_rejects_second_tx() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, clock) = fixture_multi(
        &mut ctx,
        vector[200, 201],
    );
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::vote_v2(uid_ip(b"1", 500), actor(USER_PK), vector[200]),
    );
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &thread,
        post::vote_v2(uid_ip(b"1", 500), actor(USER_PK), vector[201]),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 15)]
fun post_rejects_thread_cross_reference_mismatch() {
    let mut ctx = tx_context::dummy();
    let (forum, board, _thread, mut post, clock) = fixture(
        &mut ctx,
        option::some(1),
        vector[],
        vector[],
    );
    let wrong_thread = thread::new(
        &mut ctx,
        uid(b"wrong"),
        actor(ADMIN_PK),
        board.id(),
        2,
        option::none(),
    );
    post.apply(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &wrong_thread,
        post::set_text(uid(b"1"), actor(AUTHOR_PK), option::some(2)),
    );
    abort
}
