module forum::sharded_counter;

use sui::address;
use sui::bcs;
use sui::derived_object;
use sui::hash;
use sui::table::{Self, Table};

public enum ShardedCounterError has copy, drop, store {
    ValueMismatch,
    InvalidShardIndex,
}

public fun code(self: ShardedCounterError): u64 {
    match (self) {
        ShardedCounterError::ValueMismatch => 1,
        ShardedCounterError::InvalidShardIndex => 2,
    }
}

public struct ShardedCounter<phantom Key> has key, store {
    id: UID,
}

public fun new<Key>(ctx: &mut TxContext, shards: u64): ShardedCounter<Key> {
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

public fun single_shard<Key>(ctx: &mut TxContext): Shard<Key> {
    Shard {
        id: object::new(ctx),
        shards: 1,
        index: 0,
        counters: table::new(ctx),
    }
}

fun advance<Key>(self: &mut Shard<Key>, key: &Key, forward: bool): u64 {
    let key_addr = address::from_bytes(hash::blake2b256(&bcs::to_bytes(key)));
    let shard_index = (key_addr.to_u256() % (self.shards as u256)) as u64;
    assert!(self.index == shard_index, ShardedCounterError::InvalidShardIndex.code());

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

public fun inc<Key>(self: &mut Shard<Key>, key: &Key) {
    self.advance(key, true);
}

public fun inc_checked<Key>(self: &mut Shard<Key>, key: &Key, current: u64) {
    let value = self.advance(key, true);
    assert!(value == current + 1, ShardedCounterError::ValueMismatch.code());
}

public fun dec<Key>(self: &mut Shard<Key>, key: &Key) {
    self.advance(key, false);
}

public fun dec_checked<Key>(self: &mut Shard<Key>, key: &Key, current: u64) {
    let value = self.advance(key, false);
    assert!(value == current - 1, ShardedCounterError::ValueMismatch.code());
}
