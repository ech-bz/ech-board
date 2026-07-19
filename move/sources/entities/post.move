module forum::post;

use forum::bans::{BanKey, BanValue};
use forum::entity::{Self, Entity};
use forum::event;
use forum::registry::{Self, Registry};
use forum::sender::{Self, Sender};
use forum::user_entry::{Self, UserEntry};
use std::ascii::String;
use sui::bcs;
use sui::dynamic_field;
use sui::vec_map::{Self, VecMap};

public struct Post has key {
    id: UID,
    entity: Entity,
    genesis: bool,
}

public use fun forum::post_apply::apply as Post.apply;

public(package) fun push(self: &mut Post, event: vector<u8>) {
    self.entity.feed_mut().push(event).share();
}

public(package) fun id(self: &Post): address {
    self.id.to_address()
}

public(package) fun genesis(self: &Post): bool {
    self.genesis
}

public(package) fun share(mut self: Post) {
    self.genesis = false;
    transfer::share_object(self)
}

const VERSION: u8 = 0;

const DF_SENDER: vector<u8> = b"sender";
const DF_THREAD: vector<u8> = b"thread";
const DF_NUMBER: vector<u8> = b"number";
const DF_UID: vector<u8> = b"uid";
const DF_TS: vector<u8> = b"timestamp_ms";
const DF_DELETED: vector<u8> = b"deleted";
const DF_BANNED: vector<u8> = b"banned";
const DF_TEXT_HASH: vector<u8> = b"text_hash";
const DF_MEDIA_HASHES: vector<u8> = b"media_hashes";
const DF_REACTIONS: vector<u8> = b"reactions";
const DF_REACTED: vector<u8> = b"reacted";
const DF_VOTES: vector<u8> = b"votes";
const DF_VOTED: vector<u8> = b"voted";

public(package) fun sender(self: &Post): &Sender {
    dynamic_field::borrow(&self.id, DF_SENDER)
}

public(package) fun sender_mut(self: &mut Post): &mut Sender {
    dynamic_field::borrow_mut(&mut self.id, DF_SENDER)
}

public(package) fun thread(self: &Post): &address {
    dynamic_field::borrow(&self.id, DF_THREAD)
}

public(package) fun thread_mut(self: &mut Post): &mut address {
    dynamic_field::borrow_mut(&mut self.id, DF_THREAD)
}

public(package) fun number(self: &Post): &u64 {
    dynamic_field::borrow(&self.id, DF_NUMBER)
}

public(package) fun number_mut(self: &mut Post): &mut u64 {
    dynamic_field::borrow_mut(&mut self.id, DF_NUMBER)
}

public(package) fun uid(self: &Post): &vector<u8> {
    dynamic_field::borrow(&self.id, DF_UID)
}

public(package) fun uid_mut(self: &mut Post): &mut vector<u8> {
    dynamic_field::borrow_mut(&mut self.id, DF_UID)
}

public(package) fun timestamp(self: &Post): &u64 {
    dynamic_field::borrow(&self.id, DF_TS)
}

public(package) fun timestamp_mut(self: &mut Post): &mut u64 {
    dynamic_field::borrow_mut(&mut self.id, DF_TS)
}

public(package) fun deleted(self: &Post): &bool {
    dynamic_field::borrow(&self.id, DF_DELETED)
}

public(package) fun deleted_mut(self: &mut Post): &mut bool {
    dynamic_field::borrow_mut(&mut self.id, DF_DELETED)
}

public(package) fun banned(self: &Post): &Option<BanKey> {
    dynamic_field::borrow(&self.id, DF_BANNED)
}

public(package) fun banned_mut(self: &mut Post): &mut Option<BanKey> {
    dynamic_field::borrow_mut(&mut self.id, DF_BANNED)
}

public(package) fun text_hash(self: &Post): &Option<u256> {
    dynamic_field::borrow(&self.id, DF_TEXT_HASH)
}

public(package) fun text_hash_mut(self: &mut Post): &mut Option<u256> {
    dynamic_field::borrow_mut(&mut self.id, DF_TEXT_HASH)
}

public(package) fun media_hashes(self: &Post): &vector<u256> {
    dynamic_field::borrow(&self.id, DF_MEDIA_HASHES)
}

public(package) fun media_hashes_mut(self: &mut Post): &mut vector<u256> {
    dynamic_field::borrow_mut(&mut self.id, DF_MEDIA_HASHES)
}

public(package) fun reactions(self: &Post): &VecMap<u256, u64> {
    dynamic_field::borrow(&self.id, DF_REACTIONS)
}

public(package) fun reactions_mut(self: &mut Post): &mut VecMap<u256, u64> {
    dynamic_field::borrow_mut(&mut self.id, DF_REACTIONS)
}

public(package) fun reacted(self: &Post): &Registry<UserEntry> {
    dynamic_field::borrow(&self.id, DF_REACTED)
}

public(package) fun reacted_mut(self: &mut Post): &mut Registry<UserEntry> {
    dynamic_field::borrow_mut(&mut self.id, DF_REACTED)
}

public(package) fun votes(self: &Post): &VecMap<u256, u64> {
    dynamic_field::borrow(&self.id, DF_VOTES)
}

public(package) fun votes_mut(self: &mut Post): &mut VecMap<u256, u64> {
    dynamic_field::borrow_mut(&mut self.id, DF_VOTES)
}

public(package) fun voted(self: &Post): &Registry<UserEntry> {
    dynamic_field::borrow(&self.id, DF_VOTED)
}

public(package) fun voted_mut(self: &mut Post): &mut Registry<UserEntry> {
    dynamic_field::borrow_mut(&mut self.id, DF_VOTED)
}

fun empty(ctx: &mut TxContext): Post {
    let entity = entity::new(ctx, VERSION);
    let mut post = Post { id: object::new(ctx), entity, genesis: true };
    dynamic_field::add(&mut post.id, DF_SENDER, sender::new(0, 0));
    dynamic_field::add(&mut post.id, DF_THREAD, @0x0);
    dynamic_field::add(&mut post.id, DF_NUMBER, 0u64);
    dynamic_field::add(&mut post.id, DF_UID, vector<u8>[]);
    dynamic_field::add(&mut post.id, DF_TS, 0u64);
    dynamic_field::add(&mut post.id, DF_DELETED, false);
    dynamic_field::add(&mut post.id, DF_BANNED, option::none<BanKey>());
    dynamic_field::add(&mut post.id, DF_TEXT_HASH, option::none<u256>());
    dynamic_field::add(&mut post.id, DF_MEDIA_HASHES, vector<u256>[]);
    dynamic_field::add(&mut post.id, DF_REACTIONS, vec_map::empty<u256, u64>());
    dynamic_field::add(&mut post.id, DF_REACTED, registry::new<UserEntry>(ctx));
    dynamic_field::add(&mut post.id, DF_VOTES, vec_map::empty<u256, u64>());
    dynamic_field::add(&mut post.id, DF_VOTED, registry::new<UserEntry>(ctx));
    post
}

public(package) fun new(
    ctx: &mut TxContext,
    sender: Sender,
    uid: vector<u8>,
    thread: address,
    number: u64,
    timestamp_ms: u64,
    text_hash: Option<u256>,
    media_hashes: vector<u256>,
    vote_keys: vector<u256>,
): Post {
    let mut self = empty(ctx);
    let mut event = event::new("genesis", sender, uid);
    *self.sender_mut() = sender;

    event = event.with(&uid);
    *self.uid_mut() = uid;

    event = event.with(&thread);
    *self.thread_mut() = thread;

    event = event.with(&number);
    *self.number_mut() = number;

    event = event.with(&timestamp_ms);
    *self.timestamp_mut() = timestamp_ms;

    event = event.with(&text_hash);
    *self.text_hash_mut() = text_hash;

    event = event.with(&media_hashes);
    self.media_hashes_mut().append(media_hashes);

    event = event.with(&vote_keys);
    vote_keys.do!(|key| self.votes_mut().insert(key, 0));

    self.push(event.build());
    self
}

public(package) fun set_deleted(sender: Sender, uid: vector<u8>, deleted: bool): vector<u8> {
    event::new("set_deleted", sender, uid).with(&deleted).build()
}

public(package) fun set_text(sender: Sender, uid: vector<u8>, hash: Option<u256>): vector<u8> {
    event::new("set_text", sender, uid).with(&hash).build()
}

public(package) fun remove_media(
    sender: Sender,
    uid: vector<u8>,
    hashes: vector<u256>,
): vector<u8> {
    event::new("remove_media", sender, uid).with(&hashes).build()
}

public(package) fun set_reaction(
    sender: Sender,
    uid: vector<u8>,
    ip32_hash: u256,
    reaction_hash: u256,
): vector<u8> {
    event::new("set_reaction", sender, uid).with(&ip32_hash).with(&reaction_hash).build()
}

public(package) fun vote(
    sender: Sender,
    uid: vector<u8>,
    ip32_hash: u256,
    option_hash: u256,
): vector<u8> {
    event::new("vote", sender, uid).with(&ip32_hash).with(&option_hash).build()
}

public(package) fun set_banned(sender: Sender, uid: vector<u8>, key: Option<BanKey>): vector<u8> {
    event::new("set_banned", sender, uid).with(&key).build()
}
