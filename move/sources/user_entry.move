module forum::user_entry;

use forum::sender::Sender;

public struct UserEntry has copy, drop, store {
    sender: Sender,
    hash: u256,
}

public(package) fun new(sender: Sender, hash: u256): UserEntry {
    UserEntry { sender, hash }
}

public(package) fun sender(self: &UserEntry): &Sender {
    &self.sender
}

public(package) fun hash(self: &UserEntry): &u256 {
    &self.hash
}

public(package) fun hash_mut(self: &mut UserEntry): &mut u256 {
    &mut self.hash
}
