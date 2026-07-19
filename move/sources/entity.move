module forum::entity;

use forum::feed::{Self, Feed};

public struct Entity has store {
    feed: Feed<vector<u8>>,
    version: u8,
}

public(package) fun new(ctx: &mut TxContext, version: u8): Entity {
    Entity {
        feed: feed::new(ctx),
        version,
    }
}

public(package) fun feed_mut(self: &mut Entity): &mut Feed<vector<u8>> {
    &mut self.feed
}
