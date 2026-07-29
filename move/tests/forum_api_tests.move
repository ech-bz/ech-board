#[test_only]
module forum::forum_api_tests;

use forum::bans;
use forum::board;
use forum::forum::{Self, Forum};
use forum::post;
use forum::sender::{Self, Sender};
use forum::thread;
use std::ascii;
use sui::tx_context;

const ADMIN_PK: u256 = 1;
const MOD_PK: u256 = 2;
const USER_PK: u256 = 3;

fun actor(pk: u256): Sender {
    sender::new(pk, 0)
}

fun fixture(ctx: &mut TxContext): Forum {
    let admin = actor(ADMIN_PK);
    forum::new(ctx, admin, b"genesis", admin.addr())
}

#[test]
fun forum_admin_allowed_events() {
    let mut ctx = tx_context::dummy();
    let mut forum = fixture(&mut ctx);
    let admin = actor(ADMIN_PK);
    let moderator = actor(MOD_PK).addr();
    let clock = sui::clock::create_for_testing(&mut ctx);

    forum.apply(
        &mut ctx,
        &clock,
        forum::add_moderator(admin, b"1", moderator),
    );
    assert!(forum.mods().contains(moderator));

    forum.apply(
        &mut ctx,
        &clock,
        forum::del_moderator(admin, b"2", moderator),
    );
    assert!(!forum.mods().contains(moderator));

    let slug = ascii::string(b"test");
    forum.apply(
        &mut ctx,
        &clock,
        forum::new_board(admin, b"3", copy slug, 4, 100, option::some(11)),
    );
    assert!(forum.boards().contains(slug));

    forum.apply(
        &mut ctx,
        &clock,
        forum::set_timestamp_precision(admin, b"4", 60_000),
    );
    assert!(*forum.timestamp_precision() == 60_000);

    clock.destroy_for_testing();
    forum.share();
}

#[test]
fun forum_moderator_can_create_board_and_set_precision() {
    let mut ctx = tx_context::dummy();
    let mut forum = fixture(&mut ctx);
    let admin = actor(ADMIN_PK);
    let moderator = actor(MOD_PK);

    let clock = sui::clock::create_for_testing(&mut ctx);
    forum.apply(
        &mut ctx,
        &clock,
        forum::add_moderator(admin, b"1", moderator.addr()),
    );
    let slug = ascii::string(b"modboard");
    forum.apply(
        &mut ctx,
        &clock,
        forum::new_board(moderator, b"2", copy slug, 1, 10, option::none()),
    );
    forum.apply(
        &mut ctx,
        &clock,
        forum::set_timestamp_precision(moderator, b"3", 1_000),
    );

    assert!(forum.boards().contains(slug));
    assert!(*forum.timestamp_precision() == 1_000);

    clock.destroy_for_testing();
    forum.share();
}

#[test]
#[expected_failure(abort_code = 14)]
fun forum_moderator_cannot_add_moderator() {
    let mut ctx = tx_context::dummy();
    let mut forum = fixture(&mut ctx);
    let admin = actor(ADMIN_PK);
    let moderator = actor(MOD_PK);
    let clock = sui::clock::create_for_testing(&mut ctx);

    forum.apply(
        &mut ctx,
        &clock,
        forum::add_moderator(admin, b"1", moderator.addr()),
    );
    forum.apply(
        &mut ctx,
        &clock,
        forum::add_moderator(moderator, b"2", actor(USER_PK).addr()),
    );

    abort
}

#[test]
#[expected_failure(abort_code = 14)]
fun forum_user_cannot_create_board() {
    let mut ctx = tx_context::dummy();
    let mut forum = fixture(&mut ctx);
    let clock = sui::clock::create_for_testing(&mut ctx);
    forum.apply(
        &mut ctx,
        &clock,
        forum::new_board(
            actor(USER_PK),
            b"1",
            ascii::string(b"forbidden"),
            1,
            10,
            option::none(),
        ),
    );

    abort
}

#[test]
#[expected_failure(abort_code = 14)]
fun forum_user_cannot_set_timestamp_precision() {
    let mut ctx = tx_context::dummy();
    let mut forum = fixture(&mut ctx);
    let clock = sui::clock::create_for_testing(&mut ctx);
    forum.apply(
        &mut ctx,
        &clock,
        forum::set_timestamp_precision(actor(USER_PK), b"1", 1_000),
    );

    abort
}

#[test]
#[expected_failure(abort_code = 8)]
fun forum_rejects_invalid_board_slug() {
    let mut ctx = tx_context::dummy();
    let mut forum = fixture(&mut ctx);
    let clock = sui::clock::create_for_testing(&mut ctx);
    forum.apply(
        &mut ctx,
        &clock,
        forum::new_board(
            actor(ADMIN_PK),
            b"1",
            ascii::string(b"INVALID"),
            1,
            10,
            option::none(),
        ),
    );

    abort
}

#[test]
fun forum_post_ban_unban_allowed_events() {
    let mut ctx = tx_context::dummy();
    let admin = actor(ADMIN_PK);
    let mut forum = fixture(&mut ctx);
    let board = board::new(
        &mut ctx,
        admin,
        b"board",
        ascii::string(b"test"),
    );
    forum.boards_mut().add(ascii::string(b"test"), board.id());
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
        option::some(7),
        vector[],
        vector[],
    );
    let clock = sui::clock::create_for_testing(&mut ctx);
    let key = bans::key(forum.id(), 32, 77);

    forum.apply_post(
        &clock,
        &board,
        &thread,
        &mut post,
        forum::ban(admin, b"1", key, bans::value(9, 100)),
    );
    assert!(post.banned().borrow() == key);

    forum.apply_post(
        &clock,
        &board,
        &thread,
        &mut post,
        forum::unban(admin, b"2", key),
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
fun forum_post_ban_rejects_user() {
    let mut ctx = tx_context::dummy();
    let admin = actor(ADMIN_PK);
    let mut forum = fixture(&mut ctx);
    let board = board::new(
        &mut ctx,
        admin,
        b"board",
        ascii::string(b"test"),
    );
    forum.boards_mut().add(ascii::string(b"test"), board.id());
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
        option::some(7),
        vector[],
        vector[],
    );
    let clock = sui::clock::create_for_testing(&mut ctx);
    let key = bans::key(forum.id(), 32, 77);

    forum.apply_post(
        &clock,
        &board,
        &thread,
        &mut post,
        forum::ban(actor(USER_PK), b"1", key, bans::value(9, 100)),
    );

    abort
}
