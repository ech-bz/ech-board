module forum::entity;

use forum::feed::{Self, Feed};

public struct Entity has store {
    feed: Feed<vector<u8>>,
    version: u16,
}

public(package) fun new(ctx: &mut TxContext, version: u16): Entity {
    Entity {
        feed: feed::new(ctx),
        version,
    }
}

public(package) fun feed_mut(self: &mut Entity): &mut Feed<vector<u8>> {
    &mut self.feed
}

public(package) fun version(self: &Entity): u16 {
    self.version
}

public(package) fun set_version(self: &mut Entity, version: u16) {
    self.version = version;
}
