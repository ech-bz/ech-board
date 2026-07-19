module forum::bans;

use forum::registry::{Self, Registry};

public struct BanKey has copy, drop, store {
    level: address,
    mask: u8,
    ip_hash: u256,
}

public fun key(level: address, mask: u8, ip_hash: u256): BanKey {
    BanKey { level, mask, ip_hash }
}

public struct BanValue has drop, store {
    reason_hash: u256,
    expires: u64,
}

public fun value(reason_hash: u256, expires: u64): BanValue {
    BanValue { reason_hash, expires }
}

public struct Bans has store {
    level: address,
    ip32: Registry<BanValue>,
    ip24: Registry<BanValue>,
    ip20: Registry<BanValue>,
    ip16: Registry<BanValue>,
}

fun subnet(self: &mut Bans, mask: u8): &mut Registry<BanValue> {
    match (mask) {
        32 => &mut self.ip32,
        24 => &mut self.ip24,
        20 => &mut self.ip20,
        16 => &mut self.ip16,
        _ => abort,
    }
}

public fun new(ctx: &mut TxContext, level: address): Bans {
    Bans {
        level,
        ip32: registry::new(ctx),
        ip24: registry::new(ctx),
        ip20: registry::new(ctx),
        ip16: registry::new(ctx),
    }
}

public(package) fun ban(self: &mut Bans, key: BanKey, value: BanValue) {
    assert!(key.level == self.level);
    let bans = self.subnet(key.mask);
    let id = bans.find(key.ip_hash);
    if (id.is_some()) {
        let entry = bans.entry_mut(*id.borrow());
        entry.reason_hash = value.reason_hash;
        entry.expires = value.expires;
    } else {
        bans.add(vector[key.ip_hash], value);
    };
}

public(package) fun unban(self: &mut Bans, key: BanKey) {
    assert!(key.level == self.level);
    let bans = self.subnet(key.mask);
    let id_opt = bans.find(key.ip_hash);
    if (id_opt.is_some()) {
        bans.remove(*id_opt.borrow());
    };
}
