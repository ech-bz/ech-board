module forum::nonce;

use sui::event;
use sui::object_table::{Self, ObjectTable};

const SHARD_COUNT: u64 = 1024;

// ----- errors

public enum NonceError has copy, drop, store {
    NonceMismatch,
    InvalidShardIndex,
}

public fun code(self: NonceError): u64 {
    match (self) {
        NonceError::NonceMismatch => 1,
        NonceError::InvalidShardIndex => 2,
    }
}

// ----- nonce

public struct NonceGateShard has key, store {
    id: UID,
    index: u64,
    gates: ObjectTable<address, NonceGate>,
}

public struct NonceGateShardCreated has copy, drop {
    shard_id: ID,
}

public struct NonceGate has key, store {
    id: UID,
    nonce: u64,
}

public(package) fun advance(
    self: &mut NonceGateShard,
    ctx: &mut TxContext,
    sender: address,
    nonce: u64,
) {
    let shard_index = (sender.to_u256() % (SHARD_COUNT as u256)) as u64;
    assert!(self.index == shard_index, NonceError::InvalidShardIndex.code());

    if (!self.gates.contains(sender)) {
        let gate = NonceGate {
            id: object::new(ctx),
            nonce: 0,
        };
        self.gates.add(sender, gate);
    };
    let gate = &mut self.gates[sender];
    assert!(gate.nonce == nonce, NonceError::NonceMismatch.code());
    gate.nonce = nonce + 1;
}

// ----- init

fun init(ctx: &mut TxContext) {
    let mut index = 0u64;
    while (index < SHARD_COUNT) {
        let shard = NonceGateShard {
            id: object::new(ctx),
            index,
            gates: object_table::new(ctx),
        };
        event::emit(NonceGateShardCreated { shard_id: object::id(&shard) });
        transfer::share_object(shard);
        index = index + 1;
    };
}
