module forum::responses;

use forum::tripcode::{Self, Tripcode};
use std::ascii;
use sui::bcs;

public struct Responses has copy, drop, store {
    uid: Option<vector<u8>>,
    ip32: Option<u256>,
    tripcode: Option<Tripcode>,
    geo: Option<u32>,
}

public(package) fun new(
    uid: Option<vector<u8>>,
    ip32: Option<u256>,
    tripcode: Option<Tripcode>,
    geo: Option<u32>,
): Responses {
    Responses { uid, ip32, tripcode, geo }
}

public(package) fun peel(reader: &mut bcs::BCS): Responses {
    Responses {
        uid: reader.peel_option!(|b| b.peel_vec_u8()),
        ip32: reader.peel_option!(|b| b.peel_u256()),
        tripcode: reader.peel_option!(|b| {
            tripcode::new(b.peel_bool(), ascii::string(b.peel_vec_u8()))
        }),
        geo: reader.peel_option!(|b| b.peel_u32()),
    }
}

public(package) fun uid(self: &Responses): &Option<vector<u8>> {
    &self.uid
}

public(package) fun ip32(self: &Responses): &Option<u256> {
    &self.ip32
}

public(package) fun tripcode(self: &Responses): &Option<Tripcode> {
    &self.tripcode
}

public(package) fun geo(self: &Responses): &Option<u32> {
    &self.geo
}
