module forum::intent;

use forum::error;
use forum::event;
use forum::responses;
use forum::sender::{Self, Sender};
use forum::tripcode;
use std::ascii::{Self, String};
use sui::bcs;
use sui::ed25519;
use sui::hash;

public enum Request has copy, drop, store {
    Uid,
    Ip32(address),
    Tripcode,
    Geo,
}

public enum RequestV2 has copy, drop, store {
    Uid,
    Ip32(address),
    Tripcode,
    Geo,
    Captcha,
}

public(package) fun request_uid(): RequestV2 {
    RequestV2::Uid
}

public(package) fun request_ip32(domain: address): RequestV2 {
    RequestV2::Ip32(domain)
}

public(package) fun request_tripcode(): RequestV2 {
    RequestV2::Tripcode
}

public(package) fun request_geo(): RequestV2 {
    RequestV2::Geo
}

public(package) fun request_captcha(): RequestV2 {
    RequestV2::Captcha
}

#[allow(unused_field)]
public struct IntentObject has copy, drop, store {
    id: ID,
    mutable: bool,
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

public struct IntentV2 {
    module_name: String,
    function: String,
    nonce: u64,
    objects: vector<IntentObject>,
    requests: vector<RequestV2>,
    event: vector<u8>,
    sender: Sender,
}

public(package) fun into_event(self: IntentV2): vector<u8> {
    let IntentV2 { event, .. } = self;
    event
}

public(package) fun decode(
    data: vector<u8>,
    expected_module: String,
    expected_function: String,
    signature: vector<u8>,
    expected_requests: vector<RequestV2>,
    responses: vector<u8>,
    expected_ids: vector<ID>,
    allowed_events: vector<String>,
): IntentV2 {
    let mut intent_bcs = bcs::new(data);
    let mut intent = IntentV2 {
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
                0 => RequestV2::Uid,
                1 => RequestV2::Ip32(bcs.peel_address()),
                2 => RequestV2::Tripcode,
                3 => RequestV2::Geo,
                4 => RequestV2::Captcha,
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

    let mut uid = option::none();
    let mut ip32 = option::none();
    let mut tripcode = option::none();
    let mut geo = option::none();
    intent
        .requests
        .do_ref!(
            |req| match (req) {
                RequestV2::Uid => uid.fill(responses.peel_vec_u8()),
                RequestV2::Ip32(_) => ip32.fill(responses.peel_u256()),
                RequestV2::Tripcode => tripcode.fill(
                    tripcode::new(responses.peel_bool(), ascii::string(responses.peel_vec_u8())),
                ),
                RequestV2::Geo => geo.fill(responses.peel_u32()),
                RequestV2::Captcha => { let _ = responses.peel_u8(); },
            },
        );
    assert!(responses.into_remainder_bytes().is_empty(), error::intent_args_mismatch());

    let sender = intent.sender();
    intent.event = bcs::to_bytes(&event::version());
    intent.event.append(bcs::to_bytes(&responses::new(uid, ip32, tripcode, geo)));
    intent.event.append(bcs::to_bytes(&sender));
    intent.event.append(bcs::to_bytes(&event_tag));
    intent.event.append(event.into_remainder_bytes());

    intent
}

public(package) fun nonce(self: &IntentV2): u64 {
    self.nonce
}

public(package) fun sender(self: &IntentV2): Sender {
    self.sender
}

public(package) fun requests(self: &IntentV2): vector<RequestV2> {
    self.requests
}
