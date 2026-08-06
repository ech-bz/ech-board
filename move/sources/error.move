module forum::error;

public(package) fun intent_signature_invalid(): u64 { 1 }

public(package) fun intent_target_mismatch(): u64 { 2 }

public(package) fun intent_object_mismatch(): u64 { 3 }

public(package) fun intent_args_mismatch(): u64 { 4 }

public(package) fun intent_relay_signature_invalid(): u64 { 5 }

public(package) fun sharded_counter_index_mismatch(): u64 { 6 }

public(package) fun sharded_counter_value_mismatch(): u64 { 7 }

public(package) fun board_slug_invalid(): u64 { 8 }

public(package) fun media_limit_exceeded(): u64 { 9 }

public(package) fun post_requires_media(): u64 { 10 }

public(package) fun post_empty(): u64 { 11 }

public(package) fun board_closed(): u64 { 12 }

public(package) fun thread_closed(): u64 { 13 }

public(package) fun not_authorized(): u64 { 14 }

public(package) fun cross_reference_mismatch(): u64 { 15 }

public(package) fun reaction_not_allowed(): u64 { 16 }

public(package) fun already_voted(): u64 { 17 }

public(package) fun event_version_unsupported(): u64 { 18 }

public(package) fun entity_version_unsupported(): u64 { 19 }

public(package) fun vote_options_mismatch(): u64 { 20 }

public(package) fun vote_options_limit(): u64 { 21 }
