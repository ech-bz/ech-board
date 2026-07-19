module forum::forum;

use forum::bans::{Self, BanKey, BanValue, Bans};
use forum::board::{Self, Board};
use forum::empty::{Self, Empty};
use forum::entity::{Self, Entity};
use forum::event;
use forum::post::{Self, Post};
use forum::sender::{Self, Sender};
use forum::sharded_counter::{Self, ShardedCounter};
use forum::thread::{Self, Thread};
use std::ascii::String;
use sui::bcs;
use sui::dynamic_field;
use sui::table::{Self, Table};

public struct Forum has key {
    id: UID,
    entity: Entity,
    genesis: bool,
}

public use fun forum::forum_apply::apply as Forum.apply;
public use fun forum::forum_apply::apply_post as Forum.apply_post;

public(package) fun push(self: &mut Forum, event: vector<u8>) {
    self.entity.feed_mut().push(event).share();
}

public(package) fun id(self: &Forum): address {
    self.id.to_address()
}

public(package) fun genesis(self: &Forum): bool {
    self.genesis
}

public(package) fun share(mut self: Forum) {
    self.genesis = false;
    transfer::share_object(self)
}

const VERSION: u8 = 0;

const DF_NONCE_SHARDS: vector<u8> = b"nonce_shards";
const DF_ADMIN: vector<u8> = b"admin";
const DF_MODS: vector<u8> = b"moderators";
const DF_BANS: vector<u8> = b"bans";
const DF_BOARDS: vector<u8> = b"boards";
const DF_TIMESTAMP_PRECISION: vector<u8> = b"timestamp_precision";

public(package) fun nonce_shards(self: &Forum): &ShardedCounter<address> {
    dynamic_field::borrow(&self.id, DF_NONCE_SHARDS)
}

public(package) fun nonce_shards_mut(self: &mut Forum): &mut ShardedCounter<address> {
    dynamic_field::borrow_mut(&mut self.id, DF_NONCE_SHARDS)
}

public(package) fun admin(self: &Forum): &address {
    dynamic_field::borrow(&self.id, DF_ADMIN)
}

public(package) fun admin_mut(self: &mut Forum): &mut address {
    dynamic_field::borrow_mut(&mut self.id, DF_ADMIN)
}

public(package) fun mods(self: &Forum): &Table<address, Empty> {
    dynamic_field::borrow(&self.id, DF_MODS)
}

public(package) fun mods_mut(self: &mut Forum): &mut Table<address, Empty> {
    dynamic_field::borrow_mut(&mut self.id, DF_MODS)
}

public(package) fun bans(self: &Forum): &Bans {
    dynamic_field::borrow(&self.id, DF_BANS)
}

public(package) fun bans_mut(self: &mut Forum): &mut Bans {
    dynamic_field::borrow_mut(&mut self.id, DF_BANS)
}

public(package) fun boards(self: &Forum): &Table<String, address> {
    dynamic_field::borrow(&self.id, DF_BOARDS)
}

public(package) fun boards_mut(self: &mut Forum): &mut Table<String, address> {
    dynamic_field::borrow_mut(&mut self.id, DF_BOARDS)
}

public(package) fun timestamp_precision(self: &Forum): &u64 {
    dynamic_field::borrow(&self.id, DF_TIMESTAMP_PRECISION)
}

public(package) fun timestamp_precision_mut(self: &mut Forum): &mut u64 {
    dynamic_field::borrow_mut(&mut self.id, DF_TIMESTAMP_PRECISION)
}

fun empty(ctx: &mut TxContext): Forum {
    let entity = entity::new(ctx, VERSION);
    let mut self = Forum { id: object::new(ctx), entity, genesis: true };
    let id = self.id();
    dynamic_field::add(&mut self.id, DF_NONCE_SHARDS, sharded_counter::new<address>(ctx, 512));
    dynamic_field::add(&mut self.id, DF_ADMIN, @0x0);
    dynamic_field::add(&mut self.id, DF_MODS, table::new<address, Empty>(ctx));
    dynamic_field::add(&mut self.id, DF_BANS, bans::new(ctx, id));
    dynamic_field::add(&mut self.id, DF_BOARDS, table::new<String, address>(ctx));
    dynamic_field::add(&mut self.id, DF_TIMESTAMP_PRECISION, 0u64);
    self
}

public(package) fun new(
    ctx: &mut TxContext,
    sender: Sender,
    uid: vector<u8>,
    admin: address,
): Forum {
    let mut self = empty(ctx);
    let mut event = event::new("genesis", sender, uid);

    event = event.with(&admin);
    *self.admin_mut() = admin;

    self.push(event.build());
    self
}

public(package) fun add_moderator(sender: Sender, uid: vector<u8>, moderator: address): vector<u8> {
    event::new("add_moderator", sender, uid).with(&moderator).build()
}

public(package) fun del_moderator(sender: Sender, uid: vector<u8>, moderator: address): vector<u8> {
    event::new("del_moderator", sender, uid).with(&moderator).build()
}

public(package) fun new_board(
    sender: Sender,
    uid: vector<u8>,
    slug: String,
    max_media: u64,
    bump_limit: u64,
    desc_hash: Option<u256>,
): vector<u8> {
    event::new("new_board", sender, uid)
        .with(&slug)
        .with(&max_media)
        .with(&bump_limit)
        .with(&desc_hash)
        .build()
}

public(package) fun set_timestamp_precision(
    sender: Sender,
    uid: vector<u8>,
    precision: u64,
): vector<u8> {
    event::new("set_timestamp_precision", sender, uid).with(&precision).build()
}

public(package) fun ban(sender: Sender, uid: vector<u8>, key: BanKey, value: BanValue): vector<u8> {
    event::new("ban", sender, uid).with(&key).with(&value).build()
}

public(package) fun unban(sender: Sender, uid: vector<u8>, key: BanKey): vector<u8> {
    event::new("unban", sender, uid).with(&key).build()
}
