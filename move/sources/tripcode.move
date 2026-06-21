module forum::tripcode;

use std::ascii::String;

public struct Tripcode has copy, drop, store {
    secured: bool,
    trip: String,
}

public(package) fun new(secured: bool, trip: String): Tripcode {
    Tripcode { secured, trip }
}
