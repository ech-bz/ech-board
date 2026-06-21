module forum::feed;

use sui::derived_object;

public struct FeedEntry<T: store> has key {
    id: UID,
    value: T,
}

public fun share<T: store>(self: FeedEntry<T>) {
    transfer::share_object(self);
}

public fun value<T: store>(self: &FeedEntry<T>): &T {
    &self.value
}

public fun value_mut<T: store>(self: &mut FeedEntry<T>): &mut T {
    &mut self.value
}

public struct Feed<phantom T: store> has key, store {
    id: UID,
    counter: u64,
}

public fun new<T: store>(ctx: &mut TxContext): Feed<T> {
    Feed { id: object::new(ctx), counter: 0 }
}

public fun push<T: store>(self: &mut Feed<T>, value: T): FeedEntry<T> {
    self.counter = self.counter + 1;
    FeedEntry {
        id: derived_object::claim(&mut self.id, self.counter),
        value,
    }
}

public fun next<T: store>(self: &Feed<T>): u64 {
    self.counter + 1
}
