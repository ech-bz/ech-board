module forum::user_entry;

use forum::sender::Sender;

public struct UserEntry has copy, drop, store {
    sender: Sender,
    hash: u256,
}

public struct UserEntry2 has copy, drop, store {
    sender: Sender,
    options: vector<u256>,
}

public(package) fun new(sender: Sender, options: vector<u256>): UserEntry2 {
    UserEntry2 { sender, options }
}

public(package) fun sender(self: &UserEntry2): &Sender {
    &self.sender
}

public(package) fun options(self: &UserEntry2): &vector<u256> {
    &self.options
}
