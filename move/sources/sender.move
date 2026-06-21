module forum::sender;

use sui::address;
use sui::bcs;
use sui::hash;

public struct Sender has copy, drop, store {
    pk: u256,
    tweak: u256,
}

public fun new(pk: u256, tweak: u256): Sender {
    Sender { pk, tweak }
}

public(package) fun peel(reader: &mut bcs::BCS): Sender {
    new(reader.peel_u256(), reader.peel_u256())
}

public fun pk(self: &Sender): u256 {
    self.pk
}

public fun tweak(self: &Sender): u256 {
    self.tweak
}

public fun addr(self: &Sender): address {
    let mut data = vector[0u8];
    data.append(bcs::to_bytes(&self.pk));
    address::from_bytes(hash::blake2b256(&data))
}
