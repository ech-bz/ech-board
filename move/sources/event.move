module forum::event;

use forum::error;
use forum::responses::Responses;
use forum::sender::Sender;
use std::ascii::String;
use sui::bcs;

const VERSION: u16 = 1;

public(package) fun version(): u16 {
    VERSION
}

public struct Event has drop {
    data: vector<u8>,
}

public(package) fun new(name: String, responses: Responses, sender: Sender): Event {
    let version = VERSION;
    let mut data = bcs::to_bytes(&version);
    data.append(bcs::to_bytes(&responses));
    data.append(bcs::to_bytes(&sender));
    data.append(bcs::to_bytes(&name));
    Event { data }
}

public(package) fun with<T>(mut self: Event, value: &T): Event {
    self.data.append(bcs::to_bytes(value));
    self
}

public(package) fun build(self: Event): vector<u8> {
    self.data
}

public(package) fun peel_version(reader: &mut bcs::BCS) {
    assert!(reader.peel_u16() == VERSION, error::event_version_unsupported());
}
