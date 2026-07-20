module forum::forum;

use forum::feed::{Self, Feed};
use forum::intent;
use forum::sharded_counter::{Self, ShardedCounter, Shard};
use std::ascii::{Self, String};
use sui::clock::{Self, Clock};
use sui::table::{Self, Table};

public enum ForumError has copy, drop, store {
    BoardSlugInvalid,
    MediaLimitExceeded,
    PostRequiresMedia,
    PostEmpty,
    BoardClosed,
    ThreadClosed,
    NotAuthorized,
    CrossReferenceMismatch,
}

public fun code(self: ForumError): u64 {
    match (self) {
        ForumError::BoardSlugInvalid => 1,
        ForumError::MediaLimitExceeded => 2,
        ForumError::PostRequiresMedia => 3,
        ForumError::PostEmpty => 4,
        ForumError::BoardClosed => 5,
        ForumError::ThreadClosed => 6,
        ForumError::NotAuthorized => 7,
        ForumError::CrossReferenceMismatch => 8,
    }
}

fun init(ctx: &mut TxContext) {
    let forum = ForumProjection {
        nonce_shards: sharded_counter::new(ctx, 512),
        admin: ctx.sender(),
        mods: table::new(ctx),
        bans: table::new(ctx),
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
    BanUser {
        addr: address,
        duration_ms: u64,
    },
    UnbanUser(address),
}

public struct ForumProjection has store {
    nonce_shards: ShardedCounter<address>,
    admin: address,
    mods: Table<address, Empty>,
    bans: Table<address, u64>,
    boards: Table<String, address>,
}

fun apply_forum(
    self: &mut ForumObject<ForumEvent, ForumProjection>,
    ctx: &mut TxContext,
    clock: &Clock,
    event: ForumEvent,
    sender: address,
) {
    self.feed.push(event).share();
    match (event) {
        ForumEvent::AddModerator(addr) => {
            assert!(sender == self.projection.admin, ForumError::NotAuthorized.code());
            self.projection.mods.add(addr, Empty());
        },
        ForumEvent::DelModerator(addr) => {
            assert!(sender == self.projection.admin, ForumError::NotAuthorized.code());
            self.projection.mods.remove(addr);
        },
        ForumEvent::NewBoard { slug, max_media, bump_limit } => {
            assert!(
                sender == self.projection.admin
                    || self.projection.mods.contains(sender),
                ForumError::NotAuthorized.code(),
            );
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
                bans: table::new(ctx),
                threads: table::new(ctx),
                posts: table::new(ctx),
                bumps: feed::new(ctx),
            };
            let board = new_forum_object<BoardEvent, _>(ctx, board);
            self.projection.boards.add(slug, object::uid_to_address(&board.id));
            transfer::share_object(board);
        },
        ForumEvent::BanUser { addr, duration_ms } => {
            assert!(
                sender == self.projection.admin
                    || self.projection.mods.contains(sender),
                ForumError::NotAuthorized.code(),
            );
            self.projection.bans.add(addr, clock.timestamp_ms() + duration_ms);
        },
        ForumEvent::UnbanUser(addr) => {
            assert!(
                sender == self.projection.admin
                    || self.projection.mods.contains(sender),
                ForumError::NotAuthorized.code(),
            );
            self.projection.bans.remove(addr);
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
    },
    BanUser {
        addr: address,
        duration_ms: u64,
    },
    UnbanUser(address),
}

public struct BoardProjection has store {
    slug: String,
    max_media: u64,
    bump_limit: u64,
    closed: bool,
    deleted: bool,
    mods: Table<address, Empty>,
    bans: Table<address, u64>,
    threads: Table<u64, address>,
    posts: Table<u64, address>,
    bumps: Feed<address>,
}

fun apply_board(
    self: &mut ForumObject<BoardEvent, BoardProjection>,
    ctx: &mut TxContext,
    clock: &Clock,
    forum: &ForumObject<ForumEvent, ForumProjection>,
    event: BoardEvent,
    sender: address,
    tweak: address,
) {
    self.feed.push(event).share();
    match (event) {
        BoardEvent::AddModerator(addr) => {
            assert!(
                sender == forum.projection.admin
                    || forum.projection.mods.contains(sender),
                ForumError::NotAuthorized.code(),
            );
            self.projection.mods.add(addr, Empty());
        },
        BoardEvent::DelModerator(addr) => {
            assert!(
                sender == forum.projection.admin
                    || forum.projection.mods.contains(sender),
                ForumError::NotAuthorized.code(),
            );
            self.projection.mods.remove(addr);
        },
        BoardEvent::SetMaxMedia(max_media) => {
            assert!(
                sender == forum.projection.admin
                    || forum.projection.mods.contains(sender),
                ForumError::NotAuthorized.code(),
            );
            self.projection.max_media = max_media;
        },
        BoardEvent::SetBumpLimit(bump_limit) => {
            assert!(
                sender == forum.projection.admin
                    || forum.projection.mods.contains(sender),
                ForumError::NotAuthorized.code(),
            );
            self.projection.bump_limit = bump_limit;
        },
        BoardEvent::SetClosed(closed) => {
            assert!(
                sender == forum.projection.admin
                    || forum.projection.mods.contains(sender),
                ForumError::NotAuthorized.code(),
            );
            assert!(self.projection.closed != closed);
            assert!(!self.projection.deleted);
            self.projection.closed = closed;
        },
        BoardEvent::SetDeleted(deleted) => {
            assert!(
                sender == forum.projection.admin
                    || forum.projection.mods.contains(sender),
                ForumError::NotAuthorized.code(),
            );
            assert!(self.projection.deleted != deleted);
            assert!(self.projection.closed);
            self.projection.deleted = deleted;
        },
        BoardEvent::NewThread { text_hash, media_hashes } => {
            assert!(
                self.projection.max_media == 0 || media_hashes.length() > 0,
                ForumError::PostRequiresMedia.code(),
            );
            let number = self.projection.posts.length() + 1;
            let thread = ThreadProjection {
                board_slug: self.projection.slug,
                number,
                op: @0x0,
                closed: false,
                deleted: false,
                pinned: false,
                admin: option::none(),
                mods: table::new(ctx),
                bans: table::new(ctx),
                posts: table::new(ctx),
                last_3: vector[],
            };
            let mut thread = new_forum_object(ctx, thread);
            self.projection.threads.add(number, object::uid_to_address(&thread.id));
            thread.apply_thread_board(
                self,
                ctx,
                ThreadEvent::NewPost { text_hash, media_hashes },
                sender,
                tweak,
            );
            transfer::share_object(thread);
        },
        BoardEvent::BanUser { addr, duration_ms } => {
            assert!(
                sender == forum.projection.admin
                    || forum.projection.mods.contains(sender)
                    || self.projection.mods.contains(sender),
                ForumError::NotAuthorized.code(),
            );
            self.projection.bans.add(addr, clock.timestamp_ms() + duration_ms);
        },
        BoardEvent::UnbanUser(addr) => {
            assert!(
                sender == forum.projection.admin
                    || forum.projection.mods.contains(sender)
                    || self.projection.mods.contains(sender),
                ForumError::NotAuthorized.code(),
            );
            self.projection.bans.remove(addr);
        },
        _ => abort,
    }
}

fun apply_thread_board(
    self: &mut ForumObject<ThreadEvent, ThreadProjection>,
    board: &mut ForumObject<BoardEvent, BoardProjection>,
    ctx: &mut TxContext,
    event: ThreadEvent,
    sender: address,
    tweak: address,
) {
    self.feed.push(event).share();
    match (event) {
        ThreadEvent::NewPost { text_hash, media_hashes } => {
            assert!(
                self.projection.board_slug == board.projection.slug,
                ForumError::CrossReferenceMismatch.code(),
            );
            assert!(
                media_hashes.length() <= board.projection.max_media,
                ForumError::MediaLimitExceeded.code(),
            );
            assert!(media_hashes.length() > 0 || text_hash.is_some(), ForumError::PostEmpty.code());
            assert!(!board.projection.closed, ForumError::BoardClosed.code());
            assert!(!self.projection.closed, ForumError::ThreadClosed.code());
            let number = board.projection.posts.length() + 1;
            let post = PostProjection {
                board_slug: board.projection.slug,
                thread: self.projection.number,
                number,
                author: sender,
                tweak,
                deleted: false,
                text_hash,
                media_hashes,
            };
            let post = new_forum_object<PostEvent, _>(ctx, post);
            let post_id = object::uid_to_address(&post.id);
            board.projection.posts.add(number, post_id);
            self.projection.posts.add(number, post_id);
            if (number == self.projection.number) {
                self.projection.op = post_id;
            } else {
                self.projection.last_3.push_back(post_id);
                if (self.projection.last_3.length() > 3) {
                    self.projection.last_3.remove(0);
                };
            };
            board.feed.push(BoardEvent::NewPost { thread: self.projection.number }).share();
            if (self.projection.posts.length() <= board.projection.bump_limit) {
                board.projection.bumps.push(object::uid_to_address(&self.id)).share();
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
    NewPost {
        text_hash: Option<address>,
        media_hashes: vector<address>,
    },
    SetAdmin(Option<address>),
    BanUser {
        addr: address,
        duration_ms: u64,
    },
    UnbanUser(address),
}

public struct ThreadProjection has store {
    board_slug: String,
    number: u64,
    op: address,
    closed: bool,
    deleted: bool,
    pinned: bool,
    admin: Option<address>,
    mods: Table<address, Empty>,
    bans: Table<address, u64>,
    posts: Table<u64, address>,
    last_3: vector<address>,
}

fun apply_thread(
    self: &mut ForumObject<ThreadEvent, ThreadProjection>,
    clock: &Clock,
    forum: &ForumObject<ForumEvent, ForumProjection>,
    board: &ForumObject<BoardEvent, BoardProjection>,
    event: ThreadEvent,
    sender: address,
) {
    assert!(
        self.projection.board_slug == board.projection.slug,
        ForumError::CrossReferenceMismatch.code(),
    );
    self.feed.push(event).share();
    match (event) {
        ThreadEvent::AddModerator(addr) => {
            assert!(
                sender == forum.projection.admin
                    || forum.projection.mods.contains(sender)
                    || board.projection.mods.contains(sender)
                    || self.projection.admin.is_some_and!(|a| sender == a),
                ForumError::NotAuthorized.code(),
            );
            self.projection.mods.add(addr, Empty());
        },
        ThreadEvent::DelModerator(addr) => {
            assert!(
                sender == forum.projection.admin
                    || forum.projection.mods.contains(sender)
                    || board.projection.mods.contains(sender)
                    || self.projection.admin.is_some_and!(|a| sender == a),
                ForumError::NotAuthorized.code(),
            );
            self.projection.mods.remove(addr);
        },
        ThreadEvent::SetClosed(closed) => {
            assert!(
                sender == forum.projection.admin
                    || forum.projection.mods.contains(sender)
                    || board.projection.mods.contains(sender)
                    || self.projection.admin.is_some_and!(|a| sender == a)
                    || self.projection.mods.contains(sender),
                ForumError::NotAuthorized.code(),
            );
            assert!(self.projection.closed != closed);
            assert!(!self.projection.deleted);
            self.projection.closed = closed;
        },
        ThreadEvent::SetDeleted(deleted) => {
            assert!(
                sender == forum.projection.admin
                    || forum.projection.mods.contains(sender)
                    || board.projection.mods.contains(sender)
                    || self.projection.admin.is_some_and!(|a| sender == a)
                    || self.projection.mods.contains(sender),
                ForumError::NotAuthorized.code(),
            );
            assert!(self.projection.deleted != deleted);
            assert!(self.projection.closed);
            self.projection.deleted = deleted;
        },
        ThreadEvent::SetPinned(pinned) => {
            assert!(
                sender == forum.projection.admin
                    || board.projection.mods.contains(sender),
                ForumError::NotAuthorized.code(),
            );
            self.projection.pinned = pinned;
        },
        ThreadEvent::SetAdmin(thread_admin) => {
            assert!(
                sender == forum.projection.admin
                    || forum.projection.mods.contains(sender)
                    || board.projection.mods.contains(sender),
                ForumError::NotAuthorized.code(),
            );
            self.projection.admin = thread_admin;
        },
        ThreadEvent::BanUser { addr, duration_ms } => {
            assert!(
                sender == forum.projection.admin
                    || forum.projection.mods.contains(sender)
                    || board.projection.mods.contains(sender)
                    || self.projection.admin.is_some_and!(|a| sender == a)
                    || self.projection.mods.contains(sender),
                ForumError::NotAuthorized.code(),
            );
            self.projection.bans.add(addr, clock.timestamp_ms() + duration_ms);
        },
        ThreadEvent::UnbanUser(addr) => {
            assert!(
                sender == forum.projection.admin
                    || forum.projection.mods.contains(sender)
                    || board.projection.mods.contains(sender)
                    || self.projection.admin.is_some_and!(|a| sender == a)
                    || self.projection.mods.contains(sender),
                ForumError::NotAuthorized.code(),
            );
            self.projection.bans.remove(addr);
        },
        _ => abort,
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

fun apply_post(
    self: &mut ForumObject<PostEvent, PostProjection>,
    forum: &ForumObject<ForumEvent, ForumProjection>,
    board: &ForumObject<BoardEvent, BoardProjection>,
    thread: &ForumObject<ThreadEvent, ThreadProjection>,
    event: PostEvent,
    sender: address,
) {
    assert!(
        self.projection.board_slug == board.projection.slug,
        ForumError::CrossReferenceMismatch.code(),
    );
    assert!(
        self.projection.thread == thread.projection.number,
        ForumError::CrossReferenceMismatch.code(),
    );
    self.feed.push(event).share();
    match (event) {
        PostEvent::SetDeleted(deleted) => {
            assert!(self.projection.deleted != deleted);
            assert!(
                sender == self.projection.author
                    || sender == forum.projection.admin
                    || forum.projection.mods.contains(sender)
                    || board.projection.mods.contains(sender)
                    || thread.projection.admin.is_some_and!(|a| sender == a)
                    || thread.projection.mods.contains(sender),
                ForumError::NotAuthorized.code(),
            );
            self.projection.deleted = deleted;
        },
        PostEvent::ChangeText(hash) => {
            assert!(
                sender == self.projection.author
                    || sender == forum.projection.admin
                    || forum.projection.mods.contains(sender)
                    || board.projection.mods.contains(sender)
                    || thread.projection.admin.is_some_and!(|a| sender == a)
                    || thread.projection.mods.contains(sender),
                ForumError::NotAuthorized.code(),
            );
            self.projection.text_hash = hash;
        },
        PostEvent::RemoveMedia(hashes) => {
            assert!(
                sender == self.projection.author
                    || sender == forum.projection.admin
                    || forum.projection.mods.contains(sender)
                    || board.projection.mods.contains(sender)
                    || thread.projection.admin.is_some_and!(|a| sender == a)
                    || thread.projection.mods.contains(sender),
                ForumError::NotAuthorized.code(),
            );
            self.projection.media_hashes =
                self.projection.media_hashes.filter!(|hash| !hashes.contains(hash));
            if (self.projection.media_hashes.is_empty() && self.projection.text_hash.is_none()) {
                self.apply_post(forum, board, thread, PostEvent::SetDeleted(true), sender);
            };
        },
    }
}

public fun apply_forum_intent(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &mut ForumObject<ForumEvent, ForumProjection>,
) {
    let mut intent = intent::decode(
        intent_bytes,
        b"forum",
        b"apply_forum_intent",
        signature,
        vector[object::id(clock), object::id(nonce_shard), object::id(forum)],
    );
    nonce_shard.inc_checked(&intent.sender(), intent.nonce());

    forum.apply_forum(
        ctx,
        clock,
        match (intent.bcs().peel_enum_tag()) {
            0 => ForumEvent::AddModerator(intent.bcs().peel_address()),
            1 => ForumEvent::DelModerator(intent.bcs().peel_address()),
            2 => ForumEvent::NewBoard {
                slug: ascii::string(intent.bcs().peel_vec_u8()),
                max_media: intent.bcs().peel_u64(),
                bump_limit: intent.bcs().peel_u64(),
            },
            3 => ForumEvent::BanUser {
                addr: intent.bcs().peel_address(),
                duration_ms: intent.bcs().peel_u64(),
            },
            4 => ForumEvent::UnbanUser(intent.bcs().peel_address()),
            _ => abort,
        },
        intent.sender(),
    );
    intent.end();
}

public fun apply_board_intent(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    clock: &Clock,
    nonce_shard: &mut Shard<address>,
    forum: &ForumObject<ForumEvent, ForumProjection>,
    board: &mut ForumObject<BoardEvent, BoardProjection>,
) {
    let mut intent = intent::decode(
        intent_bytes,
        b"forum",
        b"apply_board_intent",
        signature,
        vector[object::id(clock), object::id(nonce_shard), object::id(forum), object::id(board)],
    );
    nonce_shard.inc_checked(&intent.sender(), intent.nonce());

    board.apply_board(
        ctx,
        clock,
        forum,
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
            8 => BoardEvent::BanUser {
                addr: intent.bcs().peel_address(),
                duration_ms: intent.bcs().peel_u64(),
            },
            9 => BoardEvent::UnbanUser(intent.bcs().peel_address()),
            _ => abort,
        },
        intent.sender(),
        intent.tweak(),
    );
    intent.end();
}

public fun apply_thread_board_intent(
    ctx: &mut TxContext,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    nonce_shard: &mut Shard<address>,
    forum: &ForumObject<ForumEvent, ForumProjection>,
    board: &mut ForumObject<BoardEvent, BoardProjection>,
    thread: &mut ForumObject<ThreadEvent, ThreadProjection>,
) {
    let mut intent = intent::decode(
        intent_bytes,
        b"forum",
        b"apply_thread_board_intent",
        signature,
        vector[object::id(nonce_shard), object::id(forum), object::id(board), object::id(thread)],
    );
    nonce_shard.inc_checked(&intent.sender(), intent.nonce());

    thread.apply_thread_board(
        board,
        ctx,
        match (intent.bcs().peel_enum_tag()) {
            5 => ThreadEvent::NewPost {
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
    clock: &Clock,
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    nonce_shard: &mut Shard<address>,
    forum: &ForumObject<ForumEvent, ForumProjection>,
    board: &ForumObject<BoardEvent, BoardProjection>,
    thread: &mut ForumObject<ThreadEvent, ThreadProjection>,
) {
    let mut intent = intent::decode(
        intent_bytes,
        b"forum",
        b"apply_thread_intent",
        signature,
        vector[object::id(nonce_shard), object::id(forum), object::id(board), object::id(thread)],
    );
    nonce_shard.inc_checked(&intent.sender(), intent.nonce());

    thread.apply_thread(
        clock,
        forum,
        board,
        match (intent.bcs().peel_enum_tag()) {
            0 => ThreadEvent::AddModerator(intent.bcs().peel_address()),
            1 => ThreadEvent::DelModerator(intent.bcs().peel_address()),
            2 => ThreadEvent::SetClosed(intent.bcs().peel_bool()),
            3 => ThreadEvent::SetDeleted(intent.bcs().peel_bool()),
            4 => ThreadEvent::SetPinned(intent.bcs().peel_bool()),
            6 => ThreadEvent::SetAdmin(intent.bcs().peel_option!(|bcs| bcs.peel_address())),
            7 => ThreadEvent::BanUser {
                addr: intent.bcs().peel_address(),
                duration_ms: intent.bcs().peel_u64(),
            },
            8 => ThreadEvent::UnbanUser(intent.bcs().peel_address()),
            _ => abort,
        },
        intent.sender(),
    );
    intent.end();
}

public fun apply_post_intent(
    intent_bytes: vector<u8>,
    signature: vector<u8>,
    nonce_shard: &mut Shard<address>,
    forum: &ForumObject<ForumEvent, ForumProjection>,
    board: &ForumObject<BoardEvent, BoardProjection>,
    thread: &ForumObject<ThreadEvent, ThreadProjection>,
    post: &mut ForumObject<PostEvent, PostProjection>,
) {
    let mut intent = intent::decode(
        intent_bytes,
        b"forum",
        b"apply_post_intent",
        signature,
        vector[
            object::id(nonce_shard),
            object::id(forum),
            object::id(board),
            object::id(thread),
            object::id(post),
        ],
    );
    nonce_shard.inc_checked(&intent.sender(), intent.nonce());

    post.apply_post(
        forum,
        board,
        thread,
        match (intent.bcs().peel_enum_tag()) {
            0 => PostEvent::SetDeleted(intent.bcs().peel_bool()),
            1 => PostEvent::ChangeText(intent.bcs().peel_option!(|bcs| bcs.peel_address())),
            2 => PostEvent::RemoveMedia(intent.bcs().peel_vec!(|bcs| bcs.peel_address())),
            _ => abort,
        },
        intent.sender(),
    );
    intent.end();
}
