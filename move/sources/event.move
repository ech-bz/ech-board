module forum::event;

use forum::sender::Sender;
use std::ascii::String;
use sui::bcs;

public struct Event has drop {
    data: vector<u8>,
}

public(package) fun new(name: String, sender: Sender, uid: vector<u8>): Event {
    let mut data = bcs::to_bytes(&name);
    data.append(bcs::to_bytes(&sender));
    data.append(bcs::to_bytes(&uid));
    Event { data }
}

public(package) fun with<T>(mut self: Event, value: &T): Event {
    self.data.append(bcs::to_bytes(value));
    self
}

public(package) fun build(self: Event): vector<u8> {
    self.data
}
