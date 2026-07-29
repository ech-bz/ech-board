#[test_only]
module forum::board_api_tests;

use forum::bans;
use forum::board::{Self, Board};
use forum::forum::{Self, Forum};
use forum::post;
use forum::sender::{Self, Sender};
use forum::thread;
use std::ascii;
use sui::clock::{Self, Clock};
use sui::test_scenario;
use sui::tx_context;

const ADMIN_PK: u256 = 11;
const FORUM_MOD_PK: u256 = 12;
const BOARD_MOD_PK: u256 = 13;
const USER_PK: u256 = 14;

fun actor(pk: u256): Sender {
    sender::new(pk, 0)
}

fun fixture(ctx: &mut TxContext): (Forum, Board, Clock) {
    let admin = actor(ADMIN_PK);
    let mut forum = forum::new(ctx, admin, b"forum", admin.addr());
    let board = board::new(ctx, admin, b"board", ascii::string(b"test"));
    forum.boards_mut().add(ascii::string(b"test"), board.id());
    let clock = clock::create_for_testing(ctx);
    (forum, board, clock)
}

#[test]
fun board_allowed_events() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, clock) = fixture(&mut ctx);
    let admin = actor(ADMIN_PK);
    let board_mod = actor(BOARD_MOD_PK).addr();

    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::add_moderator(admin, b"1", board_mod),
    );
    assert!(board.mods().contains(board_mod));
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::del_moderator(admin, b"2", board_mod),
    );
    assert!(!board.mods().contains(board_mod));

    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_max_media(admin, b"3", 2),
    );
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_bump_limit(admin, b"4", 10),
    );
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_description(admin, b"5", option::some(100)),
    );
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_ignore_forum_bans(admin, b"6", true),
    );
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_reactions(admin, b"7", vector[200, 201]),
    );
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::new_thread(
            actor(USER_PK),
            b"8",
            option::some(300),
            option::some(301),
            vector[302],
            vector[303],
        ),
    );

    assert!(*board.max_media() == 2);
    assert!(*board.bump_limit() == 10);
    assert!(*board.desc_hash() == option::some(100));
    assert!(*board.ignore_forum_bans());
    assert!(board.reactions().contains(&200));
    assert!(board.threads().length() == 1);
    assert!(board.posts().length() == 1);
    assert!(board.bumps().next() == 2);

    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_closed(admin, b"9", true),
    );
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_deleted(admin, b"10", true),
    );
    assert!(*board.closed());
    assert!(*board.deleted());

    clock.destroy_for_testing();
    board.share();
    forum.share();
}

#[test]
fun board_moderator_allowed_subset() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, clock) = fixture(&mut ctx);
    let admin = actor(ADMIN_PK);
    let board_mod = actor(BOARD_MOD_PK);

    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::add_moderator(admin, b"1", board_mod.addr()),
    );
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_description(board_mod, b"2", option::some(10)),
    );
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_reactions(board_mod, b"3", vector[20]),
    );

    assert!(*board.desc_hash() == option::some(10));
    assert!(board.reactions().contains(&20));

    clock.destroy_for_testing();
    board.share();
    forum.share();
}

#[test]
fun forum_moderator_can_manage_board_admin_settings() {
    let mut ctx = tx_context::dummy();
    let (mut forum, mut board, clock) = fixture(&mut ctx);
    let admin = actor(ADMIN_PK);
    let forum_mod = actor(FORUM_MOD_PK);

    forum.apply(
        &mut ctx,
        &clock,
        forum::add_moderator(admin, b"1", forum_mod.addr()),
    );
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_max_media(forum_mod, b"2", 3),
    );
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_bump_limit(forum_mod, b"3", 50),
    );
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_ignore_forum_bans(forum_mod, b"4", true),
    );

    assert!(*board.max_media() == 3);
    assert!(*board.bump_limit() == 50);
    assert!(*board.ignore_forum_bans());

    clock.destroy_for_testing();
    board.share();
    forum.share();
}

#[test]
fun board_apply_thread_creates_posts_and_bumps_within_limit() {
    let admin = actor(ADMIN_PK);
    let mut scenario = test_scenario::begin(admin.addr());
    let mut forum = forum::new(scenario.ctx(), admin, b"forum", admin.addr());
    let mut board = board::new(
        scenario.ctx(),
        admin,
        b"board",
        ascii::string(b"test"),
    );
    forum.boards_mut().add(ascii::string(b"test"), board.id());
    let mut thread = thread::new(
        scenario.ctx(),
        admin,
        b"thread",
        board.id(),
        1,
        option::none(),
    );
    let thread_id = thread.id();
    let clock = clock::create_for_testing(scenario.ctx());

    board.apply(
        scenario.ctx(),
        &clock,
        &forum,
        board::set_bump_limit(admin, b"1", 1),
    );

    board.apply_thread(
        scenario.ctx(),
        &clock,
        &forum,
        &mut thread,
        board::new_post(
            actor(USER_PK),
            b"2",
            thread_id,
            option::some(10),
            vector[],
            vector[],
        ),
    );
    let op = *thread.op();
    thread.share();
    board.share();
    forum.share();

    scenario.next_tx(actor(USER_PK).addr());
    let forum = scenario.take_shared<Forum>();
    let mut board = scenario.take_shared<Board>();
    let mut thread = scenario.take_shared<thread::Thread>();
    board.apply_thread(
        scenario.ctx(),
        &clock,
        &forum,
        &mut thread,
        board::new_post(
            actor(USER_PK),
            b"3",
            thread_id,
            option::some(11),
            vector[],
            vector[],
        ),
    );

    assert!(board.posts().length() == 2);
    assert!(thread.posts().next() == 3);
    assert!(*thread.op() == op);
    assert!(thread.last3().length() == 1);
    assert!(board.bumps().next() == 2);

    clock.destroy_for_testing();
    test_scenario::return_shared(thread);
    test_scenario::return_shared(board);
    test_scenario::return_shared(forum);
    scenario.end();
}

#[test]
fun board_apply_thread_allows_forum_moderator_on_closed_board_and_thread() {
    let mut ctx = tx_context::dummy();
    let (mut forum, mut board, clock) = fixture(&mut ctx);
    let admin = actor(ADMIN_PK);
    let moderator = actor(FORUM_MOD_PK);
    let mut thread = thread::new(
        &mut ctx,
        admin,
        b"thread",
        board.id(),
        1,
        option::none(),
    );
    let thread_id = thread.id();

    forum.apply(
        &mut ctx,
        &clock,
        forum::add_moderator(admin, b"1", moderator.addr()),
    );
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_closed(admin, b"2", true),
    );
    thread.apply(
        &forum,
        &board,
        thread::set_closed(admin, b"3", true),
    );
    board.apply_thread(
        &mut ctx,
        &clock,
        &forum,
        &mut thread,
        board::new_post(
            moderator,
            b"4",
            thread_id,
            option::some(10),
            vector[],
            vector[],
        ),
    );

    assert!(board.posts().length() == 1);
    assert!(thread.posts().next() == 2);

    clock.destroy_for_testing();
    thread.share();
    board.share();
    forum.share();
}

#[test]
fun board_post_ban_unban_allowed_events() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, clock) = fixture(&mut ctx);
    let admin = actor(ADMIN_PK);
    let thread = thread::new(
        &mut ctx,
        admin,
        b"thread",
        board.id(),
        1,
        option::none(),
    );
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
    let key = bans::key(board.id(), 32, 100);

    board.apply_post(
        &clock,
        &forum,
        &thread,
        &mut post,
        board::ban(admin, b"1", key, bans::value(2, 100)),
    );
    assert!(post.banned().borrow() == key);
    board.apply_post(
        &clock,
        &forum,
        &thread,
        &mut post,
        board::unban(admin, b"2", key),
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
fun board_user_cannot_add_moderator() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, clock) = fixture(&mut ctx);
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::add_moderator(actor(USER_PK), b"1", actor(BOARD_MOD_PK).addr()),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 14)]
fun board_moderator_cannot_change_admin_settings() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, clock) = fixture(&mut ctx);
    let admin = actor(ADMIN_PK);
    let board_mod = actor(BOARD_MOD_PK);
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::add_moderator(admin, b"1", board_mod.addr()),
    );
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_max_media(board_mod, b"2", 10),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 14)]
fun board_user_cannot_change_moderator_settings() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, clock) = fixture(&mut ctx);
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_description(actor(USER_PK), b"1", option::some(10)),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 10)]
fun board_new_thread_requires_media_when_enabled() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, clock) = fixture(&mut ctx);
    let admin = actor(ADMIN_PK);
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_max_media(admin, b"1", 1),
    );
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::new_thread(
            actor(USER_PK),
            b"2",
            option::none(),
            option::some(1),
            vector[],
            vector[],
        ),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 9)]
fun board_new_post_rejects_media_over_limit() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, clock) = fixture(&mut ctx);
    let admin = actor(ADMIN_PK);
    let mut thread = thread::new(
        &mut ctx,
        admin,
        b"thread",
        board.id(),
        1,
        option::none(),
    );
    let thread_id = thread.id();
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_max_media(admin, b"1", 1),
    );
    board.apply_thread(
        &mut ctx,
        &clock,
        &forum,
        &mut thread,
        board::new_post(
            actor(USER_PK),
            b"2",
            thread_id,
            option::none(),
            vector[1, 2],
            vector[],
        ),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 11)]
fun board_new_post_rejects_empty_post() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, clock) = fixture(&mut ctx);
    let mut thread = thread::new(
        &mut ctx,
        actor(ADMIN_PK),
        b"thread",
        board.id(),
        1,
        option::none(),
    );
    let thread_id = thread.id();
    board.apply_thread(
        &mut ctx,
        &clock,
        &forum,
        &mut thread,
        board::new_post(
            actor(USER_PK),
            b"1",
            thread_id,
            option::none(),
            vector[],
            vector[],
        ),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 12)]
fun board_new_post_rejects_user_when_board_closed() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, clock) = fixture(&mut ctx);
    let admin = actor(ADMIN_PK);
    let mut thread = thread::new(
        &mut ctx,
        admin,
        b"thread",
        board.id(),
        1,
        option::none(),
    );
    let thread_id = thread.id();
    board.apply(
        &mut ctx,
        &clock,
        &forum,
        board::set_closed(admin, b"1", true),
    );
    board.apply_thread(
        &mut ctx,
        &clock,
        &forum,
        &mut thread,
        board::new_post(
            actor(USER_PK),
            b"2",
            thread_id,
            option::some(1),
            vector[],
            vector[],
        ),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 13)]
fun board_new_post_rejects_user_when_thread_closed() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, clock) = fixture(&mut ctx);
    let admin = actor(ADMIN_PK);
    let mut thread = thread::new(
        &mut ctx,
        admin,
        b"thread",
        board.id(),
        1,
        option::none(),
    );
    let thread_id = thread.id();
    thread.apply(
        &forum,
        &board,
        thread::set_closed(admin, b"1", true),
    );
    board.apply_thread(
        &mut ctx,
        &clock,
        &forum,
        &mut thread,
        board::new_post(
            actor(USER_PK),
            b"2",
            thread_id,
            option::some(1),
            vector[],
            vector[],
        ),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 15)]
fun board_new_post_rejects_event_thread_mismatch() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, clock) = fixture(&mut ctx);
    let mut thread = thread::new(
        &mut ctx,
        actor(ADMIN_PK),
        b"thread",
        board.id(),
        1,
        option::none(),
    );
    board.apply_thread(
        &mut ctx,
        &clock,
        &forum,
        &mut thread,
        board::new_post(
            actor(USER_PK),
            b"1",
            @0xdead,
            option::some(1),
            vector[],
            vector[],
        ),
    );
    abort
}

#[test]
#[expected_failure(abort_code = 14)]
fun board_post_ban_rejects_user() {
    let mut ctx = tx_context::dummy();
    let (forum, mut board, clock) = fixture(&mut ctx);
    let admin = actor(ADMIN_PK);
    let thread = thread::new(
        &mut ctx,
        admin,
        b"thread",
        board.id(),
        1,
        option::none(),
    );
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
    let key = bans::key(board.id(), 32, 100);
    board.apply_post(
        &clock,
        &forum,
        &thread,
        &mut post,
        board::ban(actor(USER_PK), b"1", key, bans::value(2, 100)),
    );
    abort
}
