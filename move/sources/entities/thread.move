module forum::thread;

use forum::bans::{Self, BanKey, BanValue, Bans};
use forum::empty::{Self, Empty};
use forum::entity::{Self, Entity};
use forum::event;
use forum::feed::{Self, Feed};
use forum::responses::Responses;
use forum::sender::{Self, Sender};
use sui::bcs;
use sui::dynamic_field;
use sui::table::{Self, Table};

const VERSION: u16 = 3;

public struct Thread has key {
    id: UID,
    entity: Entity,
    genesis: bool,
}

public use fun forum::thread_apply::apply as Thread.apply;
public use fun forum::thread_apply::apply_post as Thread.apply_post;

public(package) fun push(self: &mut Thread, event: vector<u8>) {
    self.entity.feed_mut().push(event).share();
}

public(package) fun id(self: &Thread): address {
    self.id.to_address()
}

public(package) fun genesis(self: &Thread): bool {
    self.genesis
}

public(package) fun share(mut self: Thread) {
    self.genesis = false;
    transfer::share_object(self)
}

public(package) fun check_version(self: &Thread): bool {
    self.entity.version() == VERSION
}

const DF_BOARD: vector<u8> = b"board";
const DF_NUMBER: vector<u8> = b"number";
const DF_TOPIC_HASH: vector<u8> = b"topic_hash";
const DF_OP: vector<u8> = b"op";
const DF_CLOSED: vector<u8> = b"closed";
const DF_DELETED: vector<u8> = b"deleted";
const DF_PINNED: vector<u8> = b"pinned";
const DF_ADMIN: vector<u8> = b"admin";
const DF_MODS: vector<u8> = b"moderators";
const DF_BANS: vector<u8> = b"bans";
const DF_POSTS: vector<u8> = b"posts";
const DF_POSTS_DELETED: vector<u8> = b"posts_deleted";
const DF_LAST3: vector<u8> = b"last_posts";

public(package) fun board(self: &Thread): &address {
    dynamic_field::borrow(&self.id, DF_BOARD)
}

public(package) fun board_mut(self: &mut Thread): &mut address {
    dynamic_field::borrow_mut(&mut self.id, DF_BOARD)
}

public(package) fun number(self: &Thread): &u64 {
    dynamic_field::borrow(&self.id, DF_NUMBER)
}

public(package) fun number_mut(self: &mut Thread): &mut u64 {
    dynamic_field::borrow_mut(&mut self.id, DF_NUMBER)
}

public(package) fun topic_hash(self: &Thread): &Option<u256> {
    dynamic_field::borrow(&self.id, DF_TOPIC_HASH)
}

public(package) fun topic_hash_mut(self: &mut Thread): &mut Option<u256> {
    dynamic_field::borrow_mut(&mut self.id, DF_TOPIC_HASH)
}

public(package) fun op(self: &Thread): &address {
    dynamic_field::borrow(&self.id, DF_OP)
}

public(package) fun op_mut(self: &mut Thread): &mut address {
    dynamic_field::borrow_mut(&mut self.id, DF_OP)
}

public(package) fun closed(self: &Thread): &bool {
    dynamic_field::borrow(&self.id, DF_CLOSED)
}

public(package) fun closed_mut(self: &mut Thread): &mut bool {
    dynamic_field::borrow_mut(&mut self.id, DF_CLOSED)
}

public(package) fun deleted(self: &Thread): &bool {
    dynamic_field::borrow(&self.id, DF_DELETED)
}

public(package) fun deleted_mut(self: &mut Thread): &mut bool {
    dynamic_field::borrow_mut(&mut self.id, DF_DELETED)
}

public(package) fun admin(self: &Thread): &Option<address> {
    dynamic_field::borrow(&self.id, DF_ADMIN)
}

public(package) fun admin_mut(self: &mut Thread): &mut Option<address> {
    dynamic_field::borrow_mut(&mut self.id, DF_ADMIN)
}

public(package) fun mods(self: &Thread): &Table<address, Empty> {
    dynamic_field::borrow(&self.id, DF_MODS)
}

public(package) fun mods_mut(self: &mut Thread): &mut Table<address, Empty> {
    dynamic_field::borrow_mut(&mut self.id, DF_MODS)
}

public(package) fun bans(self: &Thread): &Bans {
    dynamic_field::borrow(&self.id, DF_BANS)
}

public(package) fun bans_mut(self: &mut Thread): &mut Bans {
    dynamic_field::borrow_mut(&mut self.id, DF_BANS)
}

public(package) fun posts(self: &Thread): &Feed<address> {
    dynamic_field::borrow(&self.id, DF_POSTS)
}

public(package) fun posts_mut(self: &mut Thread): &mut Feed<address> {
    dynamic_field::borrow_mut(&mut self.id, DF_POSTS)
}

public(package) fun posts_deleted(self: &Thread): &u64 {
    dynamic_field::borrow(&self.id, DF_POSTS_DELETED)
}

public(package) fun posts_deleted_mut(self: &mut Thread): &mut u64 {
    dynamic_field::borrow_mut(&mut self.id, DF_POSTS_DELETED)
}

public(package) fun last3(self: &Thread): &vector<address> {
    dynamic_field::borrow(&self.id, DF_LAST3)
}

public(package) fun last3_mut(self: &mut Thread): &mut vector<address> {
    dynamic_field::borrow_mut(&mut self.id, DF_LAST3)
}

fun empty(ctx: &mut TxContext): Thread {
    let entity = entity::new(ctx, 0);
    let mut self = Thread { id: object::new(ctx), entity, genesis: true };
    self.do_upgrade(ctx);
    self
}

public(package) fun do_upgrade(self: &mut Thread, ctx: &mut TxContext) {
    if (self.entity.version() < 1) self.init_v1(ctx);
    if (self.entity.version() < 2) self.init_v2(ctx);
    if (self.entity.version() < 3) self.init_v3(ctx);
}

fun init_v1(self: &mut Thread, ctx: &mut TxContext) {
    self.entity.set_version(1);
    let id = self.id();
    dynamic_field::add(&mut self.id, DF_BOARD, @0x0);
    dynamic_field::add(&mut self.id, DF_NUMBER, 0u64);
    dynamic_field::add(&mut self.id, DF_TOPIC_HASH, option::none<u256>());
    dynamic_field::add(&mut self.id, DF_OP, @0x0);
    dynamic_field::add(&mut self.id, DF_CLOSED, false);
    dynamic_field::add(&mut self.id, DF_DELETED, false);
    dynamic_field::add(&mut self.id, DF_PINNED, false);
    dynamic_field::add(&mut self.id, DF_ADMIN, option::none<address>());
    dynamic_field::add(&mut self.id, DF_MODS, table::new<address, Empty>(ctx));
    dynamic_field::add(&mut self.id, DF_BANS, bans::new(ctx, id));
    dynamic_field::add(&mut self.id, DF_POSTS, feed::new<address>(ctx));
    dynamic_field::add(&mut self.id, DF_LAST3, vector<address>[]);
}

fun init_v2(self: &mut Thread, _ctx: &mut TxContext) {
    self.entity.set_version(2);
    let _: bool = dynamic_field::remove(&mut self.id, DF_PINNED);
}

fun init_v3(self: &mut Thread, _ctx: &mut TxContext) {
    self.entity.set_version(3);
    dynamic_field::add(&mut self.id, DF_POSTS_DELETED, 0u64);
}

public(package) fun new(
    ctx: &mut TxContext,
    responses: Responses,
    sender: Sender,
    board: address,
    number: u64,
    topic_hash: Option<u256>,
): Thread {
    let mut self = empty(ctx);
    let mut event = event::new("genesis", responses, sender);

    event = event.with(&board);
    *self.board_mut() = board;

    event = event.with(&number);
    *self.number_mut() = number;

    event = event.with(&topic_hash);
    *self.topic_hash_mut() = topic_hash;

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

public(package) fun set_closed(responses: Responses, sender: Sender, closed: bool): vector<u8> {
    event::new("set_closed", responses, sender).with(&closed).build()
}

public(package) fun set_deleted(responses: Responses, sender: Sender, deleted: bool): vector<u8> {
    event::new("set_deleted", responses, sender).with(&deleted).build()
}

public(package) fun post_set_deleted(responses: Responses, sender: Sender, deleted: bool): vector<u8> {
    event::new("post_set_deleted", responses, sender).with(&deleted).build()
}

public(package) fun post_set_text(
    responses: Responses,
    sender: Sender,
    hash: Option<u256>,
): vector<u8> {
    event::new("post_set_text", responses, sender).with(&hash).build()
}

public(package) fun set_topic(
    responses: Responses,
    sender: Sender,
    topic_hash: Option<u256>,
): vector<u8> {
    event::new("set_topic", responses, sender).with(&topic_hash).build()
}

public(package) fun set_admin(
    responses: Responses,
    sender: Sender,
    admin: Option<address>,
): vector<u8> {
    event::new("set_admin", responses, sender).with(&admin).build()
}

public(package) fun new_post(responses: Responses, sender: Sender, post: address): vector<u8> {
    event::new("new_post", responses, sender).with(&post).build()
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
