module forum::intent;

use sui::address;
use sui::bcs;
use sui::ed25519;
use sui::hash;

public enum IntentError has copy, drop, store {
    SignatureInvalid,
    TargetMismatch,
    ObjectMismatch,
    ArgsMismatch,
}

public fun code(self: IntentError): u64 {
    match (self) {
        IntentError::SignatureInvalid => 1,
        IntentError::TargetMismatch => 2,
        IntentError::ObjectMismatch => 3,
        IntentError::ArgsMismatch => 4,
    }
}

#[allow(unused_field)]
public struct IntentObject has copy, drop, store {
    id: ID,
    mutable: bool,
}

public struct Intent {
    module_name: vector<u8>,
    function: vector<u8>,
    nonce: u64,
    objects: vector<IntentObject>,
    bcs: bcs::BCS,
    public_key: address,
    tweak: address,
    uid: vector<u8>,
}

public(package) fun bcs(self: &mut Intent): &mut bcs::BCS {
    &mut self.bcs
}

public(package) fun decode(
    data: vector<u8>,
    expected_module: vector<u8>,
    expected_function: vector<u8>,
    signature: vector<u8>,
    expected_ids: vector<ID>,
): Intent {
    let mut bcs = bcs::new(data);
    let intent = Intent {
        module_name: bcs.peel_vec_u8(),
        function: bcs.peel_vec_u8(),
        nonce: bcs.peel_u64(),
        objects: bcs.peel_vec!(|bcs| {
            IntentObject {
                id: bcs.peel_address().to_id(),
                mutable: bcs.peel_bool(),
            }
        }),
        bcs: bcs::new(bcs.peel_vec_u8()),
        public_key: bcs.peel_address(),
        tweak: bcs.peel_address(),
        uid: bcs.peel_vec_u8(),
    };
    assert!(bcs.into_remainder_bytes().is_empty(), IntentError::ArgsMismatch.code());

    assert!(intent.module_name == expected_module, IntentError::TargetMismatch.code());
    assert!(intent.function == expected_function, IntentError::TargetMismatch.code());

    assert!(
        ed25519::ed25519_verify(&signature, &intent.public_key.to_bytes(), &hash::blake2b256(&data)),
        IntentError::SignatureInvalid.code(),
    );

    assert!(intent.objects.length() == expected_ids.length(), IntentError::ObjectMismatch.code());
    let mut i = 0;
    while (i < expected_ids.length()) {
        assert!(intent.objects[i].id == expected_ids[i], IntentError::ObjectMismatch.code());
        i = i + 1;
    };

    intent
}

public(package) fun end(self: Intent) {
    assert!(self.bcs.into_remainder_bytes().is_empty(), IntentError::ArgsMismatch.code());
    let Intent { .. } = self;
}

public(package) fun nonce(self: &Intent): u64 {
    self.nonce
}

public(package) fun sender(self: &Intent): address {
    self.public_key
}

public(package) fun tweak(self: &Intent): address {
    self.tweak
}
