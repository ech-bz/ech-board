module forum::forum;

use forum::intent;
use forum::nonce::NonceGateShard;
use std::ascii::{Self, String};
use sui::event;
use sui::table::{Self, Table};

// ----- errors

public enum ForumError has copy, drop, store {
    BoardSlugInvalid,
    BoardSlugExists,
    ThreadBoardMismatch,
}

public fun code(self: ForumError): u64 {
    match (self) {
        ForumError::BoardSlugInvalid => 1,
        ForumError::BoardSlugExists => 2,
        ForumError::ThreadBoardMismatch => 3,
    }
}

// ----- board

public struct Board has key, store {
    id: UID,
    slug: String,
    next_post_number: u64,
}

public struct BoardCreated has copy, drop {
    board_id: ID,
}

public fun create_board_intent(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    shard: &mut NonceGateShard,
    boards: &mut BoardSlugRegistry,
) {
    let mut intent = intent::decode(
        intent_bytes,
        b"forum",
        b"create_board_intent",
        signature,
        vector[object::id(shard), object::id(boards)],
    );
    shard.advance(ctx, intent.sender(), intent.nonce());

    let slug = ascii::string(intent.bcs().peel_vec_u8());
    intent.end();

    create_board(ctx, boards, slug);
}

fun create_board(ctx: &mut TxContext, boards: &mut BoardSlugRegistry, slug: String) {
    assert!(
        slug.as_bytes().all!(|c| (*c >= 0x30 && *c <= 0x39) || (*c >= 0x61 && *c <= 0x7a)),
        ForumError::BoardSlugInvalid.code(),
    );
    assert!(slug.length() >= 1 && slug.length() <= 16, ForumError::BoardSlugInvalid.code());
    assert!(!boards.slugs.contains(slug), ForumError::BoardSlugExists.code());
    let board = Board {
        id: object::new(ctx),
        slug,
        next_post_number: 1,
    };
    event::emit(BoardCreated {
        board_id: object::id(&board),
    });
    boards.slugs.add(slug, object::id(&board));
    transfer::public_share_object(board);
}

// ----- thread

public struct Thread has key, store {
    id: UID,
    board_id: ID,
}

public struct ThreadCreated has copy, drop {
    board_id: ID,
    thread_id: ID,
}

public fun create_thread_intent(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    shard: &mut NonceGateShard,
    board: &mut Board,
) {
    let mut intent = intent::decode(
        intent_bytes,
        b"forum",
        b"create_thread_intent",
        signature,
        vector[object::id(shard), object::id(board)],
    );
    shard.advance(ctx, intent.sender(), intent.nonce());

    let content_hash = intent.bcs().peel_vec_u8();
    intent.end();

    create_thread(ctx, board, content_hash);
}

fun create_thread(ctx: &mut TxContext, board: &mut Board, content_hash: vector<u8>) {
    let thread = Thread {
        id: object::new(ctx),
        board_id: object::id(board),
    };
    event::emit(ThreadCreated {
        board_id: object::id(board),
        thread_id: object::id(&thread),
    });
    create_post(ctx, board, &thread, content_hash);
    transfer::public_share_object(thread);
}

// ----- post

public struct Post has key, store {
    id: UID,
    board_id: ID,
    thread_id: ID,
    number: u64,
    content_hash: vector<u8>,
}

public struct PostCreated has copy, drop {
    board_id: ID,
    thread_id: ID,
    post_id: ID,
}

public fun create_post_intent(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    shard: &mut NonceGateShard,
    board: &mut Board,
    thread: &Thread,
) {
    let mut intent = intent::decode(
        intent_bytes,
        b"forum",
        b"create_post_intent",
        signature,
        vector[object::id(shard), object::id(board), object::id(thread)],
    );
    shard.advance(ctx, intent.sender(), intent.nonce());

    let content_hash = intent.bcs().peel_vec_u8();
    intent.end();

    create_post(ctx, board, thread, content_hash);
}

fun create_post(ctx: &mut TxContext, board: &mut Board, thread: &Thread, content_hash: vector<u8>) {
    assert!(thread.board_id == object::id(board), ForumError::ThreadBoardMismatch.code());

    let number = board.next_post_number;
    board.next_post_number = number + 1;

    let post = Post {
        id: object::new(ctx),
        board_id: object::id(board),
        thread_id: object::id(thread),
        number,
        content_hash,
    };
    event::emit(PostCreated {
        board_id: object::id(board),
        thread_id: object::id(thread),
        post_id: object::id(&post),
    });
    transfer::public_share_object(post);
}

// ----- init

public struct BoardSlugRegistry has key, store {
    id: UID,
    slugs: Table<String, ID>,
}

fun init(ctx: &mut TxContext) {
    transfer::share_object(BoardSlugRegistry {
        id: object::new(ctx),
        slugs: table::new(ctx),
    });
}
