module forum::sharded_counter;

use forum::error;
use sui::address;
use sui::bcs;
use sui::derived_object;
use sui::hash;
use sui::table::{Self, Table};

public struct ShardedCounter<phantom Key> has key, store {
    id: UID,
}

public(package) fun new<Key>(ctx: &mut TxContext, shards: u64): ShardedCounter<Key> {
    let mut self = ShardedCounter {
        id: object::new(ctx),
    };
    let mut index = 0u64;
    while (index < shards) {
        transfer::share_object(Shard<Key> {
            id: derived_object::claim(&mut self.id, index),
            shards,
            index,
            counters: table::new(ctx),
        });
        index = index + 1;
    };
    self
}

public struct Shard<phantom Key> has key, store {
    id: UID,
    shards: u64,
    index: u64,
    counters: Table<address, u64>,
}

fun advance<Key>(self: &mut Shard<Key>, key: &Key, forward: bool): u64 {
    let key_addr = address::from_bytes(hash::blake2b256(&bcs::to_bytes(key)));
    let shard_index = (key_addr.to_u256() % (self.shards as u256)) as u64;
    assert!(self.index == shard_index, error::sharded_counter_index_mismatch());

    if (!self.counters.contains(key_addr)) {
        self.counters.add(key_addr, 0);
    };
    let value = &mut self.counters[key_addr];

    if (forward) {
        *value = *value + 1;
    } else {
        *value = *value - 1;
    };

    *value
}

public(package) fun inc<Key>(self: &mut Shard<Key>, key: &Key) {
    self.advance(key, true);
}

public(package) fun inc_checked<Key>(self: &mut Shard<Key>, key: &Key, current: u64) {
    let value = self.advance(key, true);
    assert!(value == current + 1, error::sharded_counter_value_mismatch());
}

public(package) fun dec<Key>(self: &mut Shard<Key>, key: &Key) {
    self.advance(key, false);
}

public(package) fun dec_checked<Key>(self: &mut Shard<Key>, key: &Key, current: u64) {
    let value = self.advance(key, false);
    assert!(value == current - 1, error::sharded_counter_value_mismatch());
}
