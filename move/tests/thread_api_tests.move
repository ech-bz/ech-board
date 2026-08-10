#[test_only]
module forum::thread_api_tests;

use forum::bans;
use forum::board::{Self, Board};
use forum::forum::{Self, Forum};
use forum::post;
use forum::responses::{Self, Responses};
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

fun uid(value: vector<u8>): Responses {
    responses::new(option::some(value), option::none(), option::none(), option::none())
}

fun fixture(ctx: &mut TxContext): (Forum, Board, Thread, Clock) {
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
        &mut ctx,
        &forum,
        &board,
        thread::add_moderator(uid(b"1"), admin, moderator),
    );
    assert!(thread.mods().contains(moderator));
    thread.apply(
        &mut ctx,
        &forum,
        &board,
        thread::del_moderator(uid(b"2"), admin, moderator),
    );
    assert!(!thread.mods().contains(moderator));

    thread.apply(
        &mut ctx,
        &forum,
        &board,
        thread::set_topic(uid(b"4"), admin, option::some(100)),
    );
    thread.apply(
        &mut ctx,
        &forum,
        &board,
        thread::set_admin(uid(b"5"), admin, option::some(actor(THREAD_ADMIN_PK).addr())),
    );
    assert!(*thread.topic_hash() == option::some(100));
    assert!(*thread.admin() == option::some(actor(THREAD_ADMIN_PK).addr()));

    thread.apply(
        &mut ctx,
        &forum,
        &board,
        thread::set_closed(uid(b"6"), admin, true),
    );
    thread.apply(
        &mut ctx,
        &forum,
        &board,
        thread::set_deleted(uid(b"7"), admin, true),
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
        &mut ctx,
        &forum,
        &board,
        thread::set_admin(uid(b"1"), forum_admin, option::some(thread_admin.addr())),
    );
    thread.apply(
        &mut ctx,
        &forum,
        &board,
        thread::add_moderator(uid(b"2"), thread_admin, thread_mod.addr()),
    );
    thread.apply(
        &mut ctx,
        &forum,
        &board,
        thread::set_topic(uid(b"3"), thread_mod, option::some(10)),
    );
    thread.apply(
        &mut ctx,
        &forum,
        &board,
        thread::set_closed(uid(b"4"), thread_mod, true),
    );
    thread.apply(
        &mut ctx,
        &forum,
        &board,
        thread::set_deleted(uid(b"5"), thread_mod, true),
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
        board::add_moderator(uid(b"1"), actor(ADMIN_PK), board_mod.addr()),
    );
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_pinned(uid(b"2"), board_mod, vector[thread.id()]),
    );
    assert!(board.pinned().contains(&thread.id()));

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
        &mut ctx,
        &forum,
        &board,
        thread::set_topic(uid(b"1"), actor(USER_PK), option::some(10)),
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
        uid(b"post"),
        admin,
        thread.id(),
        1,
        0,
        option::none(),
        option::some(1),
        vector[],
        vector[],
        false,
    );
    let key = bans::key(thread.id(), 32, 100);

    thread.apply_post(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &mut post,
        thread::ban(uid(b"1"), admin, key, bans::value(2, 100)),
    );
    assert!(post.banned().borrow() == key);
    thread.apply_post(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &mut post,
        thread::unban(uid(b"2"), admin, key),
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
        &mut ctx,
        &forum,
        &board,
        thread::add_moderator(uid(b"1"), actor(USER_PK), actor(THREAD_MOD_PK).addr()),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 14)]
fun thread_user_cannot_close_thread() {
    let mut ctx = tx_context::dummy();
    let (forum, board, mut thread, _clock) = fixture(&mut ctx);
    thread.apply(
        &mut ctx,
        &forum,
        &board,
        thread::set_closed(uid(b"1"), actor(USER_PK), true),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 14)]
fun board_user_cannot_pin_thread() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, mut thread, clock) = fixture(&mut ctx);
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_pinned(uid(b"1"), actor(USER_PK), vector[thread.id()]),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 14)]
fun thread_user_cannot_change_topic_after_genesis() {
    let admin = actor(ADMIN_PK);
    let mut scenario = test_scenario::begin(admin.addr());
    let mut forum = forum::new(scenario.ctx(), uid(b"forum"), admin, admin.addr());
    let board = board::new(
        scenario.ctx(),
        uid(b"board"),
        admin,
        ascii::string(b"test"),
    );
    forum.boards_mut().add(ascii::string(b"test"), board.id());
    thread::new(
        scenario.ctx(),
        uid(b"thread"),
        admin,
        board.id(),
        1,
        option::none(),
    ).share();

    scenario.next_tx(actor(USER_PK).addr());
    let mut thread = scenario.take_shared<Thread>();
    thread.apply(
        scenario.ctx(),
        &forum,
        &board,
        thread::set_topic(uid(b"1"), actor(USER_PK), option::some(1)),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 14)]
fun thread_user_cannot_set_admin() {
    let mut ctx = tx_context::dummy();
    let (forum, board, mut thread, _clock) = fixture(&mut ctx);
    thread.apply(
        &mut ctx,
        &forum,
        &board,
        thread::set_admin(uid(b"1"), actor(USER_PK), option::some(actor(USER_PK).addr())),
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
        &mut ctx,
        &forum,
        &board,
        thread::set_admin(uid(b"1"), admin, option::some(thread_admin.addr())),
    );
    thread.apply(
        &mut ctx,
        &forum,
        &board,
        thread::add_moderator(uid(b"2"), thread_admin, thread_mod.addr()),
    );
    thread.apply(
        &mut ctx,
        &forum,
        &board,
        thread::add_moderator(uid(b"3"), thread_mod, actor(USER_PK).addr()),
    );
    abort
}

#[test]
fun thread_post_set_deleted_updates_counter() {
    let mut ctx = tx_context::dummy();
    let (forum, board, mut thread, clock) = fixture(&mut ctx);
    let author = actor(USER_PK);
    let mut post1 = post::new(
        &mut ctx,
        uid(b"p1"),
        author,
        thread.id(),
        1,
        0,
        option::none(),
        option::some(1),
        vector[],
        vector[],
        false,
    );
    let mut post2 = post::new(
        &mut ctx,
        uid(b"p2"),
        author,
        thread.id(),
        2,
        0,
        option::none(),
        option::some(2),
        vector[],
        vector[],
        false,
    );

    thread.apply(&mut ctx, &forum, &board, thread::new_post(uid(b"1"), author, post1.id()));
    thread.apply(&mut ctx, &forum, &board, thread::new_post(uid(b"2"), author, post2.id()));
    assert!(*thread.posts_deleted() == 0);

    thread.apply_post(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &mut post1,
        thread::post_set_deleted(uid(b"3"), author, true),
    );
    assert!(*post1.deleted());
    assert!(*thread.posts_deleted() == 1);

    thread.apply_post(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &mut post1,
        thread::post_set_deleted(uid(b"4"), author, false),
    );
    assert!(!*post1.deleted());
    assert!(*thread.posts_deleted() == 0);

    clock.destroy_for_testing();
    post1.share();
    post2.share();
    thread.share();
    board.share();
    forum.share();
}

#[test]
fun post_set_text_none_auto_deletes_empty_post() {
    let mut ctx = tx_context::dummy();
    let (forum, board, mut thread, clock) = fixture(&mut ctx);
    let author = actor(USER_PK);
    let mut post = post::new(
        &mut ctx,
        uid(b"post"),
        author,
        thread.id(),
        1,
        0,
        option::none(),
        option::some(1),
        vector[],
        vector[],
        false,
    );
    thread.apply_post(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &mut post,
        thread::post_set_text(uid(b"1"), author, option::none()),
    );
    assert!(post.text_hash().is_none());
    assert!(*post.deleted());
    assert!(*thread.posts_deleted() == 1);

    clock.destroy_for_testing();
    post.share();
    thread.share();
    board.share();
    forum.share();
}

#[test]
#[expected_failure(abort_code = 14)]
fun thread_moderator_cannot_pin_thread() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, mut thread, clock) = fixture(&mut ctx);
    let admin = actor(ADMIN_PK);
    let thread_mod = actor(THREAD_MOD_PK);
    thread.apply(
        &mut ctx,
        &forum,
        &board,
        thread::add_moderator(uid(b"1"), admin, thread_mod.addr()),
    );
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_pinned(uid(b"2"), thread_mod, vector[thread.id()]),
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
        uid(b"post"),
        admin,
        thread.id(),
        1,
        0,
        option::none(),
        option::some(1),
        vector[],
        vector[],
        false,
    );
    let key = bans::key(thread.id(), 32, 100);
    thread.apply_post(
        &mut ctx,
        &clock,
        &forum,
        &board,
        &mut post,
        thread::ban(uid(b"1"), actor(USER_PK), key, bans::value(2, 100)),
    );
    abort
}
