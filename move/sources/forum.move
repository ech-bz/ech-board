module forum::forum;

use forum::intent::{Self, IntentGateShard};
use std::string::{Self, String};
use sui::bcs;
use sui::event;

// ----- errors

public enum ForumError has copy, drop, store {
    ThreadBoardMismatch,
}

public fun code(self: ForumError): u64 {
    match (self) {
        ForumError::ThreadBoardMismatch => 1,
    }
}

// ----- board

public struct Board has key, store {
    id: UID,
    slug: String,
    next_post_number: u64,
}

public struct BoardCreated has copy, drop {
    board_id: address,
}

public fun create_board_intent(
    ctx: &mut TxContext,
    shard: &mut IntentGateShard,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
) {
    let intent = intent::decode(intent_bytes);
    intent.verify(
        ctx,
        shard,
        signature,
        b"forum",
        b"create_board_intent",
        vector[],
    );

    let mut decoded = bcs::new(intent.payload());
    let slug = string::utf8(decoded.peel_vec_u8());
    intent::assert_payload_consumed(decoded);

    create_board(ctx, slug);
}

fun create_board(ctx: &mut TxContext, slug: String) {
    let board = Board {
        id: object::new(ctx),
        slug,
        next_post_number: 1,
    };
    let board_id = object::id(&board).to_address();
    transfer::public_share_object(board);
    event::emit(BoardCreated { board_id });
}

// ----- thread

public struct Thread has key, store {
    id: UID,
    board_id: ID,
}

public struct ThreadCreated has copy, drop {
    board_id: address,
    thread_id: address,
}

public fun create_thread_intent(
    ctx: &mut TxContext,
    shard: &mut IntentGateShard,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    board: &mut Board,
) {
    let intent = intent::decode(intent_bytes);
    intent.verify(
        ctx,
        shard,
        signature,
        b"forum",
        b"create_thread_intent",
        vector[object::id(board)],
    );

    let mut decoded = bcs::new(intent.payload());
    let content_hash = decoded.peel_vec_u8();
    intent::assert_payload_consumed(decoded);

    create_thread(ctx, board, content_hash);
}

fun create_thread(ctx: &mut TxContext, board: &mut Board, content_hash: vector<u8>) {
    let thread = Thread {
        id: object::new(ctx),
        board_id: object::id(board),
    };
    create_post(ctx, board, &thread, content_hash);
    let thread_id = object::id(&thread).to_address();
    transfer::public_share_object(thread);
    event::emit(ThreadCreated {
        board_id: object::id(board).to_address(),
        thread_id,
    });
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
    board_id: address,
    thread_id: address,
    post_id: address,
}

public fun create_post_intent(
    ctx: &mut TxContext,
    shard: &mut IntentGateShard,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    board: &mut Board,
    thread: &Thread,
) {
    let intent = intent::decode(intent_bytes);
    intent.verify(
        ctx,
        shard,
        signature,
        b"forum",
        b"create_post_intent",
        vector[object::id(board), object::id(thread)],
    );

    let mut decoded = bcs::new(intent.payload());
    let content_hash = decoded.peel_vec_u8();
    intent::assert_payload_consumed(decoded);

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
    let post_id = object::id(&post).to_address();
    transfer::public_share_object(post);
    event::emit(PostCreated {
        board_id: object::id(board).to_address(),
        thread_id: object::id(thread).to_address(),
        post_id,
    });
}
