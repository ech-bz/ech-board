#[test_only]
module forum::post_api_tests;

use forum::board::{Self, Board};
use forum::forum::{Self, Forum};
use forum::post::{Self, Post};
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

fun fixture(
    ctx: &mut TxContext,
    text_hash: Option<u256>,
    media_hashes: vector<u256>,
    vote_keys: vector<u256>,
): (Forum, Board, Thread, Post, Clock) {
    let admin = actor(ADMIN_PK);
    let mut forum = forum::new(ctx, admin, b"forum", admin.addr());
    let board = board::new(ctx, admin, b"board", ascii::string(b"test"));
    forum.boards_mut().add(ascii::string(b"test"), board.id());
    let thread = thread::new(
        ctx,
        admin,
        b"thread",
        board.id(),
        1,
        option::none(),
    );
    let post = post::new(
        ctx,
        actor(AUTHOR_PK),
        b"post",
        thread.id(),
        1,
        0,
        text_hash,
        media_hashes,
        vote_keys,
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
    let (forum, board, thread, mut post, clock) =
        fixture(&mut ctx, option::some(1), vector[10, 11], vector[]);
    let author = actor(AUTHOR_PK);

    post.apply(
        &clock,
        &forum,
        &board,
        &thread,
        post::set_text(author, b"1", option::some(2)),
    );
    assert!(*post.text_hash() == option::some(2));

    post.apply(
        &clock,
        &forum,
        &board,
        &thread,
        post::remove_media(author, b"2", vector[10]),
    );
    assert!(post.media_hashes() == &vector[11]);

    post.apply(
        &clock,
        &forum,
        &board,
        &thread,
        post::set_deleted(author, b"3", true),
    );
    assert!(*post.deleted());

    finish(forum, board, thread, post, clock);
}

#[test]
fun post_set_text_none_auto_deletes_empty_post() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, clock) =
        fixture(&mut ctx, option::some(1), vector[], vector[]);
    post.apply(
        &clock,
        &forum,
        &board,
        &thread,
        post::set_text(actor(AUTHOR_PK), b"1", option::none()),
    );
    assert!(post.text_hash().is_none());
    assert!(*post.deleted());
    finish(forum, board, thread, post, clock);
}

#[test]
fun post_remove_last_media_auto_deletes_empty_post() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, clock) =
        fixture(&mut ctx, option::none(), vector[10], vector[]);
    post.apply(
        &clock,
        &forum,
        &board,
        &thread,
        post::remove_media(actor(AUTHOR_PK), b"1", vector[10]),
    );
    assert!(post.media_hashes().is_empty());
    assert!(*post.deleted());
    finish(forum, board, thread, post, clock);
}

#[test]
fun board_moderator_can_edit_after_self_moderation_window() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, thread, mut post, mut clock) =
        fixture(&mut ctx, option::some(1), vector[10], vector[]);
    let board_mod = actor(BOARD_MOD_PK);
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::add_moderator(actor(ADMIN_PK), b"1", board_mod.addr()),
    );
    clock.set_for_testing(600_001);
    post.apply(
        &clock,
        &forum,
        &board,
        &thread,
        post::set_text(board_mod, b"2", option::some(2)),
    );
    assert!(*post.text_hash() == option::some(2));
    finish(forum, board, thread, post, clock);
}

#[test]
fun thread_moderator_can_edit_after_self_moderation_window() {
    let mut ctx = tx_context::dummy();
    let (forum, board, mut thread, mut post, mut clock) =
        fixture(&mut ctx, option::some(1), vector[], vector[]);
    let thread_mod = actor(THREAD_MOD_PK);
    thread.apply(
        &forum,
        &board,
        thread::add_moderator(actor(ADMIN_PK), b"1", thread_mod.addr()),
    );
    clock.set_for_testing(600_001);
    post.apply(
        &clock,
        &forum,
        &board,
        &thread,
        post::set_text(thread_mod, b"2", option::some(2)),
    );
    assert!(*post.text_hash() == option::some(2));
    finish(forum, board, thread, post, clock);
}

#[test]
fun empty_post_can_be_deleted_by_any_user() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, clock) =
        fixture(&mut ctx, option::none(), vector[], vector[]);
    post.apply(
        &clock,
        &forum,
        &board,
        &thread,
        post::set_deleted(actor(USER_PK), b"1", true),
    );
    assert!(*post.deleted());
    finish(forum, board, thread, post, clock);
}

#[test]
fun post_reaction_add_toggle_and_change() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, thread, mut post, clock) =
        fixture(&mut ctx, option::some(1), vector[], vector[]);
    let admin = actor(ADMIN_PK);
    let user = actor(USER_PK);
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_reactions(admin, b"1", vector[100, 101]),
    );

    post.apply(
        &clock,
        &forum,
        &board,
        &thread,
        post::set_reaction(user, b"2", 500, 100),
    );
    assert!(post.reactions()[&100] == 1);

    post.apply(
        &clock,
        &forum,
        &board,
        &thread,
        post::set_reaction(user, b"3", 500, 100),
    );
    assert!(!post.reactions().contains(&100));

    post.apply(
        &clock,
        &forum,
        &board,
        &thread,
        post::set_reaction(user, b"4", 500, 100),
    );
    post.apply(
        &clock,
        &forum,
        &board,
        &thread,
        post::set_reaction(user, b"5", 500, 101),
    );
    assert!(!post.reactions().contains(&100));
    assert!(post.reactions()[&101] == 1);

    finish(forum, board, thread, post, clock);
}

#[test]
fun post_vote_allowed_event() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, clock) =
        fixture(&mut ctx, option::some(1), vector[], vector[200]);
    post.apply(
        &clock,
        &forum,
        &board,
        &thread,
        post::vote(actor(USER_PK), b"1", 500, 200),
    );
    assert!(post.votes()[&200] == 1);
    finish(forum, board, thread, post, clock);
}

#[test]
#[expected_failure(abort_code = 14)]
fun post_author_cannot_edit_after_self_moderation_window() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, mut clock) =
        fixture(&mut ctx, option::some(1), vector[], vector[]);
    clock.set_for_testing(600_001);
    post.apply(
        &clock,
        &forum,
        &board,
        &thread,
        post::set_text(actor(AUTHOR_PK), b"1", option::some(2)),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 14)]
fun post_other_user_cannot_edit() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, clock) =
        fixture(&mut ctx, option::some(1), vector[], vector[]);
    post.apply(
        &clock,
        &forum,
        &board,
        &thread,
        post::set_deleted(actor(USER_PK), b"1", true),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 16)]
fun post_rejects_unconfigured_reaction() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, clock) =
        fixture(&mut ctx, option::some(1), vector[], vector[]);
    post.apply(
        &clock,
        &forum,
        &board,
        &thread,
        post::set_reaction(actor(USER_PK), b"1", 500, 100),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 16)]
fun post_rejects_unknown_vote_option() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, clock) =
        fixture(&mut ctx, option::some(1), vector[], vector[200]);
    post.apply(
        &clock,
        &forum,
        &board,
        &thread,
        post::vote(actor(USER_PK), b"1", 500, 201),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 17)]
fun post_rejects_duplicate_vote_by_ip() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, clock) =
        fixture(&mut ctx, option::some(1), vector[], vector[200]);
    let user = actor(USER_PK);
    post.apply(
        &clock,
        &forum,
        &board,
        &thread,
        post::vote(user, b"1", 500, 200),
    );
    post.apply(
        &clock,
        &forum,
        &board,
        &thread,
        post::vote(user, b"2", 500, 200),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 14)]
fun post_rejects_reaction_takeover_by_same_ip() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, thread, mut post, clock) =
        fixture(&mut ctx, option::some(1), vector[], vector[]);
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_reactions(actor(ADMIN_PK), b"1", vector[100]),
    );
    post.apply(
        &clock,
        &forum,
        &board,
        &thread,
        post::set_reaction(actor(USER_PK), b"2", 500, 100),
    );
    post.apply(
        &clock,
        &forum,
        &board,
        &thread,
        post::set_reaction(actor(OTHER_USER_PK), b"3", 500, 100),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 17)]
fun post_rejects_duplicate_vote_by_sender_on_different_ip() {
    let mut ctx = tx_context::dummy();
    let (forum, board, thread, mut post, clock) =
        fixture(&mut ctx, option::some(1), vector[], vector[200]);
    let user = actor(USER_PK);
    post.apply(
        &clock,
        &forum,
        &board,
        &thread,
        post::vote(user, b"1", 500, 200),
    );
    post.apply(
        &clock,
        &forum,
        &board,
        &thread,
        post::vote(user, b"2", 501, 200),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 15)]
fun post_rejects_thread_cross_reference_mismatch() {
    let mut ctx = tx_context::dummy();
    let (forum, board, _thread, mut post, clock) =
        fixture(&mut ctx, option::some(1), vector[], vector[]);
    let wrong_thread = thread::new(
        &mut ctx,
        actor(ADMIN_PK),
        b"wrong",
        board.id(),
        2,
        option::none(),
    );
    post.apply(
        &clock,
        &forum,
        &board,
        &wrong_thread,
        post::set_text(actor(AUTHOR_PK), b"1", option::some(2)),
    );
    abort
}
