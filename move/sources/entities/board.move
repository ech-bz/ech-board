module forum::board;

use forum::bans::{Self, BanKey, BanValue, Bans};
use forum::empty::Empty;
use forum::entity::{Self, Entity};
use forum::event;
use forum::feed::{Self, Feed};
use forum::responses::Responses;
use forum::sender::{Self, Sender};
use std::ascii::{Self, String};
use sui::bcs;
use sui::dynamic_field;
use sui::table::{Self, Table};
use sui::vec_set::{Self, VecSet};

const VERSION: u16 = 2;

public struct Board has key {
    id: UID,
    entity: Entity,
    genesis: bool,
}

public use fun forum::board_apply::apply as Board.apply;
public use fun forum::board_apply::apply_thread as Board.apply_thread;
public use fun forum::board_apply::apply_post as Board.apply_post;

public(package) fun push(self: &mut Board, event: vector<u8>) {
    self.entity.feed_mut().push(event).share();
}

public(package) fun id(self: &Board): address {
    self.id.to_address()
}

public(package) fun genesis(self: &Board): bool {
    self.genesis
}

public(package) fun share(mut self: Board) {
    self.genesis = false;
    transfer::share_object(self)
}

public(package) fun check_version(self: &Board): bool {
    self.entity.version() == VERSION
}

const DF_SLUG: vector<u8> = b"slug";
const DF_DESC_HASH: vector<u8> = b"description_hash";
const DF_MAX_MEDIA: vector<u8> = b"max_media";
const DF_BUMP_LIMIT: vector<u8> = b"bump_limit";
const DF_CLOSED: vector<u8> = b"closed";
const DF_DELETED: vector<u8> = b"deleted";
const DF_PINNED: vector<u8> = b"pinned";
const DF_IGNORE_BANS: vector<u8> = b"ignore_forum_bans";
const DF_MODS: vector<u8> = b"moderators";
const DF_BANS: vector<u8> = b"bans";
const DF_REACTIONS: vector<u8> = b"reactions";
const DF_THREADS: vector<u8> = b"threads";
const DF_POSTS: vector<u8> = b"posts";
const DF_BUMPS: vector<u8> = b"bumps";

public(package) fun slug(self: &Board): &String {
    dynamic_field::borrow(&self.id, DF_SLUG)
}

public(package) fun slug_mut(self: &mut Board): &mut String {
    dynamic_field::borrow_mut(&mut self.id, DF_SLUG)
}

public(package) fun desc_hash(self: &Board): &Option<u256> {
    dynamic_field::borrow(&self.id, DF_DESC_HASH)
}

public(package) fun desc_hash_mut(self: &mut Board): &mut Option<u256> {
    dynamic_field::borrow_mut(&mut self.id, DF_DESC_HASH)
}

public(package) fun max_media(self: &Board): &u64 {
    dynamic_field::borrow(&self.id, DF_MAX_MEDIA)
}

public(package) fun max_media_mut(self: &mut Board): &mut u64 {
    dynamic_field::borrow_mut(&mut self.id, DF_MAX_MEDIA)
}

public(package) fun bump_limit(self: &Board): &u64 {
    dynamic_field::borrow(&self.id, DF_BUMP_LIMIT)
}

public(package) fun bump_limit_mut(self: &mut Board): &mut u64 {
    dynamic_field::borrow_mut(&mut self.id, DF_BUMP_LIMIT)
}

public(package) fun closed(self: &Board): &bool {
    dynamic_field::borrow(&self.id, DF_CLOSED)
}

public(package) fun closed_mut(self: &mut Board): &mut bool {
    dynamic_field::borrow_mut(&mut self.id, DF_CLOSED)
}

public(package) fun deleted(self: &Board): &bool {
    dynamic_field::borrow(&self.id, DF_DELETED)
}

public(package) fun deleted_mut(self: &mut Board): &mut bool {
    dynamic_field::borrow_mut(&mut self.id, DF_DELETED)
}

public(package) fun pinned(self: &Board): &vector<address> {
    dynamic_field::borrow(&self.id, DF_PINNED)
}

public(package) fun pinned_mut(self: &mut Board): &mut vector<address> {
    dynamic_field::borrow_mut(&mut self.id, DF_PINNED)
}

public(package) fun ignore_forum_bans(self: &Board): &bool {
    dynamic_field::borrow(&self.id, DF_IGNORE_BANS)
}

public(package) fun ignore_forum_bans_mut(self: &mut Board): &mut bool {
    dynamic_field::borrow_mut(&mut self.id, DF_IGNORE_BANS)
}

public(package) fun mods(self: &Board): &Table<address, Empty> {
    dynamic_field::borrow(&self.id, DF_MODS)
}

public(package) fun mods_mut(self: &mut Board): &mut Table<address, Empty> {
    dynamic_field::borrow_mut(&mut self.id, DF_MODS)
}

public(package) fun bans(self: &Board): &Bans {
    dynamic_field::borrow(&self.id, DF_BANS)
}

public(package) fun bans_mut(self: &mut Board): &mut Bans {
    dynamic_field::borrow_mut(&mut self.id, DF_BANS)
}

public(package) fun reactions(self: &Board): &VecSet<u256> {
    dynamic_field::borrow(&self.id, DF_REACTIONS)
}

public(package) fun reactions_mut(self: &mut Board): &mut VecSet<u256> {
    dynamic_field::borrow_mut(&mut self.id, DF_REACTIONS)
}

public(package) fun threads(self: &Board): &Table<u64, address> {
    dynamic_field::borrow(&self.id, DF_THREADS)
}

public(package) fun threads_mut(self: &mut Board): &mut Table<u64, address> {
    dynamic_field::borrow_mut(&mut self.id, DF_THREADS)
}

public(package) fun posts(self: &Board): &Table<u64, address> {
    dynamic_field::borrow(&self.id, DF_POSTS)
}

public(package) fun posts_mut(self: &mut Board): &mut Table<u64, address> {
    dynamic_field::borrow_mut(&mut self.id, DF_POSTS)
}

public(package) fun bumps(self: &Board): &Feed<address> {
    dynamic_field::borrow(&self.id, DF_BUMPS)
}

public(package) fun bumps_mut(self: &mut Board): &mut Feed<address> {
    dynamic_field::borrow_mut(&mut self.id, DF_BUMPS)
}

fun empty(ctx: &mut TxContext): Board {
    let entity = entity::new(ctx, 0);
    let mut self = Board { id: object::new(ctx), entity, genesis: true };
    self.do_upgrade(ctx);
    self
}

public(package) fun do_upgrade(self: &mut Board, ctx: &mut TxContext) {
    if (self.entity.version() < 1) self.init_v1(ctx);
    if (self.entity.version() < 2) self.init_v2(ctx);
}

fun init_v1(self: &mut Board, ctx: &mut TxContext) {
    self.entity.set_version(1);
    let id = self.id();
    dynamic_field::add(&mut self.id, DF_SLUG, ascii::string(b""));
    dynamic_field::add(&mut self.id, DF_DESC_HASH, option::none<u256>());
    dynamic_field::add(&mut self.id, DF_MAX_MEDIA, 0u64);
    dynamic_field::add(&mut self.id, DF_BUMP_LIMIT, 0u64);
    dynamic_field::add(&mut self.id, DF_CLOSED, false);
    dynamic_field::add(&mut self.id, DF_DELETED, false);
    dynamic_field::add(&mut self.id, DF_IGNORE_BANS, false);
    dynamic_field::add(&mut self.id, DF_MODS, table::new<address, Empty>(ctx));
    dynamic_field::add(&mut self.id, DF_BANS, bans::new(ctx, id));
    dynamic_field::add(&mut self.id, DF_REACTIONS, vec_set::empty<u256>());
    dynamic_field::add(&mut self.id, DF_THREADS, table::new<u64, address>(ctx));
    dynamic_field::add(&mut self.id, DF_POSTS, table::new<u64, address>(ctx));
    dynamic_field::add(&mut self.id, DF_BUMPS, feed::new<address>(ctx));
}

fun init_v2(self: &mut Board, _ctx: &mut TxContext) {
    self.entity.set_version(2);
    dynamic_field::add(&mut self.id, DF_PINNED, vector<address>[]);
}

public(package) fun new(
    ctx: &mut TxContext,
    responses: Responses,
    sender: Sender,
    slug: String,
): Board {
    let mut self = empty(ctx);
    let mut event = event::new("genesis", responses, sender);

    event = event.with(&slug);
    *self.slug_mut() = slug;

    self.push(event.build());
    self
}

public(package) fun add_moderator(
    responses: Responses,
    sender: Sender,
    moderator: address,
): vector<u8> {
    event::new("add_moderator", responses, sender).with(&moderator).build()
}

public(package) fun del_moderator(
    responses: Responses,
    sender: Sender,
    moderator: address,
): vector<u8> {
    event::new("del_moderator", responses, sender).with(&moderator).build()
}

public(package) fun set_max_media(
    responses: Responses,
    sender: Sender,
    max_media: u64,
): vector<u8> {
    event::new("set_max_media", responses, sender).with(&max_media).build()
}

public(package) fun set_bump_limit(
    responses: Responses,
    sender: Sender,
    bump_limit: u64,
): vector<u8> {
    event::new("set_bump_limit", responses, sender).with(&bump_limit).build()
}

public(package) fun set_closed(responses: Responses, sender: Sender, closed: bool): vector<u8> {
    event::new("set_closed", responses, sender).with(&closed).build()
}

public(package) fun set_deleted(responses: Responses, sender: Sender, deleted: bool): vector<u8> {
    event::new("set_deleted", responses, sender).with(&deleted).build()
}

public(package) fun set_pinned(
    responses: Responses,
    sender: Sender,
    pinned: vector<address>,
): vector<u8> {
    event::new("set_pinned", responses, sender).with(&pinned).build()
}

public(package) fun new_thread(
    responses: Responses,
    sender: Sender,
    topic_hash: Option<u256>,
    text_hash: Option<u256>,
    media_hashes: vector<u256>,
    vote_keys: vector<u256>,
    name_hash: Option<u256>,
): vector<u8> {
    event::new("new_thread", responses, sender)
        .with(&topic_hash)
        .with(&text_hash)
        .with(&media_hashes)
        .with(&vote_keys)
        .with(&name_hash)
        .build()
}

public(package) fun new_thread_v2(
    responses: Responses,
    sender: Sender,
    topic_hash: Option<u256>,
    text_hash: Option<u256>,
    media_hashes: vector<u256>,
    name_hash: Option<u256>,
    vote_keys: vector<u256>,
    multi_vote: bool,
): vector<u8> {
    event::new("new_thread_v2", responses, sender)
        .with(&topic_hash)
        .with(&text_hash)
        .with(&media_hashes)
        .with(&name_hash)
        .with(&vote_keys)
        .with(&multi_vote)
        .build()
}

public(package) fun set_description(
    responses: Responses,
    sender: Sender,
    desc_hash: Option<u256>,
): vector<u8> {
    event::new("set_description", responses, sender).with(&desc_hash).build()
}

public(package) fun set_ignore_forum_bans(
    responses: Responses,
    sender: Sender,
    ignore: bool,
): vector<u8> {
    event::new("set_ignore_forum_bans", responses, sender).with(&ignore).build()
}

public(package) fun set_reactions(
    responses: Responses,
    sender: Sender,
    reactions: vector<u256>,
): vector<u8> {
    event::new("set_reactions", responses, sender).with(&reactions).build()
}

public(package) fun new_post(
    responses: Responses,
    sender: Sender,
    thread: address,
    text_hash: Option<u256>,
    media_hashes: vector<u256>,
    vote_keys: vector<u256>,
    name_hash: Option<u256>,
): vector<u8> {
    event::new("new_post", responses, sender)
        .with(&thread)
        .with(&text_hash)
        .with(&media_hashes)
        .with(&vote_keys)
        .with(&name_hash)
        .build()
}

public(package) fun new_post_v2(
    responses: Responses,
    sender: Sender,
    thread: address,
    text_hash: Option<u256>,
    media_hashes: vector<u256>,
    name_hash: Option<u256>,
    vote_keys: vector<u256>,
    multi_vote: bool,
): vector<u8> {
    event::new("new_post_v2", responses, sender)
        .with(&thread)
        .with(&text_hash)
        .with(&media_hashes)
        .with(&name_hash)
        .with(&vote_keys)
        .with(&multi_vote)
        .build()
}

public(package) fun new_post_migrate(
    responses: Responses,
    sender: Sender,
    timestamp_ms: u64,
    thread: address,
    text_hash: Option<u256>,
    media_hashes: vector<u256>,
    vote_keys: vector<u256>,
    name_hash: Option<u256>,
): vector<u8> {
    event::new("new_post_migrate", responses, sender)
        .with(&timestamp_ms)
        .with(&thread)
        .with(&text_hash)
        .with(&media_hashes)
        .with(&vote_keys)
        .with(&name_hash)
        .build()
}

public(package) fun new_post_migrate_v2(
    responses: Responses,
    sender: Sender,
    timestamp_ms: u64,
    thread: address,
    text_hash: Option<u256>,
    media_hashes: vector<u256>,
    name_hash: Option<u256>,
    vote_keys: vector<u256>,
    multi_vote: bool,
): vector<u8> {
    event::new("new_post_migrate_v2", responses, sender)
        .with(&timestamp_ms)
        .with(&thread)
        .with(&text_hash)
        .with(&media_hashes)
        .with(&name_hash)
        .with(&vote_keys)
        .with(&multi_vote)
        .build()
}

public(package) fun ban(
    responses: Responses,
    sender: Sender,
    key: BanKey,
    value: BanValue,
): vector<u8> {
    event::new("ban", responses, sender).with(&key).with(&value).build()
}

public(package) fun unban(responses: Responses, sender: Sender, key: BanKey): vector<u8> {
    event::new("unban", responses, sender).with(&key).build()
}

public(package) fun upgrade(responses: Responses, sender: Sender): vector<u8> {
    event::new("upgrade", responses, sender).build()
}
