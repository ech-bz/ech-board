#[test_only]
module forum::thread_api_tests;

use forum::bans;
use forum::board::{Self, Board};
use forum::forum::{Self, Forum};
use forum::post;
use forum::sender::{Self, Sender};
use forum::thread::{Self, Thread};
use std::ascii;
use sui::clock::{Self, Clock};
use sui::test_scenario;
use sui::tx_context;

const ADMIN_PK: u256 = 21;
const BOARD_MOD_PK: u256 = 22;
const THREAD_ADMIN_PK: u256 = 23;
const THREAD_MOD_PK: u256 = 24;
const USER_PK: u256 = 25;

fun actor(pk: u256): Sender {
    sender::new(pk, 0)
}

fun fixture(ctx: &mut TxContext): (Forum, Board, Thread, Clock) {
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
    let clock = clock::create_for_testing(ctx);
    (forum, board, thread, clock)
}

#[test]
fun thread_allowed_events_for_forum_admin() {
    let mut ctx = tx_context::dummy();
    let (forum, board, mut thread, clock) = fixture(&mut ctx);
    let admin = actor(ADMIN_PK);
    let moderator = actor(THREAD_MOD_PK).addr();

    thread.apply(
        &forum,
        &board,
        thread::add_moderator(admin, b"1", moderator),
    );
    assert!(thread.mods().contains(moderator));
    thread.apply(
        &forum,
        &board,
        thread::del_moderator(admin, b"2", moderator),
    );
    assert!(!thread.mods().contains(moderator));

    thread.apply(
        &forum,
        &board,
        thread::set_pinned(admin, b"3", true),
    );
    thread.apply(
        &forum,
        &board,
        thread::set_topic(admin, b"4", option::some(100)),
    );
    thread.apply(
        &forum,
        &board,
        thread::set_admin(admin, b"5", option::some(actor(THREAD_ADMIN_PK).addr())),
    );
    assert!(*thread.pinned());
    assert!(*thread.topic_hash() == option::some(100));
    assert!(*thread.admin() == option::some(actor(THREAD_ADMIN_PK).addr()));

    thread.apply(
        &forum,
        &board,
        thread::set_closed(admin, b"6", true),
    );
    thread.apply(
        &forum,
        &board,
        thread::set_deleted(admin, b"7", true),
    );
    assert!(*thread.closed());
    assert!(*thread.deleted());

    clock.destroy_for_testing();
    thread.share();
    board.share();
    forum.share();
}

#[test]
fun thread_admin_and_moderator_allowed_subsets() {
    let mut ctx = tx_context::dummy();
    let (forum, board, mut thread, clock) = fixture(&mut ctx);
    let forum_admin = actor(ADMIN_PK);
    let thread_admin = actor(THREAD_ADMIN_PK);
    let thread_mod = actor(THREAD_MOD_PK);

    thread.apply(
        &forum,
        &board,
        thread::set_admin(forum_admin, b"1", option::some(thread_admin.addr())),
    );
    thread.apply(
        &forum,
        &board,
        thread::add_moderator(thread_admin, b"2", thread_mod.addr()),
    );
    thread.apply(
        &forum,
        &board,
        thread::set_topic(thread_mod, b"3", option::some(10)),
    );
    thread.apply(
        &forum,
        &board,
        thread::set_closed(thread_mod, b"4", true),
    );
    thread.apply(
        &forum,
        &board,
        thread::set_deleted(thread_mod, b"5", true),
    );

    assert!(thread.mods().contains(thread_mod.addr()));
    assert!(*thread.topic_hash() == option::some(10));
    assert!(*thread.deleted());

    clock.destroy_for_testing();
    thread.share();
    board.share();
    forum.share();
}

#[test]
fun board_moderator_can_pin_thread() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, mut thread, clock) = fixture(&mut ctx);
    let board_mod = actor(BOARD_MOD_PK);
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::add_moderator(actor(ADMIN_PK), b"1", board_mod.addr()),
    );
    thread.apply(
        &forum,
        &board,
        thread::set_pinned(board_mod, b"2", true),
    );
    assert!(*thread.pinned());

    clock.destroy_for_testing();
    thread.share();
    board.share();
    forum.share();
}

#[test]
fun thread_user_can_set_topic_during_genesis() {
    let mut ctx = tx_context::dummy();
    let (forum, board, mut thread, clock) = fixture(&mut ctx);
    thread.apply(
        &forum,
        &board,
        thread::set_topic(actor(USER_PK), b"1", option::some(10)),
    );
    assert!(*thread.topic_hash() == option::some(10));

    clock.destroy_for_testing();
    thread.share();
    board.share();
    forum.share();
}

#[test]
fun thread_post_ban_unban_allowed_events() {
    let mut ctx = tx_context::dummy();
    let (forum, board, mut thread, clock) = fixture(&mut ctx);
    let admin = actor(ADMIN_PK);
    let mut post = post::new(
        &mut ctx,
        admin,
        b"post",
        thread.id(),
        1,
        0,
        option::some(1),
        vector[],
        vector[],
    );
    let key = bans::key(thread.id(), 32, 100);

    thread.apply_post(
        &clock,
        &forum,
        &board,
        &mut post,
        thread::ban(admin, b"1", key, bans::value(2, 100)),
    );
    assert!(post.banned().borrow() == key);
    thread.apply_post(
        &clock,
        &forum,
        &board,
        &mut post,
        thread::unban(admin, b"2", key),
    );
    assert!(post.banned().is_none());

    clock.destroy_for_testing();
    post.share();
    thread.share();
    board.share();
    forum.share();
}

#[test]
#[expected_failure(abort_code = 14)]
fun thread_user_cannot_add_moderator() {
    let mut ctx = tx_context::dummy();
    let (forum, board, mut thread, _clock) = fixture(&mut ctx);
    thread.apply(
        &forum,
        &board,
        thread::add_moderator(actor(USER_PK), b"1", actor(THREAD_MOD_PK).addr()),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 14)]
fun thread_user_cannot_close_thread() {
    let mut ctx = tx_context::dummy();
    let (forum, board, mut thread, _clock) = fixture(&mut ctx);
    thread.apply(
        &forum,
        &board,
        thread::set_closed(actor(USER_PK), b"1", true),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 14)]
fun thread_user_cannot_pin_thread() {
    let mut ctx = tx_context::dummy();
    let (forum, board, mut thread, _clock) = fixture(&mut ctx);
    thread.apply(
        &forum,
        &board,
        thread::set_pinned(actor(USER_PK), b"1", true),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 14)]
fun thread_user_cannot_change_topic_after_genesis() {
    let admin = actor(ADMIN_PK);
    let mut scenario = test_scenario::begin(admin.addr());
    let mut forum = forum::new(scenario.ctx(), admin, b"forum", admin.addr());
    let board = board::new(
        scenario.ctx(),
        admin,
        b"board",
        ascii::string(b"test"),
    );
    forum.boards_mut().add(ascii::string(b"test"), board.id());
    thread::new(
        scenario.ctx(),
        admin,
        b"thread",
        board.id(),
        1,
        option::none(),
    ).share();

    scenario.next_tx(actor(USER_PK).addr());
    let mut thread = scenario.take_shared<Thread>();
    thread.apply(
        &forum,
        &board,
        thread::set_topic(actor(USER_PK), b"1", option::some(1)),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 14)]
fun thread_user_cannot_set_admin() {
    let mut ctx = tx_context::dummy();
    let (forum, board, mut thread, _clock) = fixture(&mut ctx);
    thread.apply(
        &forum,
        &board,
        thread::set_admin(actor(USER_PK), b"1", option::some(actor(USER_PK).addr())),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 14)]
fun thread_moderator_cannot_add_moderator() {
    let mut ctx = tx_context::dummy();
    let (forum, board, mut thread, _clock) = fixture(&mut ctx);
    let admin = actor(ADMIN_PK);
    let thread_admin = actor(THREAD_ADMIN_PK);
    let thread_mod = actor(THREAD_MOD_PK);
    thread.apply(
        &forum,
        &board,
        thread::set_admin(admin, b"1", option::some(thread_admin.addr())),
    );
    thread.apply(
        &forum,
        &board,
        thread::add_moderator(thread_admin, b"2", thread_mod.addr()),
    );
    thread.apply(
        &forum,
        &board,
        thread::add_moderator(thread_mod, b"3", actor(USER_PK).addr()),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 14)]
fun thread_moderator_cannot_pin_thread() {
    let mut ctx = tx_context::dummy();
    let (forum, board, mut thread, _clock) = fixture(&mut ctx);
    let admin = actor(ADMIN_PK);
    let thread_mod = actor(THREAD_MOD_PK);
    thread.apply(
        &forum,
        &board,
        thread::add_moderator(admin, b"1", thread_mod.addr()),
    );
    thread.apply(
        &forum,
        &board,
        thread::set_pinned(thread_mod, b"2", true),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 14)]
fun thread_post_ban_rejects_user() {
    let mut ctx = tx_context::dummy();
    let (forum, board, mut thread, clock) = fixture(&mut ctx);
    let admin = actor(ADMIN_PK);
    let mut post = post::new(
        &mut ctx,
        admin,
        b"post",
        thread.id(),
        1,
        0,
        option::some(1),
        vector[],
        vector[],
    );
    let key = bans::key(thread.id(), 32, 100);
    thread.apply_post(
        &clock,
        &forum,
        &board,
        &mut post,
        thread::ban(actor(USER_PK), b"1", key, bans::value(2, 100)),
    );
    abort
}
