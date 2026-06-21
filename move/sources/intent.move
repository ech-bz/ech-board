module forum::intent;

use sui::address;
use sui::bcs;
use sui::ed25519;
use sui::event;
use sui::hash;
use sui::object_table::{Self, ObjectTable};
use sui::vec_map::{Self, VecMap};

const SHARD_COUNT: u64 = 1024;

// ----- errors

public enum IntentError has copy, drop, store {
    SignatureInvalid,
    NonceMismatch,
    GateOwnerMismatch,
    TargetMismatch,
    ObjectMismatch,
    ArgsMismatch,
    InvalidShardIndex,
}

public fun code(self: IntentError): u64 {
    match (self) {
        IntentError::SignatureInvalid => 1,
        IntentError::NonceMismatch => 2,
        IntentError::GateOwnerMismatch => 3,
        IntentError::TargetMismatch => 4,
        IntentError::ObjectMismatch => 5,
        IntentError::ArgsMismatch => 6,
        IntentError::InvalidShardIndex => 8,
    }
}

// ----- intent

#[allow(unused_field)]
public struct IntentObject has copy, drop, store {
    id: ID,
    mutable: bool,
}

public struct Intent has copy, drop, store {
    module_name: vector<u8>,
    function: vector<u8>,
    nonce: u64,
    objects: vector<IntentObject>,
    payload: vector<u8>,
    public_key: vector<u8>,
}

public(package) fun payload(self: &Intent): vector<u8> {
    self.payload
}

fun verify_signature(self: &Intent, signature: vector<u8>) {
    let mut data = vector<u8>[3u8, 0u8, 0u8];
    data.append(bcs::to_bytes(self));
    assert!(
        ed25519::ed25519_verify(&signature, &self.public_key, &hash::blake2b256(&data)),
        IntentError::SignatureInvalid.code(),
    );
}

fun verify_objects(self: &Intent, expected_ids: vector<ID>) {
    assert!(self.objects.length() == expected_ids.length(), IntentError::ObjectMismatch.code());
    let mut i = 0;
    while (i < expected_ids.length()) {
        assert!(self.objects[i].id == expected_ids[i], IntentError::ObjectMismatch.code());
        i = i + 1;
    };
}

fun verify_shard(self: &Intent, shard: &IntentGateShard) {
    let digest = hash::blake2b256(&self.public_key);
    let first = digest[0] as u64;
    let second = (digest[1] as u64) << 8;
    let shard_index = (first + second) % SHARD_COUNT;
    assert!(shard.index == shard_index, IntentError::InvalidShardIndex.code());
}

fun sender(self: &Intent): address {
    let mut data = vector<u8>[0u8];
    data.append(self.public_key);
    address::from_bytes(hash::blake2b256(&data))
}

public(package) fun verify(
    self: &Intent,
    ctx: &mut TxContext,
    shard: &mut IntentGateShard,
    signature: vector<u8>,
    expected_module: vector<u8>,
    expected_function: vector<u8>,
    expected_ids: vector<ID>,
) {
    assert!(self.module_name == expected_module, IntentError::TargetMismatch.code());
    assert!(self.function == expected_function, IntentError::TargetMismatch.code());
    self.verify_signature(signature);
    self.verify_objects(expected_ids);
    self.verify_shard(shard);
    shard.gate(ctx, self.sender()).advance_nonce(self.nonce);
}

// bcs

public(package) fun decode(data: vector<u8>): Intent {
    let mut decoded = bcs::new(data);
    let intent = Intent {
        module_name: decoded.peel_vec_u8(),
        function: decoded.peel_vec_u8(),
        nonce: decoded.peel_u64(),
        objects: decoded.peel_vec!(|bcs| {
            IntentObject {
                id: bcs.peel_address().to_id(),
                mutable: bcs.peel_bool(),
            }
        }),
        payload: decoded.peel_vec_u8(),
        public_key: decoded.peel_vec_u8(),
    };
    assert_payload_consumed(decoded);
    intent
}

public(package) fun assert_payload_consumed(decoded: bcs::BCS) {
    assert!(decoded.into_remainder_bytes().is_empty(), IntentError::ArgsMismatch.code());
}

// ----- intent gates

public struct IntentGateRegistry has key, store {
    id: UID,
    shards: VecMap<u64, ID>,
}

public struct IntentGateRegistryCreated has copy, drop {
    registry_id: address,
}

public struct IntentGate has key, store {
    id: UID,
    owner: address,
    nonce: u64,
}

fun advance_nonce(self: &mut IntentGate, nonce: u64) {
    assert!(nonce == self.nonce, IntentError::NonceMismatch.code());
    self.nonce = nonce + 1;
}

public struct IntentGateShard has key, store {
    id: UID,
    registry_id: address,
    index: u64,
    gates: ObjectTable<address, IntentGate>,
}

fun gate(shard: &mut IntentGateShard, ctx: &mut TxContext, sender: address): &mut IntentGate {
    if (!shard.gates.contains(sender)) {
        shard
            .gates
            .add(
                sender,
                IntentGate {
                    id: object::new(ctx),
                    owner: sender,
                    nonce: 0,
                },
            );
    };

    let gate = &mut shard.gates[sender];
    assert!(gate.owner == sender, IntentError::GateOwnerMismatch.code());
    gate
}

// ----- init

fun init(ctx: &mut TxContext) {
    let mut registry = IntentGateRegistry {
        id: object::new(ctx),
        shards: vec_map::empty(),
    };
    let registry_id = object::id(&registry).to_address();
    let mut index = 0u64;
    while (index < SHARD_COUNT) {
        let shard = IntentGateShard {
            id: object::new(ctx),
            registry_id,
            index,
            gates: object_table::new(ctx),
        };
        registry.shards.insert(index, object::id(&shard));
        transfer::share_object(shard);
        index = index + 1;
    };
    event::emit(IntentGateRegistryCreated { registry_id });
    transfer::share_object(registry);
}
