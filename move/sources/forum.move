module forum::forum;

use forum::feed::{Self, Feed};
use forum::intent;
use forum::sharded_counter::{Self, ShardedCounter, Shard};
use std::ascii::{Self, String};
use sui::derived_object;
use sui::table::{Self, Table};

public enum ForumError has copy, drop, store {
    BoardSlugInvalid,
    MediaLimitExceeded,
    PostRequiresMedia,
    PostEmpty,
    BoardClosed,
    ThreadClosed,
}

public fun code(self: ForumError): u64 {
    match (self) {
        ForumError::BoardSlugInvalid => 1,
        ForumError::MediaLimitExceeded => 2,
        ForumError::PostRequiresMedia => 3,
        ForumError::PostEmpty => 4,
        ForumError::BoardClosed => 5,
        ForumError::ThreadClosed => 6,
    }
}

fun init(ctx: &mut TxContext) {
    let forum = ForumProjection {
        nonce_shards: sharded_counter::new(ctx, 512),
        mods: table::new(ctx),
        boards: table::new(ctx),
    };
    transfer::share_object(new_forum_object<ForumEvent, ForumProjection>(ctx, forum));
}

public struct Empty has drop, store ()

public struct ForumObject<phantom E: store, P: store> has key, store {
    id: UID,
    feed: Feed<E>,
    projection: P,
}

fun new_forum_object<E: store, P: store>(ctx: &mut TxContext, projection: P): ForumObject<E, P> {
    ForumObject<E, P> {
        id: object::new(ctx),
        feed: feed::new(ctx),
        projection,
    }
}

public enum ForumEvent has copy, drop, store {
    AddModerator(address),
    DelModerator(address),
    NewBoard {
        slug: String,
        max_media: u64,
        bump_limit: u64,
    },
}

public struct ForumProjection has store {
    nonce_shards: ShardedCounter<address>,
    mods: Table<address, Empty>,
    boards: Table<String, address>,
}

fun apply_forum(
    self: &mut ForumObject<ForumEvent, ForumProjection>,
    ctx: &mut TxContext,
    event: ForumEvent,
) {
    let forum = &mut self.projection;
    self.feed.push(event).share();
    match (event) {
        ForumEvent::AddModerator(addr) => {
            forum.mods.add(addr, Empty());
        },
        ForumEvent::DelModerator(addr) => {
            forum.mods.remove(addr);
        },
        ForumEvent::NewBoard { slug, max_media, bump_limit } => {
            assert!(
                slug.as_bytes().all!(|c| (*c >= 0x30 && *c <= 0x39) || (*c >= 0x61 && *c <= 0x7a)),
                ForumError::BoardSlugInvalid.code(),
            );
            assert!(slug.length() >= 1 && slug.length() <= 16, ForumError::BoardSlugInvalid.code());
            let board = BoardProjection {
                slug,
                max_media,
                bump_limit,
                closed: false,
                deleted: false,
                mods: table::new(ctx),
                threads: table::new(ctx),
                posts: table::new(ctx),
                bumps: feed::new(ctx),
            };
            let board = new_forum_object<BoardEvent, _>(ctx, board);
            forum.boards.add(slug, object::uid_to_address(&board.id));
            transfer::share_object(board);
        },
    }
}

public enum BoardEvent has copy, drop, store {
    AddModerator(address),
    DelModerator(address),
    SetMaxMedia(u64),
    SetBumpLimit(u64),
    SetClosed(bool),
    SetDeleted(bool),
    NewThread {
        text_hash: Option<address>,
        media_hashes: vector<address>,
    },
    NewPost {
        thread: u64,
        text_hash: Option<address>,
        media_hashes: vector<address>,
    },
}

public struct BoardProjection has store {
    slug: String,
    max_media: u64,
    bump_limit: u64,
    closed: bool,
    deleted: bool,
    mods: Table<address, Empty>,
    threads: Table<u64, address>,
    posts: Table<u64, address>,
    bumps: Feed<address>,
}

fun apply_board(
    self: &mut ForumObject<BoardEvent, BoardProjection>,
    ctx: &mut TxContext,
    event: BoardEvent,
    sender: address,
    tweak: address,
) {
    let board = &mut self.projection;
    self.feed.push(event).share();
    match (event) {
        BoardEvent::AddModerator(addr) => {
            board.mods.add(addr, Empty());
        },
        BoardEvent::DelModerator(addr) => {
            board.mods.remove(addr);
        },
        BoardEvent::SetMaxMedia(max_media) => board.max_media = max_media,
        BoardEvent::SetBumpLimit(bump_limit) => board.bump_limit = bump_limit,
        BoardEvent::SetClosed(closed) => {
            assert!(board.closed != closed);
            assert!(!board.deleted);
            board.closed = closed;
        },
        BoardEvent::SetDeleted(deleted) => {
            assert!(board.deleted != deleted);
            assert!(board.closed);
            board.deleted = deleted;
        },
        BoardEvent::NewThread { text_hash, media_hashes } => {
            assert!(
                board.max_media == 0 || media_hashes.length() > 0,
                ForumError::PostRequiresMedia.code(),
            );
            let number = board.posts.length() + 1;
            let post = PostProjection {
                board_slug: board.slug,
                thread: number,
                number,
                author: sender,
                tweak,
                deleted: false,
                text_hash,
                media_hashes,
            };
            let post = new_forum_object<PostEvent, _>(
                ctx,
                post,
            );
            let thread = ThreadProjection {
                board_slug: board.slug,
                number,
                op: object::uid_to_address(&post.id),
                closed: false,
                deleted: false,
                pinned: false,
                mods: table::new(ctx),
                posts: table::new(ctx),
                last_3: vector[],
            };
            let mut thread = new_forum_object(ctx, thread);
            thread.apply_thread(ThreadEvent::NewPost {
                number,
                post_id: object::uid_to_address(&post.id),
            });
            board.threads.add(number, object::uid_to_address(&thread.id));
            board.posts.add(number, object::uid_to_address(&post.id));
            self
                .feed
                .push(BoardEvent::NewPost {
                    thread: number,
                    text_hash,
                    media_hashes,
                })
                .share();
            if (thread.projection.posts.length() <= board.bump_limit) {
                board.bumps.push(object::uid_to_address(&thread.id)).share();
            };
            transfer::share_object(thread);
            transfer::share_object(post);
        },
        _ => abort,
    }
}

fun apply_board_thread(
    self: &mut ForumObject<BoardEvent, BoardProjection>,
    thread: &mut ForumObject<ThreadEvent, ThreadProjection>,
    ctx: &mut TxContext,
    event: BoardEvent,
    sender: address,
    tweak: address,
) {
    let board = &mut self.projection;
    self.feed.push(event).share();
    match (event) {
        BoardEvent::NewPost { thread: thread_num, text_hash, media_hashes } => {
            assert!(
                media_hashes.length() <= board.max_media,
                ForumError::MediaLimitExceeded.code(),
            );
            assert!(media_hashes.length() > 0 || text_hash.is_some(), ForumError::PostEmpty.code());
            assert!(!board.closed, ForumError::BoardClosed.code());
            assert!(!thread.projection.closed, ForumError::ThreadClosed.code());
            let number = board.posts.length() + 1;
            let post = PostProjection {
                board_slug: board.slug,
                thread: thread_num,
                number,
                author: sender,
                tweak,
                deleted: false,
                text_hash,
                media_hashes,
            };
            let post = new_forum_object<PostEvent, _>(ctx, post);
            board.posts.add(number, object::uid_to_address(&post.id));
            thread.apply_thread(ThreadEvent::NewPost {
                number,
                post_id: object::uid_to_address(&post.id),
            });
            if (thread.projection.posts.length() <= board.bump_limit) {
                board.bumps.push(object::uid_to_address(&thread.id)).share();
            };
            transfer::share_object(post);
        },
        _ => abort,
    }
}

public enum ThreadEvent has copy, drop, store {
    AddModerator(address),
    DelModerator(address),
    SetClosed(bool),
    SetDeleted(bool),
    SetPinned(bool),
    NewPost { number: u64, post_id: address }, // private
}

public struct ThreadProjection has store {
    board_slug: String,
    number: u64,
    op: address,
    closed: bool,
    deleted: bool,
    pinned: bool,
    mods: Table<address, Empty>,
    posts: Table<u64, address>,
    last_3: vector<address>,
}

fun apply_thread(self: &mut ForumObject<ThreadEvent, ThreadProjection>, event: ThreadEvent) {
    let thread = &mut self.projection;
    self.feed.push(event).share();
    match (event) {
        ThreadEvent::AddModerator(addr) => {
            thread.mods.add(addr, Empty());
        },
        ThreadEvent::DelModerator(addr) => {
            thread.mods.remove(addr);
        },
        ThreadEvent::SetClosed(closed) => {
            assert!(thread.closed != closed);
            assert!(!thread.deleted);
            thread.closed = closed;
        },
        ThreadEvent::SetDeleted(deleted) => {
            assert!(thread.deleted != deleted);
            assert!(thread.closed);
            thread.deleted = deleted;
        },
        ThreadEvent::SetPinned(pinned) => thread.pinned = pinned,
        ThreadEvent::NewPost { number, post_id } => {
            thread.posts.add(number, post_id);
            if (number == thread.number) {
                thread.op = post_id;
            } else {
                thread.last_3.push_back(post_id);
                if (thread.last_3.length() > 3) {
                    thread.last_3.remove(0);
                };
            };
        },
    }
}

public enum PostEvent has copy, drop, store {
    SetDeleted(bool),
    ChangeText(Option<address>),
    RemoveMedia(vector<address>),
}

public struct PostProjection has store {
    board_slug: String,
    thread: u64,
    number: u64,
    author: address,
    tweak: address,
    deleted: bool,
    text_hash: Option<address>,
    media_hashes: vector<address>,
}

fun apply_post(self: &mut ForumObject<PostEvent, PostProjection>, event: PostEvent) {
    let post = &mut self.projection;
    self.feed.push(event).share();
    match (event) {
        PostEvent::SetDeleted(deleted) => post.deleted = deleted,
        PostEvent::ChangeText(hash) => post.text_hash = hash,
        PostEvent::RemoveMedia(hashes) => {
            let media_hashes = post.media_hashes.filter!(|hash| !hashes.contains(hash));
            assert!(media_hashes.length() + hashes.length() == post.media_hashes.length());
            post.media_hashes = media_hashes;
        },
    }
}

public fun apply_forum_intent(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    nonce_shard: &mut Shard<address>,
    forum: &mut ForumObject<ForumEvent, ForumProjection>,
) {
    let mut intent = intent::decode(
        intent_bytes,
        b"forum",
        b"apply_forum_intent",
        signature,
        vector[object::id(nonce_shard), object::id(forum)],
    );
    nonce_shard.inc_checked(&intent.sender(), intent.nonce());

    forum.apply_forum(
        ctx,
        match (intent.bcs().peel_enum_tag()) {
            0 => ForumEvent::AddModerator(intent.bcs().peel_address()),
            1 => ForumEvent::DelModerator(intent.bcs().peel_address()),
            2 => ForumEvent::NewBoard {
                slug: ascii::string(intent.bcs().peel_vec_u8()),
                max_media: intent.bcs().peel_u64(),
                bump_limit: intent.bcs().peel_u64(),
            },
            _ => abort,
        },
    );
    intent.end();
}

public fun apply_board_intent(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    nonce_shard: &mut Shard<address>,
    board: &mut ForumObject<BoardEvent, BoardProjection>,
) {
    let mut intent = intent::decode(
        intent_bytes,
        b"forum",
        b"apply_board_intent",
        signature,
        vector[object::id(nonce_shard), object::id(board)],
    );
    nonce_shard.inc_checked(&intent.sender(), intent.nonce());

    board.apply_board(
        ctx,
        match (intent.bcs().peel_enum_tag()) {
            0 => BoardEvent::AddModerator(intent.bcs().peel_address()),
            1 => BoardEvent::DelModerator(intent.bcs().peel_address()),
            2 => BoardEvent::SetMaxMedia(intent.bcs().peel_u64()),
            3 => BoardEvent::SetBumpLimit(intent.bcs().peel_u64()),
            4 => BoardEvent::SetClosed(intent.bcs().peel_bool()),
            5 => BoardEvent::SetDeleted(intent.bcs().peel_bool()),
            6 => BoardEvent::NewThread {
                text_hash: intent.bcs().peel_option!(|bcs| bcs.peel_address()),
                media_hashes: intent.bcs().peel_vec!(|bcs| bcs.peel_address()),
            },
            _ => abort,
        },
        intent.sender(),
        intent.tweak(),
    );
    intent.end();
}

public fun apply_board_thread_intent(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    nonce_shard: &mut Shard<address>,
    board: &mut ForumObject<BoardEvent, BoardProjection>,
    thread: &mut ForumObject<ThreadEvent, ThreadProjection>,
) {
    let mut intent = intent::decode(
        intent_bytes,
        b"forum",
        b"apply_board_thread_intent",
        signature,
        vector[object::id(nonce_shard), object::id(board), object::id(thread)],
    );
    nonce_shard.inc_checked(&intent.sender(), intent.nonce());

    board.apply_board_thread(
        thread,
        ctx,
        match (intent.bcs().peel_enum_tag()) {
            7 => BoardEvent::NewPost {
                thread: intent.bcs().peel_u64(),
                text_hash: intent.bcs().peel_option!(|bcs| bcs.peel_address()),
                media_hashes: intent.bcs().peel_vec!(|bcs| bcs.peel_address()),
            },
            _ => abort,
        },
        intent.sender(),
        intent.tweak(),
    );
    intent.end();
}

public fun apply_thread_intent(
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    nonce_shard: &mut Shard<address>,
    thread: &mut ForumObject<ThreadEvent, ThreadProjection>,
) {
    let mut intent = intent::decode(
        intent_bytes,
        b"forum",
        b"apply_thread_intent",
        signature,
        vector[object::id(nonce_shard), object::id(thread)],
    );
    nonce_shard.inc_checked(&intent.sender(), intent.nonce());

    thread.apply_thread(match (intent.bcs().peel_enum_tag()) {
        0 => ThreadEvent::AddModerator(intent.bcs().peel_address()),
        1 => ThreadEvent::DelModerator(intent.bcs().peel_address()),
        2 => ThreadEvent::SetClosed(intent.bcs().peel_bool()),
        3 => ThreadEvent::SetDeleted(intent.bcs().peel_bool()),
        4 => ThreadEvent::SetPinned(intent.bcs().peel_bool()),
        _ => abort,
    });
    intent.end();
}

public fun apply_post_intent(
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    nonce_shard: &mut Shard<address>,
    post: &mut ForumObject<PostEvent, PostProjection>,
) {
    let mut intent = intent::decode(
        intent_bytes,
        b"forum",
        b"apply_post_intent",
        signature,
        vector[object::id(nonce_shard), object::id(post)],
    );
    nonce_shard.inc_checked(&intent.sender(), intent.nonce());

    post.apply_post(match (intent.bcs().peel_enum_tag()) {
        0 => PostEvent::SetDeleted(intent.bcs().peel_bool()),
        1 => PostEvent::ChangeText(intent.bcs().peel_option!(|bcs| bcs.peel_address())),
        2 => PostEvent::RemoveMedia(intent.bcs().peel_vec!(|bcs| bcs.peel_address())),
        _ => abort,
    });
    intent.end();
}
