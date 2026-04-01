//! Combined extension e2e tests for OFT-STD.
//!
//! Tests verify the interaction between multiple extensions when enabled simultaneously.

use crate::integration_tests::{
    setup::{create_recipient_address, decode_packet, setup, wire_endpoint, wire_oft, TestSetup},
    utils::{
        address_to_peer_bytes32, create_send_param, globally_disable_rate_limiter, lz_receive, mint_oft_token_to,
        mint_to, outbound_rate_limit_usage, quote_oft, quote_send, scan_packet_sent_event, send_with_fee,
        set_default_fee_bps, set_outbound_rate_limit, set_paused, try_send, validate_packet,
    },
};
use soroban_sdk::{testutils::Address as _, token::TokenClient, Address};

/// Test fee + rate limit: rate limit consumes the post-fee amount (amount_received_ld)
#[test]
fn test_fee_plus_rate_limit() {
    let TestSetup { env, chain_a, chain_b } = setup();
    wire_endpoint(&env, &[&chain_a, &chain_b]);
    wire_oft(&env, &[&chain_a, &chain_b]);
    globally_disable_rate_limiter(&env, &chain_b);

    let sender = Address::generate(&env);
    let receiver = create_recipient_address(&env);
    let executor = Address::generate(&env);

    mint_oft_token_to(&env, &chain_a, &sender, 100_000_000);
    mint_to(&env, &chain_a.owner, &chain_a.native_token, &sender, 10_000_000_000);

    // Enable 10% fee and outbound rate limit of 5M.
    set_default_fee_bps(&env, &chain_a, 1_000); // 10%
    set_outbound_rate_limit(&env, &chain_a, chain_b.eid, 5_000_000, 3600);

    let to = address_to_peer_bytes32(&receiver);
    let amount_ld = 2_000_000i128;
    let send_param = create_send_param(&env, chain_b.eid, amount_ld, 0, &to);
    let (_, _, oft_receipt) = quote_oft(&chain_a, &sender, &send_param);

    // Fee = 10% of 2M = 200,000; amount_received = 1,800,000
    let expected_fee = 200_000i128;
    assert_eq!(oft_receipt.amount_sent_ld, amount_ld);
    assert_eq!(oft_receipt.amount_received_ld, amount_ld - expected_fee);

    let fee = quote_send(&env, &chain_a, &sender, &send_param, false);
    send_with_fee(&env, &chain_a, &sender, &send_param, &fee, &sender, &oft_receipt, &chain_a.fee_collector);

    // Scan events immediately after send
    let packet_event = scan_packet_sent_event(&env, &chain_a.endpoint.address).unwrap();
    validate_packet(&env, &chain_b, &packet_event);
    let packet = decode_packet(&env, &packet_event.0);

    // Rate limit usage = post-fee amount (amount_received_ld), not the gross amount
    let usage = outbound_rate_limit_usage(&env, &chain_a, chain_b.eid);
    assert_eq!(usage, oft_receipt.amount_received_ld);

    // Fee was collected on source chain
    let fee_collector_balance = TokenClient::new(&env, &chain_a.oft_token).balance(&chain_a.fee_collector);
    assert_eq!(fee_collector_balance, expected_fee);

    lz_receive(&env, &chain_b, &executor, &packet, &receiver, 0);

    let receiver_balance = TokenClient::new(&env, &chain_b.oft_token).balance(&receiver);
    assert_eq!(receiver_balance, oft_receipt.amount_received_ld);
}

/// Test pause + rate limit: paused overrides rate limit (send fails even if within rate limit)
#[test]
fn test_pause_overrides_rate_limit() {
    let TestSetup { env, chain_a, chain_b } = setup();
    wire_endpoint(&env, &[&chain_a, &chain_b]);
    wire_oft(&env, &[&chain_a, &chain_b]);

    let sender = Address::generate(&env);
    let receiver = create_recipient_address(&env);

    mint_oft_token_to(&env, &chain_a, &sender, 100_000_000);
    mint_to(&env, &chain_a.owner, &chain_a.native_token, &sender, 10_000_000_000);

    set_outbound_rate_limit(&env, &chain_a, chain_b.eid, 10_000_000, 3600);

    let to = address_to_peer_bytes32(&receiver);
    let send_param = create_send_param(&env, chain_b.eid, 1_000_000, 0, &to);
    let (_, _, oft_receipt) = quote_oft(&chain_a, &sender, &send_param);
    let fee = quote_send(&env, &chain_a, &sender, &send_param, false);

    // Pause — send should fail even though rate limit has plenty of capacity
    set_paused(&env, &chain_a, true);
    let success = try_send(&env, &chain_a, &sender, &send_param, &fee, &sender, &oft_receipt);
    assert!(!success, "send should fail when paused, regardless of rate limit capacity");
}
