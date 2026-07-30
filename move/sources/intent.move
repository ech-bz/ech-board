module forum::intent;

use forum::error;
use forum::sender::{Self, Sender};
use std::ascii::{Self, String};
use sui::bcs;
use sui::ed25519;
use sui::hash;

public enum Request has copy, drop, store {
    Uid,
    Ip32(address),
}

public(package) fun request_uid(): Request {
    Request::Uid
}

public(package) fun request_ip32(domain: address): Request {
    Request::Ip32(domain)
}

#[allow(unused_field)]
public struct IntentObject has copy, drop, store {
    id: ID,
    mutable: bool,
}

public struct IntentResponses has drop, store {
    uid: Option<vector<u8>>,
    ip32: Option<u256>,
}

public struct Intent {
    module_name: String,
    function: String,
    nonce: u64,
    objects: vector<IntentObject>,
    requests: vector<Request>,
    event: vector<u8>,
    sender: Sender,
}

public(package) fun into_event(self: Intent): vector<u8> {
    let Intent { event, .. } = self;
    event
}

public(package) fun uid(self: &IntentResponses): vector<u8> {
    *self.uid.borrow()
}

public(package) fun ip32(self: &IntentResponses): u256 {
    *self.ip32.borrow()
}

public(package) fun decode(
    data: vector<u8>,
    expected_module: String,
    expected_function: String,
    signature: vector<u8>,
    expected_requests: vector<Request>,
    responses: vector<u8>,
    expected_ids: vector<ID>,
    allowed_events: vector<String>,
): Intent {
    let mut intent_bcs = bcs::new(data);
    let mut intent = Intent {
        module_name: ascii::string(intent_bcs.peel_vec_u8()),
        function: ascii::string(intent_bcs.peel_vec_u8()),
        nonce: intent_bcs.peel_u64(),
        objects: intent_bcs.peel_vec!(|bcs| {
            IntentObject {
                id: bcs.peel_address().to_id(),
                mutable: bcs.peel_bool(),
            }
        }),
        requests: intent_bcs.peel_vec!(
            |bcs| match (bcs.peel_enum_tag()) {
                0 => Request::Uid,
                1 => Request::Ip32(bcs.peel_address()),
                _ => abort error::intent_args_mismatch(),
            },
        ),
        event: intent_bcs.peel_vec_u8(),
        sender: sender::new(intent_bcs.peel_u256(), intent_bcs.peel_u256()),
    };
    assert!(intent_bcs.into_remainder_bytes().is_empty(), error::intent_args_mismatch());

    assert!(intent.module_name == expected_module, error::intent_target_mismatch());
    assert!(intent.function == expected_function, error::intent_target_mismatch());
    assert!(intent.requests == expected_requests, error::intent_args_mismatch());

    assert!(
        ed25519::ed25519_verify(
            &signature,
            &bcs::to_bytes(&intent.sender.pk()),
            &hash::blake2b256(&data),
        ),
        error::intent_signature_invalid(),
    );

    intent
        .objects
        .zip_do_ref!(
            &expected_ids,
            |obj, id| assert!(obj.id == id, error::intent_object_mismatch()),
        );

    let mut responses = bcs::new(responses);
    let relay_sig = responses.peel_vec_u8();
    let relay_pk = responses.peel_u256();
    let response_bytes = responses.into_remainder_bytes();
    let mut message = signature;
    message.append(copy response_bytes);
    assert!(
        ed25519::ed25519_verify(
            &relay_sig,
            &bcs::to_bytes(&relay_pk),
            &hash::blake2b256(&message),
        ),
        error::intent_relay_signature_invalid(),
    );
    let mut responses = bcs::new(response_bytes);

    let mut event = bcs::new(intent.event);
    let event_tag = event.peel_vec_u8();
    assert!(allowed_events.any!(|tag| tag.as_bytes() == event_tag), error::intent_args_mismatch());
    intent.event = bcs::to_bytes(&event_tag);
    let sender = intent.sender();
    intent.event.append(bcs::to_bytes(&sender));

    let mut uid = option::none();
    let mut ip32 = option::none();
    intent
        .requests
        .do_ref!(
            |req| match (req) {
                Request::Uid => uid.fill(responses.peel_vec_u8()),
                Request::Ip32(_) => ip32.fill(responses.peel_u256()),
            },
        );
    assert!(responses.into_remainder_bytes().is_empty(), error::intent_args_mismatch());
    uid.do!(|v| intent.event.append(bcs::to_bytes(&v)));
    ip32.do!(|v| intent.event.append(bcs::to_bytes(&v)));

    intent.event.append(event.into_remainder_bytes());

    intent
}

public(package) fun nonce(self: &Intent): u64 {
    self.nonce
}

public(package) fun sender(self: &Intent): Sender {
    self.sender
}

public(package) fun requests(self: &Intent): vector<Request> {
    self.requests
}
