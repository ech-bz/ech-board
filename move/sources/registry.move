module forum::registry;

use sui::table::{Self, Table};

public struct Registry<phantom T: store> has store {
    counter: u64,
    entries: Table<u64, T>,
    identities: Table<u64, vector<u256>>,
    index: Table<u256, u64>,
}

public fun new<T: store>(ctx: &mut TxContext): Registry<T> {
    Registry<T> {
        counter: 0,
        entries: table::new(ctx),
        identities: table::new(ctx),
        index: table::new(ctx),
    }
}

public(package) fun add<T: store>(self: &mut Registry<T>, hashes: vector<u256>, entry: T): u64 {
    hashes.do_ref!(|h| self.index.add(*h, self.counter));
    self.identities.add(self.counter, hashes);
    self.entries.add(self.counter, entry);
    self.counter = self.counter + 1;
    self.counter - 1
}

public(package) fun find<T: store>(self: &Registry<T>, hash: u256): Option<u64> {
    if (self.index.contains(hash)) {
        option::some(*self.index.borrow(hash))
    } else {
        option::none()
    }
}

public(package) fun remove<T: store>(self: &mut Registry<T>, id: u64): T {
    self.identities.remove(id).do!(|h| self.index.remove(h));
    self.entries.remove(id)
}

public(package) fun entry<T: store>(self: &Registry<T>, id: u64): &T {
    &self.entries[id]
}

public(package) fun entry_mut<T: store>(self: &mut Registry<T>, id: u64): &mut T {
    &mut self.entries[id]
}
