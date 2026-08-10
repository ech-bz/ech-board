module forum::main;

use forum::board::Board;
use forum::entity::{Self, Entity};
use forum::forum::{Self, Forum};
use forum::intent;
use forum::post::Post;
use forum::responses;
use forum::sender;
use forum::sharded_counter::Shard;
use forum::thread::Thread;
use sui::clock::Clock;

fun init(ctx: &mut TxContext) {
    let admin = ctx.sender();
    forum::new(
        ctx,
        responses::new(option::some(vector[]), option::none(), option::none(), option::none()),
        sender::new(0, 0),
        admin,
    ).share();
}

public fun forum_apply_intent_uid(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    responses: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &mut Forum,
) {
    let intent = intent::decode(
        intent_bytes,
        "main",
        "forum_apply_intent_uid",
        signature,
        vector[intent::request_uid()],
        responses,
        vector[object::id(clock), object::id(nonce_shard), object::id(forum)],
        vector["upgrade", "add_moderator", "del_moderator", "new_board", "set_timestamp_precision"],
    );
    nonce_shard.inc_checked(&intent.sender().addr(), intent.nonce());
    forum.apply(ctx, clock, intent.into_event());
}

public fun forum_apply_post_intent_uid(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    responses: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &mut Forum,
    board: &Board,
    thread: &Thread,
    post: &mut Post,
) {
    let intent = intent::decode(
        intent_bytes,
        "main",
        "forum_apply_post_intent_uid",
        signature,
        vector[intent::request_uid()],
        responses,
        vector[
            object::id(clock),
            object::id(nonce_shard),
            object::id(forum),
            object::id(board),
            object::id(thread),
            object::id(post),
        ],
        vector["ban", "unban"],
    );
    nonce_shard.inc_checked(&intent.sender().addr(), intent.nonce());
    forum.apply_post(ctx, clock, board, thread, post, intent.into_event());
}

public fun board_apply_intent_uid(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    responses: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &Forum,
    board: &mut Board,
) {
    let intent = intent::decode(
        intent_bytes,
        "main",
        "board_apply_intent_uid",
        signature,
        vector[intent::request_uid()],
        responses,
        vector[object::id(clock), object::id(nonce_shard), object::id(forum), object::id(board)],
        vector[
            "upgrade",
            "add_moderator",
            "del_moderator",
            "set_max_media",
            "set_bump_limit",
            "set_closed",
            "set_deleted",
            "new_thread_migrate_v2",
            "set_description",
            "set_ignore_forum_bans",
            "set_reactions",
            "set_pinned",
        ],
    );
    nonce_shard.inc_checked(&intent.sender().addr(), intent.nonce());
    board.apply(ctx, clock, forum, intent.into_event());
}

public fun board_apply_intent_uid_geo(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    responses: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &Forum,
    board: &mut Board,
) {
    let intent = intent::decode(
        intent_bytes,
        "main",
        "board_apply_intent_uid_geo",
        signature,
        vector[intent::request_uid(), intent::request_geo()],
        responses,
        vector[object::id(clock), object::id(nonce_shard), object::id(forum), object::id(board)],
        vector["new_thread_migrate_v2"],
    );
    nonce_shard.inc_checked(&intent.sender().addr(), intent.nonce());
    board.apply(ctx, clock, forum, intent.into_event());
}

public fun board_apply_intent_uid_tripcode(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    responses: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &Forum,
    board: &mut Board,
) {
    let intent = intent::decode(
        intent_bytes,
        "main",
        "board_apply_intent_uid_tripcode",
        signature,
        vector[intent::request_uid(), intent::request_tripcode()],
        responses,
        vector[object::id(clock), object::id(nonce_shard), object::id(forum), object::id(board)],
        vector["new_thread_migrate_v2"],
    );
    nonce_shard.inc_checked(&intent.sender().addr(), intent.nonce());
    board.apply(ctx, clock, forum, intent.into_event());
}

public fun board_apply_intent_uid_geo_tripcode(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    responses: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &Forum,
    board: &mut Board,
) {
    let intent = intent::decode(
        intent_bytes,
        "main",
        "board_apply_intent_uid_geo_tripcode",
        signature,
        vector[intent::request_uid(), intent::request_geo(), intent::request_tripcode()],
        responses,
        vector[object::id(clock), object::id(nonce_shard), object::id(forum), object::id(board)],
        vector["new_thread_migrate_v2"],
    );
    nonce_shard.inc_checked(&intent.sender().addr(), intent.nonce());
    board.apply(ctx, clock, forum, intent.into_event());
}

public fun board_apply_intent_uid_captcha(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    responses: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &Forum,
    board: &mut Board,
) {
    let intent = intent::decode(
        intent_bytes,
        "main",
        "board_apply_intent_uid_captcha",
        signature,
        vector[intent::request_uid(), intent::request_captcha()],
        responses,
        vector[object::id(clock), object::id(nonce_shard), object::id(forum), object::id(board)],
        vector["new_thread_v2"],
    );
    nonce_shard.inc_checked(&intent.sender().addr(), intent.nonce());
    board.apply(ctx, clock, forum, intent.into_event());
}

public fun board_apply_intent_uid_geo_captcha(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    responses: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &Forum,
    board: &mut Board,
) {
    let intent = intent::decode(
        intent_bytes,
        "main",
        "board_apply_intent_uid_geo_captcha",
        signature,
        vector[intent::request_uid(), intent::request_geo(), intent::request_captcha()],
        responses,
        vector[object::id(clock), object::id(nonce_shard), object::id(forum), object::id(board)],
        vector["new_thread_v2"],
    );
    nonce_shard.inc_checked(&intent.sender().addr(), intent.nonce());
    board.apply(ctx, clock, forum, intent.into_event());
}

public fun board_apply_intent_uid_tripcode_captcha(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    responses: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &Forum,
    board: &mut Board,
) {
    let intent = intent::decode(
        intent_bytes,
        "main",
        "board_apply_intent_uid_tripcode_captcha",
        signature,
        vector[intent::request_uid(), intent::request_tripcode(), intent::request_captcha()],
        responses,
        vector[object::id(clock), object::id(nonce_shard), object::id(forum), object::id(board)],
        vector["new_thread_v2"],
    );
    nonce_shard.inc_checked(&intent.sender().addr(), intent.nonce());
    board.apply(ctx, clock, forum, intent.into_event());
}

public fun board_apply_intent_uid_geo_tripcode_captcha(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    responses: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &Forum,
    board: &mut Board,
) {
    let intent = intent::decode(
        intent_bytes,
        "main",
        "board_apply_intent_uid_geo_tripcode_captcha",
        signature,
        vector[intent::request_uid(), intent::request_geo(), intent::request_tripcode(), intent::request_captcha()],
        responses,
        vector[object::id(clock), object::id(nonce_shard), object::id(forum), object::id(board)],
        vector["new_thread_v2"],
    );
    nonce_shard.inc_checked(&intent.sender().addr(), intent.nonce());
    board.apply(ctx, clock, forum, intent.into_event());
}

public fun board_apply_thread_intent_uid(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    responses: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &Forum,
    board: &mut Board,
    thread: &mut Thread,
) {
    let intent = intent::decode(
        intent_bytes,
        "main",
        "board_apply_thread_intent_uid",
        signature,
        vector[intent::request_uid()],
        responses,
        vector[
            object::id(clock),
            object::id(nonce_shard),
            object::id(forum),
            object::id(board),
            object::id(thread),
        ],
        vector["new_post_migrate_v2"],
    );
    nonce_shard.inc_checked(&intent.sender().addr(), intent.nonce());
    board.apply_thread(ctx, clock, forum, thread, intent.into_event());
}

public fun board_apply_thread_intent_uid_geo(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    responses: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &Forum,
    board: &mut Board,
    thread: &mut Thread,
) {
    let intent = intent::decode(
        intent_bytes,
        "main",
        "board_apply_thread_intent_uid_geo",
        signature,
        vector[intent::request_uid(), intent::request_geo()],
        responses,
        vector[
            object::id(clock),
            object::id(nonce_shard),
            object::id(forum),
            object::id(board),
            object::id(thread),
        ],
        vector["new_post_migrate_v2"],
    );
    nonce_shard.inc_checked(&intent.sender().addr(), intent.nonce());
    board.apply_thread(ctx, clock, forum, thread, intent.into_event());
}

public fun board_apply_thread_intent_uid_tripcode(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    responses: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &Forum,
    board: &mut Board,
    thread: &mut Thread,
) {
    let intent = intent::decode(
        intent_bytes,
        "main",
        "board_apply_thread_intent_uid_tripcode",
        signature,
        vector[intent::request_uid(), intent::request_tripcode()],
        responses,
        vector[
            object::id(clock),
            object::id(nonce_shard),
            object::id(forum),
            object::id(board),
            object::id(thread),
        ],
        vector["new_post_migrate_v2"],
    );
    nonce_shard.inc_checked(&intent.sender().addr(), intent.nonce());
    board.apply_thread(ctx, clock, forum, thread, intent.into_event());
}

public fun board_apply_thread_intent_uid_geo_tripcode(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    responses: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &Forum,
    board: &mut Board,
    thread: &mut Thread,
) {
    let intent = intent::decode(
        intent_bytes,
        "main",
        "board_apply_thread_intent_uid_geo_tripcode",
        signature,
        vector[intent::request_uid(), intent::request_geo(), intent::request_tripcode()],
        responses,
        vector[
            object::id(clock),
            object::id(nonce_shard),
            object::id(forum),
            object::id(board),
            object::id(thread),
        ],
        vector["new_post_migrate_v2"],
    );
    nonce_shard.inc_checked(&intent.sender().addr(), intent.nonce());
    board.apply_thread(ctx, clock, forum, thread, intent.into_event());
}

public fun board_apply_thread_intent_uid_captcha(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    responses: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &Forum,
    board: &mut Board,
    thread: &mut Thread,
) {
    let intent = intent::decode(
        intent_bytes,
        "main",
        "board_apply_thread_intent_uid_captcha",
        signature,
        vector[intent::request_uid(), intent::request_captcha()],
        responses,
        vector[
            object::id(clock),
            object::id(nonce_shard),
            object::id(forum),
            object::id(board),
            object::id(thread),
        ],
        vector["new_post_v2"],
    );
    nonce_shard.inc_checked(&intent.sender().addr(), intent.nonce());
    board.apply_thread(ctx, clock, forum, thread, intent.into_event());
}

public fun board_apply_thread_intent_uid_geo_captcha(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    responses: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &Forum,
    board: &mut Board,
    thread: &mut Thread,
) {
    let intent = intent::decode(
        intent_bytes,
        "main",
        "board_apply_thread_intent_uid_geo_captcha",
        signature,
        vector[intent::request_uid(), intent::request_geo(), intent::request_captcha()],
        responses,
        vector[
            object::id(clock),
            object::id(nonce_shard),
            object::id(forum),
            object::id(board),
            object::id(thread),
        ],
        vector["new_post_v2"],
    );
    nonce_shard.inc_checked(&intent.sender().addr(), intent.nonce());
    board.apply_thread(ctx, clock, forum, thread, intent.into_event());
}

public fun board_apply_thread_intent_uid_tripcode_captcha(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    responses: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &Forum,
    board: &mut Board,
    thread: &mut Thread,
) {
    let intent = intent::decode(
        intent_bytes,
        "main",
        "board_apply_thread_intent_uid_tripcode_captcha",
        signature,
        vector[intent::request_uid(), intent::request_tripcode(), intent::request_captcha()],
        responses,
        vector[
            object::id(clock),
            object::id(nonce_shard),
            object::id(forum),
            object::id(board),
            object::id(thread),
        ],
        vector["new_post_v2"],
    );
    nonce_shard.inc_checked(&intent.sender().addr(), intent.nonce());
    board.apply_thread(ctx, clock, forum, thread, intent.into_event());
}

public fun board_apply_thread_intent_uid_geo_tripcode_captcha(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    responses: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &Forum,
    board: &mut Board,
    thread: &mut Thread,
) {
    let intent = intent::decode(
        intent_bytes,
        "main",
        "board_apply_thread_intent_uid_geo_tripcode_captcha",
        signature,
        vector[intent::request_uid(), intent::request_geo(), intent::request_tripcode(), intent::request_captcha()],
        responses,
        vector[
            object::id(clock),
            object::id(nonce_shard),
            object::id(forum),
            object::id(board),
            object::id(thread),
        ],
        vector["new_post_v2"],
    );
    nonce_shard.inc_checked(&intent.sender().addr(), intent.nonce());
    board.apply_thread(ctx, clock, forum, thread, intent.into_event());
}

public fun board_apply_post_intent_uid(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    responses: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &Forum,
    board: &mut Board,
    thread: &Thread,
    post: &mut Post,
) {
    let intent = intent::decode(
        intent_bytes,
        "main",
        "board_apply_post_intent_uid",
        signature,
        vector[intent::request_uid()],
        responses,
        vector[
            object::id(clock),
            object::id(nonce_shard),
            object::id(forum),
            object::id(board),
            object::id(thread),
            object::id(post),
        ],
        vector["ban", "unban"],
    );
    nonce_shard.inc_checked(&intent.sender().addr(), intent.nonce());
    board.apply_post(ctx, clock, forum, thread, post, intent.into_event());
}

public fun thread_apply_intent_uid(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    responses: vector<u8>,
    nonce_shard: &mut Shard<address>,
    forum: &Forum,
    board: &Board,
    thread: &mut Thread,
) {
    let intent = intent::decode(
        intent_bytes,
        "main",
        "thread_apply_intent_uid",
        signature,
        vector[intent::request_uid()],
        responses,
        vector[object::id(nonce_shard), object::id(forum), object::id(board), object::id(thread)],
        vector[
            "upgrade",
            "add_moderator",
            "del_moderator",
            "set_closed",
            "set_deleted",
            "set_topic",
            "set_admin",
        ],
    );
    nonce_shard.inc_checked(&intent.sender().addr(), intent.nonce());
    thread.apply(ctx, forum, board, intent.into_event());
}

public fun thread_apply_post_intent_uid(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    responses: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &Forum,
    board: &Board,
    thread: &mut Thread,
    post: &mut Post,
) {
    let intent = intent::decode(
        intent_bytes,
        "main",
        "thread_apply_post_intent_uid",
        signature,
        vector[intent::request_uid()],
        responses,
        vector[
            object::id(clock),
            object::id(nonce_shard),
            object::id(forum),
            object::id(board),
            object::id(thread),
            object::id(post),
        ],
        vector["ban", "unban"],
    );
    nonce_shard.inc_checked(&intent.sender().addr(), intent.nonce());
    thread.apply_post(ctx, clock, forum, board, post, intent.into_event());
}

public fun post_apply_intent_uid(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    responses: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &Forum,
    board: &Board,
    thread: &Thread,
    post: &mut Post,
) {
    let intent = intent::decode(
        intent_bytes,
        "main",
        "post_apply_intent_uid",
        signature,
        vector[intent::request_uid()],
        responses,
        vector[
            object::id(clock),
            object::id(nonce_shard),
            object::id(forum),
            object::id(board),
            object::id(thread),
            object::id(post),
        ],
        vector["upgrade", "set_deleted", "set_text", "ban_media", "unban_media"],
    );
    nonce_shard.inc_checked(&intent.sender().addr(), intent.nonce());
    post.apply(ctx, clock, forum, board, thread, intent.into_event());
}

public fun post_apply_intent_uid_ip32(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    responses: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &Forum,
    board: &Board,
    thread: &Thread,
    post: &mut Post,
) {
    let intent = intent::decode(
        intent_bytes,
        "main",
        "post_apply_intent_uid_ip32",
        signature,
        vector[intent::request_uid(), intent::request_ip32(post.id())],
        responses,
        vector[
            object::id(clock),
            object::id(nonce_shard),
            object::id(forum),
            object::id(board),
            object::id(thread),
            object::id(post),
        ],
        vector["set_reaction", "vote_v2"],
    );
    nonce_shard.inc_checked(&intent.sender().addr(), intent.nonce());
    post.apply(ctx, clock, forum, board, thread, intent.into_event());
}
