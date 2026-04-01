//! Rate Limiter extension e2e tests for OFT-STD.
//!
//! Tests verify that the RateLimiter extension properly enforces rate limits
//! on cross-chain transfers. Uses real EndpointV2 and SimpleMessageLib.

use crate::integration_tests::{
    setup::{create_recipient_address, decode_packet, setup, wire_endpoint, wire_oft, TestSetup},
    utils::{
        address_to_peer_bytes32, advance_time, create_send_param, globally_disable_rate_limiter,
        inbound_rate_limit_capacity, lz_receive, mint_oft_token_to, mint_to, outbound_rate_limit_capacity,
        outbound_rate_limit_usage, quote_oft, quote_send, scan_packet_sent_event, send,
        set_bidirectional_net_rate_limit, set_inbound_rate_limit, set_outbound_rate_limit,
        set_rate_limit_config, set_rate_limit_exemption, try_lz_receive, try_send, validate_packet,
    },
};
use crate::extensions::rate_limiter::RateLimitConfig;
use soroban_sdk::{testutils::Address as _, token::TokenClient, Address};

/// Test e2e send with rate limiter globally disabled (unlimited)
#[test]
fn test_send_without_rate_limit() {
    let TestSetup { env, chain_a, chain_b } = setup();
    wire_endpoint(&env, &[&chain_a, &chain_b]);
    wire_oft(&env, &[&chain_a, &chain_b]);
    globally_disable_rate_limiter(&env, &chain_a);
    globally_disable_rate_limiter(&env, &chain_b);

    let sender = Address::generate(&env);
    let receiver = create_recipient_address(&env);
    let executor = Address::generate(&env);

    mint_oft_token_to(&env, &chain_a, &sender, 100_000_000);
    mint_to(&env, &chain_a.owner, &chain_a.native_token, &sender, 10_000_000_000);

    let capacity = outbound_rate_limit_capacity(&env, &chain_a, chain_b.eid);
    assert_eq!(capacity, i128::MAX);

    let to = address_to_peer_bytes32(&receiver);
    let send_param = create_send_param(&env, chain_b.eid, 50_000_000, 0, &to);
    let (_, _, oft_receipt) = quote_oft(&chain_a, &sender, &send_param);
    let fee = quote_send(&env, &chain_a, &sender, &send_param, false);

    send(&env, &chain_a, &sender, &send_param, &fee, &sender, &oft_receipt);

    let packet_event = scan_packet_sent_event(&env, &chain_a.endpoint.address).unwrap();
    validate_packet(&env, &chain_b, &packet_event);
    let packet = decode_packet(&env, &packet_event.0);

    lz_receive(&env, &chain_b, &executor, &packet, &receiver, 0);

    let receiver_balance = TokenClient::new(&env, &chain_b.oft_token).balance(&receiver);
    assert_eq!(receiver_balance, oft_receipt.amount_received_ld);
}

/// Test e2e send within rate limit succeeds
#[test]
fn test_send_within_rate_limit() {
    let TestSetup { env, chain_a, chain_b } = setup();
    wire_endpoint(&env, &[&chain_a, &chain_b]);
    wire_oft(&env, &[&chain_a, &chain_b]);
    globally_disable_rate_limiter(&env, &chain_b);

    let sender = Address::generate(&env);
    let receiver = create_recipient_address(&env);
    let executor = Address::generate(&env);

    mint_oft_token_to(&env, &chain_a, &sender, 100_000_000);
    mint_to(&env, &chain_a.owner, &chain_a.native_token, &sender, 10_000_000_000);

    set_outbound_rate_limit(&env, &chain_a, chain_b.eid, 10_000_000, 3600);

    let capacity = outbound_rate_limit_capacity(&env, &chain_a, chain_b.eid);
    assert_eq!(capacity, 10_000_000);

    let to = address_to_peer_bytes32(&receiver);
    let send_param = create_send_param(&env, chain_b.eid, 5_000_000, 0, &to);
    let (_, _, oft_receipt) = quote_oft(&chain_a, &sender, &send_param);
    let fee = quote_send(&env, &chain_a, &sender, &send_param, false);

    let in_flight_before = outbound_rate_limit_usage(&env, &chain_a, chain_b.eid);
    assert_eq!(in_flight_before, 0);

    send(&env, &chain_a, &sender, &send_param, &fee, &sender, &oft_receipt);

    let packet_event =
        scan_packet_sent_event(&env, &chain_a.endpoint.address).expect("packet_sent event should be emitted");

    let in_flight = outbound_rate_limit_usage(&env, &chain_a, chain_b.eid);
    assert_eq!(in_flight, oft_receipt.amount_received_ld);
    validate_packet(&env, &chain_b, &packet_event);
    let packet = decode_packet(&env, &packet_event.0);

    lz_receive(&env, &chain_b, &executor, &packet, &receiver, 0);

    let receiver_balance = TokenClient::new(&env, &chain_b.oft_token).balance(&receiver);
    assert_eq!(receiver_balance, oft_receipt.amount_received_ld);
}

/// Test e2e send exceeding rate limit fails
#[test]
fn test_send_exceeds_rate_limit() {
    let TestSetup { env, chain_a, chain_b } = setup();
    wire_endpoint(&env, &[&chain_a, &chain_b]);
    wire_oft(&env, &[&chain_a, &chain_b]);

    let sender = Address::generate(&env);
    let receiver = create_recipient_address(&env);

    mint_oft_token_to(&env, &chain_a, &sender, 100_000_000);
    mint_to(&env, &chain_a.owner, &chain_a.native_token, &sender, 10_000_000_000);

    set_outbound_rate_limit(&env, &chain_a, chain_b.eid, 1_000_000, 3600);

    let to = address_to_peer_bytes32(&receiver);
    let send_param = create_send_param(&env, chain_b.eid, 5_000_000, 0, &to);
    let (_, _, oft_receipt) = quote_oft(&chain_a, &sender, &send_param);
    let fee = quote_send(&env, &chain_a, &sender, &send_param, false);

    let success = try_send(&env, &chain_a, &sender, &send_param, &fee, &sender, &oft_receipt);
    assert!(!success);
}

/// Test rate limit decay over time
#[test]
fn test_rate_limit_decay() {
    let TestSetup { env, chain_a, chain_b } = setup();
    wire_endpoint(&env, &[&chain_a, &chain_b]);
    wire_oft(&env, &[&chain_a, &chain_b]);
    globally_disable_rate_limiter(&env, &chain_b);

    let sender = Address::generate(&env);
    let receiver = create_recipient_address(&env);
    let executor = Address::generate(&env);

    mint_oft_token_to(&env, &chain_a, &sender, 100_000_000);
    mint_to(&env, &chain_a.owner, &chain_a.native_token, &sender, 10_000_000_000);

    set_outbound_rate_limit(&env, &chain_a, chain_b.eid, 10_000_000, 1000);

    let to = address_to_peer_bytes32(&receiver);

    let send_param1 = create_send_param(&env, chain_b.eid, 8_000_000, 0, &to);
    let (_, _, oft_receipt1) = quote_oft(&chain_a, &sender, &send_param1);
    let fee1 = quote_send(&env, &chain_a, &sender, &send_param1, false);
    send(&env, &chain_a, &sender, &send_param1, &fee1, &sender, &oft_receipt1);

    let packet_event1 = scan_packet_sent_event(&env, &chain_a.endpoint.address).unwrap();
    validate_packet(&env, &chain_b, &packet_event1);
    let packet1 = decode_packet(&env, &packet_event1.0);
    lz_receive(&env, &chain_b, &executor, &packet1, &receiver, 0);

    let capacity_before = outbound_rate_limit_capacity(&env, &chain_a, chain_b.eid);
    assert!(capacity_before < 3_000_000);

    advance_time(&env, 500);

    let capacity_after = outbound_rate_limit_capacity(&env, &chain_a, chain_b.eid);
    assert!(capacity_after > capacity_before);

    let send_param2 = create_send_param(&env, chain_b.eid, 4_000_000, 0, &to);
    let (_, _, oft_receipt2) = quote_oft(&chain_a, &sender, &send_param2);
    let fee2 = quote_send(&env, &chain_a, &sender, &send_param2, false);
    send(&env, &chain_a, &sender, &send_param2, &fee2, &sender, &oft_receipt2);

    let packet_event2 = scan_packet_sent_event(&env, &chain_a.endpoint.address).unwrap();
    validate_packet(&env, &chain_b, &packet_event2);
    let packet2 = decode_packet(&env, &packet_event2.0);
    lz_receive(&env, &chain_b, &executor, &packet2, &receiver, 0);
}

/// Test outbound-only mode: round-trip shows outbound capacity is NOT released on receive
#[test]
fn test_gross_mode_does_not_release() {
    let TestSetup { env, chain_a, chain_b } = setup();
    wire_endpoint(&env, &[&chain_a, &chain_b]);
    wire_oft(&env, &[&chain_a, &chain_b]);
    globally_disable_rate_limiter(&env, &chain_b);

    let sender_a = Address::generate(&env);
    let sender_b = Address::generate(&env);
    let receiver_b = create_recipient_address(&env);
    let receiver_a = create_recipient_address(&env);
    let executor = Address::generate(&env);

    mint_oft_token_to(&env, &chain_a, &sender_a, 100_000_000);
    mint_to(&env, &chain_a.owner, &chain_a.native_token, &sender_a, 10_000_000_000);
    mint_oft_token_to(&env, &chain_b, &sender_b, 100_000_000);
    mint_to(&env, &chain_b.owner, &chain_b.native_token, &sender_b, 10_000_000_000);

    set_outbound_rate_limit(&env, &chain_a, chain_b.eid, 10_000_000, 3600);

    let to_b = address_to_peer_bytes32(&receiver_b);
    let send_param_1 = create_send_param(&env, chain_b.eid, 8_000_000, 0, &to_b);
    let (_, _, oft_receipt_1) = quote_oft(&chain_a, &sender_a, &send_param_1);
    let fee_1 = quote_send(&env, &chain_a, &sender_a, &send_param_1, false);
    send(&env, &chain_a, &sender_a, &send_param_1, &fee_1, &sender_a, &oft_receipt_1);

    let packet_event_1 = scan_packet_sent_event(&env, &chain_a.endpoint.address).unwrap();
    validate_packet(&env, &chain_b, &packet_event_1);
    let packet_1 = decode_packet(&env, &packet_event_1.0);

    let in_flight_after_send = outbound_rate_limit_usage(&env, &chain_a, chain_b.eid);
    assert_eq!(in_flight_after_send, oft_receipt_1.amount_received_ld);
    assert_eq!(outbound_rate_limit_capacity(&env, &chain_a, chain_b.eid), 10_000_000 - in_flight_after_send);

    lz_receive(&env, &chain_b, &executor, &packet_1, &receiver_b, 0);

    let to_a = address_to_peer_bytes32(&receiver_a);
    let send_param_2 = create_send_param(&env, chain_a.eid, 6_000_000, 0, &to_a);
    let (_, _, oft_receipt_2) = quote_oft(&chain_b, &sender_b, &send_param_2);
    let fee_2 = quote_send(&env, &chain_b, &sender_b, &send_param_2, false);
    send(&env, &chain_b, &sender_b, &send_param_2, &fee_2, &sender_b, &oft_receipt_2);

    let packet_event_2 = scan_packet_sent_event(&env, &chain_b.endpoint.address).unwrap();
    validate_packet(&env, &chain_a, &packet_event_2);
    let packet_2 = decode_packet(&env, &packet_event_2.0);

    lz_receive(&env, &chain_a, &executor, &packet_2, &receiver_a, 0);

    let in_flight_after_return = outbound_rate_limit_usage(&env, &chain_a, chain_b.eid);
    assert_eq!(
        in_flight_after_return, in_flight_after_send,
        "outbound-only: in-flight should not decrease when receiving"
    );

    let capacity_after_return = outbound_rate_limit_capacity(&env, &chain_a, chain_b.eid);
    assert!(capacity_after_return < 3_000_000, "outbound-only: capacity should remain low");
}

/// Test net mode: round-trip shows net mode releases outbound capacity on receive
#[test]
fn test_net_mode_does_release() {
    let TestSetup { env, chain_a, chain_b } = setup();
    wire_endpoint(&env, &[&chain_a, &chain_b]);
    wire_oft(&env, &[&chain_a, &chain_b]);
    globally_disable_rate_limiter(&env, &chain_b);

    let sender_a = Address::generate(&env);
    let sender_b = Address::generate(&env);
    let receiver_b = create_recipient_address(&env);
    let receiver_a = create_recipient_address(&env);
    let executor = Address::generate(&env);

    mint_oft_token_to(&env, &chain_a, &sender_a, 100_000_000);
    mint_to(&env, &chain_a.owner, &chain_a.native_token, &sender_a, 10_000_000_000);
    mint_oft_token_to(&env, &chain_b, &sender_b, 100_000_000);
    mint_to(&env, &chain_b.owner, &chain_b.native_token, &sender_b, 10_000_000_000);

    set_bidirectional_net_rate_limit(&env, &chain_a, chain_b.eid, 10_000_000, 3600);

    let to_b = address_to_peer_bytes32(&receiver_b);
    let send_param_1 = create_send_param(&env, chain_b.eid, 8_000_000, 0, &to_b);
    let (_, _, oft_receipt_1) = quote_oft(&chain_a, &sender_a, &send_param_1);
    let fee_1 = quote_send(&env, &chain_a, &sender_a, &send_param_1, false);
    send(&env, &chain_a, &sender_a, &send_param_1, &fee_1, &sender_a, &oft_receipt_1);

    let packet_event_1 = scan_packet_sent_event(&env, &chain_a.endpoint.address).unwrap();
    validate_packet(&env, &chain_b, &packet_event_1);
    let packet_1 = decode_packet(&env, &packet_event_1.0);

    let in_flight_after_send = outbound_rate_limit_usage(&env, &chain_a, chain_b.eid);
    assert_eq!(in_flight_after_send, oft_receipt_1.amount_received_ld);
    assert_eq!(outbound_rate_limit_capacity(&env, &chain_a, chain_b.eid), 10_000_000 - in_flight_after_send);

    lz_receive(&env, &chain_b, &executor, &packet_1, &receiver_b, 0);

    let to_a = address_to_peer_bytes32(&receiver_a);
    let send_param_2 = create_send_param(&env, chain_a.eid, 6_000_000, 0, &to_a);
    let (_, _, oft_receipt_2) = quote_oft(&chain_b, &sender_b, &send_param_2);
    let fee_2 = quote_send(&env, &chain_b, &sender_b, &send_param_2, false);
    send(&env, &chain_b, &sender_b, &send_param_2, &fee_2, &sender_b, &oft_receipt_2);

    let packet_event_2 = scan_packet_sent_event(&env, &chain_b.endpoint.address).unwrap();
    validate_packet(&env, &chain_a, &packet_event_2);
    let packet_2 = decode_packet(&env, &packet_event_2.0);

    lz_receive(&env, &chain_a, &executor, &packet_2, &receiver_a, 0);

    let in_flight_after_return = outbound_rate_limit_usage(&env, &chain_a, chain_b.eid);
    assert!(
        in_flight_after_return < in_flight_after_send,
        "net mode: outbound in-flight should decrease when receiving"
    );

    let capacity_after_return = outbound_rate_limit_capacity(&env, &chain_a, chain_b.eid);
    assert!(capacity_after_return > 7_000_000, "net mode: capacity should increase after receiving");
}

/// Test inbound rate limit blocks lz_receive when exceeded
#[test]
fn test_inbound_rate_limit_blocks_receive() {
    let TestSetup { env, chain_a, chain_b } = setup();
    wire_endpoint(&env, &[&chain_a, &chain_b]);
    wire_oft(&env, &[&chain_a, &chain_b]);
    globally_disable_rate_limiter(&env, &chain_a);

    let sender = Address::generate(&env);
    let receiver = create_recipient_address(&env);
    let executor = Address::generate(&env);

    mint_oft_token_to(&env, &chain_a, &sender, 100_000_000);
    mint_to(&env, &chain_a.owner, &chain_a.native_token, &sender, 10_000_000_000);

    // Set inbound rate limit on chain_b: only 1M allowed from chain_a
    set_inbound_rate_limit(&env, &chain_b, chain_a.eid, 1_000_000, 3600);
    assert_eq!(inbound_rate_limit_capacity(&env, &chain_b, chain_a.eid), 1_000_000);

    // Send 5M from chain_a (outbound unlimited)
    let to = address_to_peer_bytes32(&receiver);
    let send_param = create_send_param(&env, chain_b.eid, 5_000_000, 0, &to);
    let (_, _, oft_receipt) = quote_oft(&chain_a, &sender, &send_param);
    let fee = quote_send(&env, &chain_a, &sender, &send_param, false);
    send(&env, &chain_a, &sender, &send_param, &fee, &sender, &oft_receipt);

    let packet_event = scan_packet_sent_event(&env, &chain_a.endpoint.address).unwrap();
    validate_packet(&env, &chain_b, &packet_event);
    let packet = decode_packet(&env, &packet_event.0);

    // Receive on chain_b should fail — inbound capacity (1M) < transfer amount (5M)
    let success = try_lz_receive(&env, &chain_b, &executor, &packet, &receiver, 0);
    assert!(!success, "receive should be blocked by inbound rate limit");
}

/// Test that an exempt sender bypasses outbound rate limit on send.
///
/// Verifies the full e2e path: quote_oft reports limited capacity (matching EVM behavior
/// where quoteOFT does not account for per-address exemptions), but send succeeds for an
/// amount exceeding the rate limit, and the exempt sender's transfer does NOT consume
/// outbound usage. Non-exempt senders remain limited.
#[test]
fn test_exempt_sender_bypasses_outbound_rate_limit() {
    let TestSetup { env, chain_a, chain_b } = setup();
    wire_endpoint(&env, &[&chain_a, &chain_b]);
    wire_oft(&env, &[&chain_a, &chain_b]);
    globally_disable_rate_limiter(&env, &chain_b);

    let exempt_sender = Address::generate(&env);
    let normal_sender = Address::generate(&env);
    let receiver = create_recipient_address(&env);
    let executor = Address::generate(&env);

    mint_oft_token_to(&env, &chain_a, &exempt_sender, 100_000_000);
    mint_oft_token_to(&env, &chain_a, &normal_sender, 100_000_000);
    mint_to(&env, &chain_a.owner, &chain_a.native_token, &exempt_sender, 10_000_000_000);
    mint_to(&env, &chain_a.owner, &chain_a.native_token, &normal_sender, 10_000_000_000);

    // Set outbound rate limit with address exemption enabled
    set_rate_limit_config(
        &env,
        &chain_a,
        chain_b.eid,
        RateLimitConfig {
            outbound_enabled: true,
            inbound_enabled: false,
            net_accounting_enabled: false,
            address_exemption_enabled: true,
            outbound_limit: 1_000_000,
            inbound_limit: 0,
            outbound_window: 3600,
            inbound_window: 0,
        },
    );

    // Mark exempt_sender as exempt
    set_rate_limit_exemption(&env, &chain_a, &exempt_sender, true);

    let to = address_to_peer_bytes32(&receiver);

    let send_param = create_send_param(&env, chain_b.eid, 5_000_000, 0, &to);
    let (oft_limit, _, oft_receipt) = quote_oft(&chain_a, &exempt_sender, &send_param);
    assert_eq!(oft_limit.max_amount_ld, 1_000_000, "quote_oft should report rate-limited capacity");

    // Exempt sender can send 5M despite the 1M rate limit
    let fee = quote_send(&env, &chain_a, &exempt_sender, &send_param, false);
    send(&env, &chain_a, &exempt_sender, &send_param, &fee, &exempt_sender, &oft_receipt);

    // Scan packet event immediately after send (events are cleared by subsequent contract calls)
    let packet_event = scan_packet_sent_event(&env, &chain_a.endpoint.address).unwrap();

    // Outbound usage should remain 0 — exempt sender does not consume capacity
    let usage_after = outbound_rate_limit_usage(&env, &chain_a, chain_b.eid);
    assert_eq!(usage_after, 0, "exempt sender should not consume outbound usage");

    // Full capacity is still available for non-exempt users
    let capacity_after = outbound_rate_limit_capacity(&env, &chain_a, chain_b.eid);
    assert_eq!(capacity_after, 1_000_000, "capacity should be unchanged after exempt send");

    // Verify the transfer went through end-to-end
    validate_packet(&env, &chain_b, &packet_event);
    let packet = decode_packet(&env, &packet_event.0);
    lz_receive(&env, &chain_b, &executor, &packet, &receiver, 0);

    let receiver_balance = TokenClient::new(&env, &chain_b.oft_token).balance(&receiver);
    assert_eq!(receiver_balance, oft_receipt.amount_received_ld);

    // Non-exempt sender is still blocked when exceeding the rate limit
    let send_param_normal = create_send_param(&env, chain_b.eid, 5_000_000, 0, &to);
    let (_, _, oft_receipt_normal) = quote_oft(&chain_a, &normal_sender, &send_param_normal);
    let fee_normal = quote_send(&env, &chain_a, &normal_sender, &send_param_normal, false);
    let success = try_send(&env, &chain_a, &normal_sender, &send_param_normal, &fee_normal, &normal_sender, &oft_receipt_normal);
    assert!(!success, "non-exempt sender should still be blocked by rate limit");
}

/// Test that an exempt receiver bypasses inbound rate limit on lz_receive.
///
/// Verifies that when `address_exemption_enabled` is true and the receiver is exempt,
/// `lz_receive` succeeds for an amount exceeding the inbound rate limit, and the
/// exempt receiver's transfer does NOT consume inbound usage. A non-exempt receiver
/// is still blocked.
#[test]
fn test_exempt_receiver_bypasses_inbound_rate_limit() {
    let TestSetup { env, chain_a, chain_b } = setup();
    wire_endpoint(&env, &[&chain_a, &chain_b]);
    wire_oft(&env, &[&chain_a, &chain_b]);
    globally_disable_rate_limiter(&env, &chain_a);

    let sender = Address::generate(&env);
    let exempt_receiver = create_recipient_address(&env);
    let normal_receiver = create_recipient_address(&env);
    let executor = Address::generate(&env);

    mint_oft_token_to(&env, &chain_a, &sender, 100_000_000);
    mint_to(&env, &chain_a.owner, &chain_a.native_token, &sender, 10_000_000_000);

    // Set inbound rate limit on chain_b with address exemption enabled
    set_rate_limit_config(
        &env,
        &chain_b,
        chain_a.eid,
        RateLimitConfig {
            outbound_enabled: false,
            inbound_enabled: true,
            net_accounting_enabled: false,
            address_exemption_enabled: true,
            outbound_limit: 0,
            inbound_limit: 1_000_000,
            outbound_window: 0,
            inbound_window: 3600,
        },
    );

    // Mark exempt_receiver as exempt on chain_b
    set_rate_limit_exemption(&env, &chain_b, &exempt_receiver, true);

    // Send 5M from chain_a (outbound unlimited on chain_a)
    let to_exempt = address_to_peer_bytes32(&exempt_receiver);
    let send_param = create_send_param(&env, chain_b.eid, 5_000_000, 0, &to_exempt);
    let (_, _, oft_receipt) = quote_oft(&chain_a, &sender, &send_param);
    let fee = quote_send(&env, &chain_a, &sender, &send_param, false);
    send(&env, &chain_a, &sender, &send_param, &fee, &sender, &oft_receipt);

    let packet_event = scan_packet_sent_event(&env, &chain_a.endpoint.address).unwrap();
    validate_packet(&env, &chain_b, &packet_event);
    let packet = decode_packet(&env, &packet_event.0);

    // Exempt receiver can receive 5M despite the 1M inbound rate limit
    lz_receive(&env, &chain_b, &executor, &packet, &exempt_receiver, 0);

    let exempt_balance = TokenClient::new(&env, &chain_b.oft_token).balance(&exempt_receiver);
    assert_eq!(exempt_balance, oft_receipt.amount_received_ld);

    // Inbound usage should remain 0 — exempt receiver does not consume capacity
    let inbound_usage = inbound_rate_limit_capacity(&env, &chain_b, chain_a.eid);
    assert_eq!(inbound_usage, 1_000_000, "inbound capacity should be unchanged after exempt receive");

    // Non-exempt receiver is still blocked when exceeding the inbound rate limit
    let to_normal = address_to_peer_bytes32(&normal_receiver);
    let send_param2 = create_send_param(&env, chain_b.eid, 5_000_000, 0, &to_normal);
    let (_, _, oft_receipt2) = quote_oft(&chain_a, &sender, &send_param2);
    let fee2 = quote_send(&env, &chain_a, &sender, &send_param2, false);
    send(&env, &chain_a, &sender, &send_param2, &fee2, &sender, &oft_receipt2);

    let packet_event2 = scan_packet_sent_event(&env, &chain_a.endpoint.address).unwrap();
    validate_packet(&env, &chain_b, &packet_event2);
    let packet2 = decode_packet(&env, &packet_event2.0);

    let success = try_lz_receive(&env, &chain_b, &executor, &packet2, &normal_receiver, 0);
    assert!(!success, "non-exempt receiver should still be blocked by inbound rate limit");
}
