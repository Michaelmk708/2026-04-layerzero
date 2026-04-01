extern crate std;

use crate as oft;
use crate::extensions::rate_limiter::{
    RateLimitConfig, RateLimitError, RateLimitGlobalConfig, RateLimitState, RateLimiter, RateLimiterInternal,
    RATE_LIMITER_MANAGER_ROLE, UNLIMITED_AMOUNT,
};
use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Ledger as _},
    Address, Env, Symbol,
};
use utils::auth::Auth;
use utils::rbac::{grant_role_no_auth, RoleBasedAccessControl};

// ============================================================================
// Test Contract
// ============================================================================

#[contract]
struct RateLimiterTestContract;

impl Auth for RateLimiterTestContract {
    fn authorizer(env: &Env) -> Option<Address> {
        Some(env.current_contract_address())
    }
}

impl RateLimiterInternal for RateLimiterTestContract {}

#[contractimpl(contracttrait)]
impl RateLimiter for RateLimiterTestContract {}

#[contractimpl(contracttrait)]
impl RoleBasedAccessControl for RateLimiterTestContract {}

#[contractimpl]
impl RateLimiterTestContract {
    pub fn init_roles(env: Env) {
        let contract_id = env.current_contract_address();
        grant_role_no_auth(&env, &contract_id, &Symbol::new(&env, RATE_LIMITER_MANAGER_ROLE), &contract_id);
    }

    pub fn outflow(env: Env, id: u128, from: Address, amount: i128) {
        <Self as RateLimiterInternal>::__outflow(&env, id, &from, amount);
    }

    pub fn inflow(env: Env, id: u128, to: Address, amount: i128) {
        <Self as RateLimiterInternal>::__inflow(&env, id, &to, amount);
    }

    pub fn get_state_and_config(env: Env, id: u128) -> (u128, RateLimitState, RateLimitConfig) {
        <Self as RateLimiterInternal>::__get_rate_limit_state_and_config(&env, id)
    }
}

// ============================================================================
// Test Setup
// ============================================================================

struct TestSetup {
    env: Env,
    client: RateLimiterTestContractClient<'static>,
    contract_id: Address,
}

fn setup() -> TestSetup {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let contract_id = env.register(RateLimiterTestContract, ());
    let client = RateLimiterTestContractClient::new(&env, &contract_id);
    client.init_roles();
    TestSetup { env, client, contract_id }
}

fn id(v: u32) -> u128 {
    v as u128
}

fn outbound_config(limit: i128, window: u64) -> RateLimitConfig {
    RateLimitConfig {
        outbound_enabled: true,
        inbound_enabled: false,
        net_accounting_enabled: false,
        address_exemption_enabled: false,
        outbound_limit: limit,
        inbound_limit: 0,
        outbound_window: window,
        inbound_window: 0,
    }
}

fn inbound_config(limit: i128, window: u64) -> RateLimitConfig {
    RateLimitConfig {
        outbound_enabled: false,
        inbound_enabled: true,
        net_accounting_enabled: false,
        address_exemption_enabled: false,
        outbound_limit: 0,
        inbound_limit: limit,
        outbound_window: 0,
        inbound_window: window,
    }
}

fn bidirectional_net_config(limit: i128, window: u64) -> RateLimitConfig {
    RateLimitConfig {
        outbound_enabled: true,
        inbound_enabled: true,
        net_accounting_enabled: true,
        address_exemption_enabled: false,
        outbound_limit: limit,
        inbound_limit: limit,
        outbound_window: window,
        inbound_window: window,
    }
}

fn set_config(client: &RateLimiterTestContractClient, contract_id: &Address, eid: u32, config: RateLimitConfig) {
    client.set_rate_limit_config(&id(eid), &Some(config), contract_id);
}

// ============================================================================
// Default State Tests (closed by default — limits=0 blocks all transfers)
// ============================================================================

#[test]
fn test_default_state_blocks_outflow() {
    let TestSetup { env, client, .. } = setup();
    let user = Address::generate(&env);

    let usages = client.get_rate_limit_usages(&id(42));
    assert_eq!(usages.outbound_available_amount, 0);
    assert_eq!(usages.inbound_available_amount, 0);

    let res = client.try_outflow(&id(42), &user, &1i128);
    assert_eq!(res.err().unwrap().ok().unwrap(), RateLimitError::RateLimitExceeded.into());
}

#[test]
fn test_default_state_blocks_inflow() {
    let TestSetup { env, client, .. } = setup();
    let user = Address::generate(&env);

    let res = client.try_inflow(&id(42), &user, &1i128);
    assert_eq!(res.err().unwrap().ok().unwrap(), RateLimitError::RateLimitExceeded.into());
}

// ============================================================================
// Global Config Tests
// ============================================================================

#[test]
fn test_set_and_get_global_config() {
    let TestSetup { client, contract_id, .. } = setup();

    let gc = RateLimitGlobalConfig { use_global_state: true, is_globally_disabled: false };
    client.set_rate_limit_global_config(&Some(gc.clone()), &contract_id);
    assert_eq!(client.get_rate_limit_global_config(), gc);

    client.set_rate_limit_global_config(&None, &contract_id);
    assert_eq!(client.get_rate_limit_global_config(), RateLimitGlobalConfig::default());
}

#[test]
fn test_globally_disabled_bypasses_rate_limit() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);

    let res = client.try_outflow(&id(1), &user, &1i128);
    assert!(res.is_err());

    client.set_rate_limit_global_config(
        &Some(RateLimitGlobalConfig { use_global_state: false, is_globally_disabled: true }),
        &contract_id,
    );

    client.outflow(&id(1), &user, &999i128);

    let usages = client.get_rate_limit_usages(&id(1));
    assert_eq!(usages.outbound_available_amount, UNLIMITED_AMOUNT);
    assert_eq!(usages.inbound_available_amount, UNLIMITED_AMOUNT);
}

#[test]
fn test_disabled_direction_returns_unlimited() {
    let TestSetup { client, contract_id, .. } = setup();

    // Outbound-only config: inbound disabled → inbound available = UNLIMITED
    set_config(&client, &contract_id, 1, outbound_config(100, 10));
    let usages = client.get_rate_limit_usages(&id(1));
    assert_eq!(usages.outbound_available_amount, 100);
    assert_eq!(usages.inbound_available_amount, UNLIMITED_AMOUNT);

    // Inbound-only config: outbound disabled → outbound available = UNLIMITED
    let inbound_only = RateLimitConfig {
        outbound_enabled: false,
        inbound_enabled: true,
        net_accounting_enabled: false,
        address_exemption_enabled: false,
        outbound_limit: 0,
        inbound_limit: 200,
        outbound_window: 0,
        inbound_window: 10,
    };
    set_config(&client, &contract_id, 2, inbound_only);
    let usages = client.get_rate_limit_usages(&id(2));
    assert_eq!(usages.outbound_available_amount, UNLIMITED_AMOUNT);
    assert_eq!(usages.inbound_available_amount, 200);
}

// ============================================================================
// Set Configs Tests
// ============================================================================

#[test]
fn test_set_configs_basic() {
    let TestSetup { client, contract_id, .. } = setup();
    let eid = 77u32;

    set_config(&client, &contract_id, eid, outbound_config(100, 10));

    let (_, config) = client.rate_limits(&id(eid));
    let config = config.unwrap();
    assert_eq!(config.outbound_limit, 100);
    assert_eq!(config.outbound_window, 10);
    assert!(config.outbound_enabled);
}

#[test]
fn test_set_configs_rejects_negative_outbound_limit() {
    let TestSetup { client, contract_id, .. } = setup();

    let res = client.try_set_rate_limit_config(&id(1), &Some(outbound_config(-1, 10)), &contract_id);
    assert_eq!(res.err().unwrap().ok().unwrap(), RateLimitError::OutboundLimitNegative.into());
}

#[test]
fn test_set_configs_rejects_negative_inbound_limit() {
    let TestSetup { client, contract_id, .. } = setup();

    let config = RateLimitConfig {
        outbound_enabled: false,
        inbound_enabled: true,
        net_accounting_enabled: false,
        address_exemption_enabled: false,
        outbound_limit: 0,
        inbound_limit: -1,
        outbound_window: 0,
        inbound_window: 10,
    };
    let res = client.try_set_rate_limit_config(&id(1), &Some(config), &contract_id);
    assert_eq!(res.err().unwrap().ok().unwrap(), RateLimitError::InboundLimitNegative.into());
}

// ============================================================================
// Outflow / Inflow Tests
// ============================================================================

#[test]
fn test_outflow_within_capacity() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);
    let eid = 77u32;

    set_config(&client, &contract_id, eid, outbound_config(100, 100));

    client.outflow(&id(eid), &user, &60i128);
    let usages = client.get_rate_limit_usages(&id(eid));
    assert_eq!(usages.outbound_usage, 60);
    assert_eq!(usages.outbound_available_amount, 40);
}

#[test]
fn test_outflow_exceeds_capacity() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);
    let eid = 77u32;

    set_config(&client, &contract_id, eid, outbound_config(100, 100));
    client.outflow(&id(eid), &user, &60i128);

    let res = client.try_outflow(&id(eid), &user, &41i128);
    assert_eq!(res.err().unwrap().ok().unwrap(), RateLimitError::RateLimitExceeded.into());
}

#[test]
fn test_outflow_exact_capacity() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);
    let eid = 1u32;

    set_config(&client, &contract_id, eid, outbound_config(100, 100));

    client.outflow(&id(eid), &user, &100i128);
    let usages = client.get_rate_limit_usages(&id(eid));
    assert_eq!(usages.outbound_usage, 100);
    assert_eq!(usages.outbound_available_amount, 0);
}

#[test]
fn test_outflow_zero_amount_succeeds() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);
    let eid = 1u32;

    set_config(&client, &contract_id, eid, outbound_config(100, 100));

    client.outflow(&id(eid), &user, &0i128);
    let usages = client.get_rate_limit_usages(&id(eid));
    assert_eq!(usages.outbound_usage, 0);
}

#[test]
fn test_different_ids_are_independent() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);

    set_config(&client, &contract_id, 1, outbound_config(100, 10));
    set_config(&client, &contract_id, 2, outbound_config(200, 20));

    client.outflow(&id(1), &user, &50i128);
    let usages1 = client.get_rate_limit_usages(&id(1));
    let usages2 = client.get_rate_limit_usages(&id(2));
    assert_eq!(usages1.outbound_usage, 50);
    assert_eq!(usages2.outbound_usage, 0);
}

#[test]
fn test_inflow_within_capacity() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);
    let eid = 77u32;

    set_config(&client, &contract_id, eid, inbound_config(100, 100));

    client.inflow(&id(eid), &user, &60i128);
    let usages = client.get_rate_limit_usages(&id(eid));
    assert_eq!(usages.inbound_usage, 60);
    assert_eq!(usages.inbound_available_amount, 40);
}

#[test]
fn test_inflow_exceeds_capacity() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);
    let eid = 77u32;

    set_config(&client, &contract_id, eid, inbound_config(100, 100));
    client.inflow(&id(eid), &user, &60i128);

    let res = client.try_inflow(&id(eid), &user, &41i128);
    assert_eq!(res.err().unwrap().ok().unwrap(), RateLimitError::RateLimitExceeded.into());
}

#[test]
fn test_negative_amount_rejected() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);

    set_config(&client, &contract_id, 1, outbound_config(100, 100));

    let res = client.try_outflow(&id(1), &user, &-1i128);
    assert_eq!(res.err().unwrap().ok().unwrap(), RateLimitError::InvalidAmount.into());

    set_config(&client, &contract_id, 2, inbound_config(100, 100));
    let res = client.try_inflow(&id(2), &user, &-1i128);
    assert_eq!(res.err().unwrap().ok().unwrap(), RateLimitError::InvalidAmount.into());
}

#[test]
fn test_limit_zero_blocks_all() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);

    set_config(&client, &contract_id, 1, outbound_config(0, 10));

    let usages = client.get_rate_limit_usages(&id(1));
    assert_eq!(usages.outbound_available_amount, 0);

    let res = client.try_outflow(&id(1), &user, &1i128);
    assert_eq!(res.err().unwrap().ok().unwrap(), RateLimitError::RateLimitExceeded.into());
}

// ============================================================================
// Net Accounting Tests
// ============================================================================

#[test]
fn test_net_accounting_inflow_releases_outbound() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);
    let eid = 77u32;

    set_config(&client, &contract_id, eid, bidirectional_net_config(100, 100));

    client.outflow(&id(eid), &user, &60i128);
    let usages = client.get_rate_limit_usages(&id(eid));
    assert_eq!(usages.outbound_usage, 60);
    assert_eq!(usages.outbound_available_amount, 40);

    client.inflow(&id(eid), &user, &30i128);
    let usages2 = client.get_rate_limit_usages(&id(eid));
    assert_eq!(usages2.outbound_usage, 30, "net accounting should release outbound");
    assert_eq!(usages2.outbound_available_amount, 70);
    assert_eq!(usages2.inbound_usage, 30);
}

#[test]
fn test_no_net_accounting_inflow_does_not_release() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);
    let eid = 77u32;

    let config = RateLimitConfig {
        outbound_enabled: true,
        inbound_enabled: true,
        net_accounting_enabled: false,
        address_exemption_enabled: false,
        outbound_limit: 100,
        inbound_limit: 100,
        outbound_window: 100,
        inbound_window: 100,
    };
    set_config(&client, &contract_id, eid, config);

    client.outflow(&id(eid), &user, &60i128);
    client.inflow(&id(eid), &user, &30i128);

    let usages = client.get_rate_limit_usages(&id(eid));
    assert_eq!(usages.outbound_usage, 60, "gross: outbound should not release");
    assert_eq!(usages.inbound_usage, 30);
}

// ============================================================================
// Decay Tests
// ============================================================================

#[test]
fn test_decay_reduces_usage_over_time() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);
    let eid = 5u32;

    env.ledger().set_timestamp(1_000);
    set_config(&client, &contract_id, eid, outbound_config(100, 10));

    client.outflow(&id(eid), &user, &100i128);
    assert_eq!(client.get_rate_limit_usages(&id(eid)).outbound_usage, 100);

    env.ledger().set_timestamp(1_005);
    let usages = client.get_rate_limit_usages(&id(eid));
    assert_eq!(usages.outbound_usage, 50);
    assert_eq!(usages.outbound_available_amount, 50);
}

#[test]
fn test_decay_fully_restores_capacity() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);
    let eid = 1u32;

    env.ledger().set_timestamp(1_000);
    set_config(&client, &contract_id, eid, outbound_config(100, 10));

    client.outflow(&id(eid), &user, &100i128);
    assert_eq!(client.get_rate_limit_usages(&id(eid)).outbound_available_amount, 0);

    env.ledger().set_timestamp(1_010);
    let usages = client.get_rate_limit_usages(&id(eid));
    assert_eq!(usages.outbound_usage, 0);
    assert_eq!(usages.outbound_available_amount, 100);
}

#[test]
fn test_decay_beyond_window_clamps_to_zero() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    set_config(&client, &contract_id, 1, outbound_config(100, 10));

    client.outflow(&id(1), &user, &50i128);

    env.ledger().set_timestamp(2_000);
    assert_eq!(client.get_rate_limit_usages(&id(1)).outbound_usage, 0);
}

#[test]
fn test_timestamp_regression_errors() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    set_config(&client, &contract_id, 5, outbound_config(100, 10));
    client.outflow(&id(5), &user, &100i128);

    env.ledger().set_timestamp(999);
    let res = client.try_get_rate_limit_usages(&id(5));
    assert_eq!(res.err().unwrap().ok().unwrap(), RateLimitError::LastUpdatedInFuture.into());
}

// ============================================================================
// Config Inheritance Tests
// ============================================================================

#[test]
fn test_config_inheritance_from_default() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);

    let default_config = RateLimitConfig {
        outbound_enabled: true,
        inbound_enabled: false,
        net_accounting_enabled: false,
        address_exemption_enabled: false,
        outbound_limit: 500,
        inbound_limit: 0,
        outbound_window: 100,
        inbound_window: 0,
    };
    client.set_rate_limit_config(&id(0), &Some(default_config), &contract_id);

    let usages = client.get_rate_limit_usages(&id(42));
    assert_eq!(usages.outbound_available_amount, 500);

    client.outflow(&id(42), &user, &100i128);
    let usages = client.get_rate_limit_usages(&id(42));
    assert_eq!(usages.outbound_usage, 100);
    assert_eq!(usages.outbound_available_amount, 400);
}

#[test]
fn test_per_id_override_uses_own_config() {
    let TestSetup { client, contract_id, .. } = setup();

    let default_config = RateLimitConfig {
        outbound_enabled: true,
        inbound_enabled: false,
        net_accounting_enabled: false,
        address_exemption_enabled: false,
        outbound_limit: 500,
        inbound_limit: 0,
        outbound_window: 100,
        inbound_window: 0,
    };
    client.set_rate_limit_config(&id(0), &Some(default_config), &contract_id);

    set_config(&client, &contract_id, 42, outbound_config(200, 50));

    let usages_42 = client.get_rate_limit_usages(&id(42));
    assert_eq!(usages_42.outbound_available_amount, 200);

    let usages_99 = client.get_rate_limit_usages(&id(99));
    assert_eq!(usages_99.outbound_available_amount, 500);
}

#[test]
fn test_global_state_shares_usage() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);

    client.set_rate_limit_global_config(
        &Some(RateLimitGlobalConfig { use_global_state: true, is_globally_disabled: false }),
        &contract_id,
    );

    let default_config = RateLimitConfig {
        outbound_enabled: true,
        inbound_enabled: false,
        net_accounting_enabled: false,
        address_exemption_enabled: false,
        outbound_limit: 100,
        inbound_limit: 0,
        outbound_window: 100,
        inbound_window: 0,
    };
    client.set_rate_limit_config(&id(0), &Some(default_config), &contract_id);

    client.outflow(&id(1), &user, &30i128);
    client.outflow(&id(2), &user, &40i128);

    let usages = client.get_rate_limit_usages(&id(99));
    assert_eq!(usages.outbound_usage, 70);
    assert_eq!(usages.outbound_available_amount, 30);
}

// ============================================================================
// Config Removal Tests
// ============================================================================

#[test]
fn test_remove_config_falls_back_to_default() {
    let TestSetup { client, contract_id, .. } = setup();

    let default_config = RateLimitConfig {
        outbound_enabled: true,
        inbound_enabled: false,
        net_accounting_enabled: false,
        address_exemption_enabled: false,
        outbound_limit: 500,
        inbound_limit: 0,
        outbound_window: 100,
        inbound_window: 0,
    };
    client.set_rate_limit_config(&id(0), &Some(default_config), &contract_id);

    set_config(&client, &contract_id, 42, outbound_config(200, 50));
    assert_eq!(client.get_rate_limit_usages(&id(42)).outbound_available_amount, 200);
    assert!(client.rate_limits(&id(42)).1.is_some());

    client.set_rate_limit_config(&id(42), &None, &contract_id);
    assert!(client.rate_limits(&id(42)).1.is_none());

    let usages = client.get_rate_limit_usages(&id(42));
    assert_eq!(usages.outbound_available_amount, 500, "should fall back to default config");
}

#[test]
fn test_remove_config_preserves_state() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);

    let default_config = RateLimitConfig {
        outbound_enabled: true,
        inbound_enabled: false,
        net_accounting_enabled: false,
        address_exemption_enabled: false,
        outbound_limit: 500,
        inbound_limit: 0,
        outbound_window: 100,
        inbound_window: 0,
    };
    client.set_rate_limit_config(&id(0), &Some(default_config), &contract_id);

    set_config(&client, &contract_id, 42, outbound_config(200, 50));
    client.outflow(&id(42), &user, &100i128);

    let (state_before, _) = client.rate_limits(&id(42));
    assert_eq!(state_before.outbound_usage, 100);

    client.set_rate_limit_config(&id(42), &None, &contract_id);

    let (state_after, _) = client.rate_limits(&id(42));
    assert_eq!(state_after.outbound_usage, 100, "state should be preserved after config removal");
}

// ============================================================================
// Config Update Tests
// ============================================================================

#[test]
fn test_updating_config_preserves_usage() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);
    let eid = 42u32;

    env.ledger().set_timestamp(1_000);
    set_config(&client, &contract_id, eid, outbound_config(100, 10));
    client.outflow(&id(eid), &user, &100i128);

    env.ledger().set_timestamp(1_005);
    set_config(&client, &contract_id, eid, outbound_config(200, 20));

    let (state, config) = client.rate_limits(&id(eid));
    assert_eq!(state.outbound_usage, 100, "raw usage in storage should be preserved");
    assert_eq!(config.unwrap().outbound_limit, 200, "config should be updated");
}

#[test]
fn test_reducing_limit_below_in_flight_clamps_capacity() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);

    set_config(&client, &contract_id, 1, outbound_config(100, 100));
    client.outflow(&id(1), &user, &90i128);

    set_config(&client, &contract_id, 1, outbound_config(50, 100));
    assert_eq!(client.get_rate_limit_usages(&id(1)).outbound_available_amount, 0);
}

// ============================================================================
// Address Exemption Tests
// ============================================================================

#[test]
fn test_address_exemption_bypasses_rate_limit() {
    let TestSetup { env, client, contract_id } = setup();
    let exempt_user = Address::generate(&env);
    let normal_user = Address::generate(&env);

    let config = RateLimitConfig {
        outbound_enabled: true,
        inbound_enabled: true,
        net_accounting_enabled: false,
        address_exemption_enabled: true,
        outbound_limit: 10,
        inbound_limit: 10,
        outbound_window: 100,
        inbound_window: 100,
    };
    set_config(&client, &contract_id, 1, config);

    client.set_rate_limit_exemption(&exempt_user, &true, &contract_id);
    assert!(client.is_rate_limit_exemption(&exempt_user));

    // Exempt user bypasses both outbound and inbound
    client.outflow(&id(1), &exempt_user, &999i128);
    client.inflow(&id(1), &exempt_user, &999i128);
    let usages = client.get_rate_limit_usages(&id(1));
    assert_eq!(usages.outbound_usage, 0, "exempt user should not affect outbound usage");
    assert_eq!(usages.inbound_usage, 0, "exempt user should not affect inbound usage");

    // Non-exempt user is still rate-limited
    let res = client.try_outflow(&id(1), &normal_user, &11i128);
    assert_eq!(res.err().unwrap().ok().unwrap(), RateLimitError::RateLimitExceeded.into());

    // Remove exemption → user is now rate-limited
    client.set_rate_limit_exemption(&exempt_user, &false, &contract_id);
    assert!(!client.is_rate_limit_exemption(&exempt_user));
    let res = client.try_outflow(&id(1), &exempt_user, &11i128);
    assert_eq!(res.err().unwrap().ok().unwrap(), RateLimitError::RateLimitExceeded.into());
}

#[test]
fn test_exemption_ignored_when_config_flag_disabled() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);

    set_config(&client, &contract_id, 1, outbound_config(10, 100));
    client.set_rate_limit_exemption(&user, &true, &contract_id);

    let res = client.try_outflow(&id(1), &user, &11i128);
    assert_eq!(res.err().unwrap().ok().unwrap(), RateLimitError::RateLimitExceeded.into());
}

#[test]
fn test_exemption_state_idempotent() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);

    // true → true
    client.set_rate_limit_exemption(&user, &true, &contract_id);
    let res = client.try_set_rate_limit_exemption(&user, &true, &contract_id);
    assert_eq!(res.err().unwrap().ok().unwrap(), RateLimitError::ExemptionStateIdempotent.into());

    // false → false
    client.set_rate_limit_exemption(&user, &false, &contract_id);
    let res = client.try_set_rate_limit_exemption(&user, &false, &contract_id);
    assert_eq!(res.err().unwrap().ok().unwrap(), RateLimitError::ExemptionStateIdempotent.into());
}

// ============================================================================
// Set States Tests
// ============================================================================

#[test]
fn test_set_states_manually() {
    let TestSetup { env, client, contract_id } = setup();

    env.ledger().set_timestamp(1_000);
    set_config(&client, &contract_id, 1, outbound_config(100, 100));

    client.set_rate_limit_state(
        &id(1),
        &RateLimitState { outbound_usage: 50, inbound_usage: 20, last_updated: 1_000 },
        &contract_id,
    );

    let (state, _) = client.rate_limits(&id(1));
    assert_eq!(state.outbound_usage, 50);
    assert_eq!(state.inbound_usage, 20);
    assert_eq!(state.last_updated, 1_000);
}

#[test]
fn test_set_states_rejects_future_timestamp() {
    let TestSetup { env, client, contract_id } = setup();

    env.ledger().set_timestamp(1_000);
    let res = client.try_set_rate_limit_state(
        &id(1),
        &RateLimitState { outbound_usage: 0, inbound_usage: 0, last_updated: 2_000 },
        &contract_id,
    );
    assert_eq!(res.err().unwrap().ok().unwrap(), RateLimitError::LastUpdatedInFuture.into());
}

#[test]
fn test_set_states_rejects_negative_usage() {
    let TestSetup { client, contract_id, .. } = setup();

    let res = client.try_set_rate_limit_state(
        &id(1),
        &RateLimitState { outbound_usage: -1, inbound_usage: 0, last_updated: 0 },
        &contract_id,
    );
    assert_eq!(res.err().unwrap().ok().unwrap(), RateLimitError::OutboundUsageNegative.into());

    let res2 = client.try_set_rate_limit_state(
        &id(1),
        &RateLimitState { outbound_usage: 0, inbound_usage: -1, last_updated: 0 },
        &contract_id,
    );
    assert_eq!(res2.err().unwrap().ok().unwrap(), RateLimitError::InboundUsageNegative.into());
}

// ============================================================================
// Checkpoint Tests
// ============================================================================

#[test]
fn test_checkpoint_snapshots_decayed_usage() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    set_config(&client, &contract_id, 1, bidirectional_net_config(100, 10));

    client.outflow(&id(1), &user, &80i128);
    client.inflow(&id(1), &user, &40i128);

    env.ledger().set_timestamp(1_001);

    client.checkpoint_rate_limit(&id(1), &contract_id);

    let (state, _) = client.rate_limits(&id(1));
    assert_eq!(state.outbound_usage, 30, "checkpoint should store decayed outbound usage");
    assert_eq!(state.inbound_usage, 30, "checkpoint should store decayed inbound usage");
    assert_eq!(state.last_updated, 1_001);
}

#[test]
fn test_checkpoint_global_state_writes_to_default_id() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    client.set_rate_limit_global_config(
        &Some(RateLimitGlobalConfig { use_global_state: true, is_globally_disabled: false }),
        &contract_id,
    );
    set_config(&client, &contract_id, 0, outbound_config(100, 10));

    client.outflow(&id(77), &user, &100i128);

    env.ledger().set_timestamp(1_005);

    client.checkpoint_rate_limit(&id(77), &contract_id);

    let (default_state, _) = client.rate_limits(&id(0));
    assert_eq!(default_state.outbound_usage, 50, "checkpoint should write decayed usage to DEFAULT_ID");
    assert_eq!(default_state.last_updated, 1_005);

    let (per_id_state, _) = client.rate_limits(&id(77));
    assert_eq!(per_id_state, RateLimitState::default(), "per-ID state should remain untouched");
}

// ============================================================================
// __get_rate_limit_state_and_config Resolution Tests
// ============================================================================

#[test]
fn test_get_state_and_config_no_config_returns_defaults() {
    let TestSetup { client, .. } = setup();

    let (state_id, state, config) = client.get_state_and_config(&id(42));
    assert_eq!(state_id, id(42));
    assert_eq!(state, RateLimitState::default());
    assert_eq!(config, RateLimitConfig::default());
}

#[test]
fn test_get_state_and_config_inheritance_and_fallback() {
    let TestSetup { client, contract_id, .. } = setup();

    let default_cfg = outbound_config(500, 100);
    client.set_rate_limit_config(&id(0), &Some(default_cfg.clone()), &contract_id);

    // No per-ID config → inherits DEFAULT
    let (state_id, _, config) = client.get_state_and_config(&id(42));
    assert_eq!(state_id, id(42));
    assert_eq!(config, default_cfg);

    // Per-ID config exists → uses per-ID config
    let per_id_cfg = outbound_config(999, 50);
    client.set_rate_limit_config(&id(42), &Some(per_id_cfg.clone()), &contract_id);
    let (_, _, config) = client.get_state_and_config(&id(42));
    assert_eq!(config, per_id_cfg);

    // Remove per-ID config → falls back to DEFAULT
    client.set_rate_limit_config(&id(42), &None, &contract_id);
    let (_, _, config) = client.get_state_and_config(&id(42));
    assert_eq!(config, default_cfg);

    // Remove DEFAULT config → falls back to RateLimitConfig::default()
    client.set_rate_limit_config(&id(0), &None, &contract_id);
    let (_, _, config) = client.get_state_and_config(&id(42));
    assert_eq!(config, RateLimitConfig::default());
}

#[test]
fn test_get_state_and_config_override_with_state() {
    let TestSetup { env, client, contract_id } = setup();

    env.ledger().set_timestamp(1_000);
    let per_id_cfg = outbound_config(200, 50);
    client.set_rate_limit_config(&id(42), &Some(per_id_cfg.clone()), &contract_id);

    let seeded_state = RateLimitState { outbound_usage: 75, inbound_usage: 25, last_updated: 1_000 };
    client.set_rate_limit_state(&id(42), &seeded_state, &contract_id);

    let (state_id, state, config) = client.get_state_and_config(&id(42));
    assert_eq!(state_id, id(42));
    assert_eq!(state, seeded_state);
    assert_eq!(config, per_id_cfg);
}

#[test]
fn test_get_state_and_config_global_state_ignores_per_id() {
    let TestSetup { env, client, contract_id } = setup();

    env.ledger().set_timestamp(1_000);
    client.set_rate_limit_global_config(
        &Some(RateLimitGlobalConfig { use_global_state: true, is_globally_disabled: false }),
        &contract_id,
    );

    let default_cfg = outbound_config(500, 100);
    client.set_rate_limit_config(&id(0), &Some(default_cfg.clone()), &contract_id);
    let default_state = RateLimitState { outbound_usage: 30, inbound_usage: 10, last_updated: 1_000 };
    client.set_rate_limit_state(&id(0), &default_state, &contract_id);

    // Per-ID config and different state — should be ignored when use_global_state is true
    client.set_rate_limit_config(&id(42), &Some(outbound_config(200, 50)), &contract_id);
    client.set_rate_limit_state(
        &id(42),
        &RateLimitState { outbound_usage: 999, inbound_usage: 888, last_updated: 1_000 },
        &contract_id,
    );

    let (state_id, state, config) = client.get_state_and_config(&id(42));
    assert_eq!(state_id, 0u128, "state_id should be DEFAULT_ID");
    assert_eq!(state, default_state, "state should come from DEFAULT_ID");
    assert_eq!(config, default_cfg, "config should resolve from DEFAULT_ID");
}

// ============================================================================
// Edge Case Tests (window=0, huge limit)
// ============================================================================

#[test]
fn test_window_zero_decays_instantly() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    set_config(&client, &contract_id, 1, outbound_config(100, 0));

    client.outflow(&id(1), &user, &100i128);
    let usages = client.get_rate_limit_usages(&id(1));
    assert_eq!(usages.outbound_usage, 100);
    assert_eq!(usages.outbound_available_amount, 0);

    // 1 second later: effective_window=1 → decay = limit * 1 / 1 = limit → full restore
    env.ledger().set_timestamp(1_001);
    let usages = client.get_rate_limit_usages(&id(1));
    assert_eq!(usages.outbound_usage, 0);
    assert_eq!(usages.outbound_available_amount, 100);
}

#[test]
fn test_huge_limit_no_overflow() {
    let TestSetup { env, client, contract_id } = setup();
    let user = Address::generate(&env);
    let huge_limit = i128::MAX / 2;

    env.ledger().set_timestamp(1_000);
    set_config(&client, &contract_id, 1, outbound_config(huge_limit, 100));

    client.outflow(&id(1), &user, &huge_limit);
    let usages = client.get_rate_limit_usages(&id(1));
    assert_eq!(usages.outbound_usage, huge_limit);
    assert_eq!(usages.outbound_available_amount, 0);

    // After full window: saturating_mul caps decay, but capacity partially restores without panic
    env.ledger().set_timestamp(1_100);
    let usages = client.get_rate_limit_usages(&id(1));
    assert!(usages.outbound_usage < huge_limit, "usage should decrease");
    assert!(usages.outbound_available_amount > 0, "some capacity should restore");
}
