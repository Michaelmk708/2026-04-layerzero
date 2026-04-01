extern crate std;

use crate::extensions::oft_fee::FEE_CONFIG_MANAGER_ROLE;
use crate::extensions::oft_fee::{OFTFee, OFTFeeError, OFTFeeInternal};
use soroban_sdk::{
    contract, contractimpl,
    testutils::Address as _,
    token::{StellarAssetClient, TokenClient},
    Address, Env, Symbol,
};
use utils::auth::Auth;
use utils::rbac::{grant_role_no_auth, RoleBasedAccessControl};

// ============================================================================
// Test Contract
// ============================================================================

#[contract]
struct FeeTestContract;

impl Auth for FeeTestContract {
    fn authorizer(env: &Env) -> Option<Address> {
        Some(env.current_contract_address())
    }
}

impl OFTFeeInternal for FeeTestContract {}

#[contractimpl(contracttrait)]
impl OFTFee for FeeTestContract {}

#[contractimpl(contracttrait)]
impl RoleBasedAccessControl for FeeTestContract {}

#[contractimpl]
impl FeeTestContract {
    pub fn init_roles(env: Env) {
        let contract_id = env.current_contract_address();
        grant_role_no_auth(&env, &contract_id, &Symbol::new(&env, FEE_CONFIG_MANAGER_ROLE), &contract_id);
    }

    pub fn charge_fee(env: Env, token: Address, from: Address, fee_amount: i128) {
        <Self as OFTFeeInternal>::__charge_fee(&env, &token, &from, fee_amount);
    }
}

// ============================================================================
// Test Setup
// ============================================================================

struct TestSetup {
    env: Env,
    client: FeeTestContractClient<'static>,
    contract_id: Address,
    token: Address,
    from: Address,
    fee_deposit: Address,
}

fn setup() -> TestSetup {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let contract_id = env.register(FeeTestContract, ());
    let client = FeeTestContractClient::new(&env, &contract_id);
    client.init_roles();

    let token_admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token = sac.address();

    let from = Address::generate(&env);
    let fee_deposit = Address::generate(&env);

    StellarAssetClient::new(&env, &token).mint(&from, &1_000_000i128);

    TestSetup { env, client, contract_id, token, from, fee_deposit }
}

fn id(v: u32) -> u128 {
    v as u128
}

// ============================================================================
// Set Default Fee BPS Tests
// ============================================================================

#[test]
fn test_set_default_fee_bps_rejects_invalid_value() {
    let TestSetup { client, contract_id, .. } = setup();

    let res = client.try_set_default_fee_bps(&10_001u32, &contract_id);
    assert_eq!(res.err().unwrap().ok().unwrap(), OFTFeeError::InvalidBps.into());
}

// ============================================================================
// Set Fee BPS Tests
// ============================================================================

#[test]
fn test_set_fee_bps_rejects_invalid_value() {
    let TestSetup { client, contract_id, .. } = setup();
    let id_101 = id(101);

    let res = client.try_set_fee_bps(&id_101, &Some(10_001u32), &contract_id);
    assert_eq!(res.err().unwrap().ok().unwrap(), OFTFeeError::InvalidBps.into());
}

#[test]
fn test_set_fee_bps_set_and_remove() {
    let TestSetup { client, contract_id, .. } = setup();
    let id_101 = id(101);

    client.set_fee_bps(&id_101, &Some(200u32), &contract_id);
    assert_eq!(client.fee_bps(&id_101), Some(200));
    assert_eq!(client.get_fee(&id_101, &10_000i128), 200);

    client.set_default_fee_bps(&111u32, &contract_id);
    client.set_fee_bps(&id_101, &None, &contract_id);
    assert_eq!(client.fee_bps(&id_101), None);
    assert_eq!(client.default_fee_bps(), 111);
    assert_eq!(client.get_fee(&id_101, &10_000i128), 111);

    // Per-ID Some(0) explicitly overrides default to zero fee
    client.set_fee_bps(&id_101, &Some(0u32), &contract_id);
    assert_eq!(client.fee_bps(&id_101), Some(0));
    assert_eq!(client.get_fee(&id_101, &10_000i128), 0);
    assert_eq!(client.get_fee(&id(99), &10_000i128), 111, "other IDs still use default");

    // set_default_fee_bps(0) removes the default fee
    client.set_default_fee_bps(&0u32, &contract_id);
    assert_eq!(client.default_fee_bps(), 0);
    assert_eq!(client.get_fee(&id(99), &10_000i128), 0);
}

// ============================================================================
// Set Fee Deposit Tests
// ============================================================================

#[test]
fn test_set_fee_deposit_same_value() {
    let TestSetup { client, contract_id, fee_deposit, .. } = setup();

    client.set_fee_deposit(&fee_deposit, &contract_id);
    let res = client.try_set_fee_deposit(&fee_deposit, &contract_id);
    assert_eq!(res.err().unwrap().ok().unwrap(), OFTFeeError::SameValue.into());
}

// ============================================================================
// Charge Fee Tests
// ============================================================================

#[test]
fn test_charge_fee_zero_amount_no_transfer() {
    let TestSetup { env, client, contract_id, token, from, fee_deposit, .. } = setup();

    client.set_fee_deposit(&fee_deposit, &contract_id);

    let token_client = TokenClient::new(&env, &token);
    let from_before = token_client.balance(&from);
    let dep_before = token_client.balance(&fee_deposit);

    client.charge_fee(&token, &from, &0i128);

    assert_eq!(token_client.balance(&from), from_before);
    assert_eq!(token_client.balance(&fee_deposit), dep_before);
}

#[test]
fn test_charge_fee_transfers() {
    let TestSetup { env, client, contract_id, token, from, fee_deposit, .. } = setup();

    client.set_fee_deposit(&fee_deposit, &contract_id);

    let token_client = TokenClient::new(&env, &token);
    let from_before = token_client.balance(&from);
    let dep_before = token_client.balance(&fee_deposit);

    client.charge_fee(&token, &from, &123i128);

    assert_eq!(token_client.balance(&from), from_before - 123);
    assert_eq!(token_client.balance(&fee_deposit), dep_before + 123);
}

// ============================================================================
// Fee View Tests
// ============================================================================

#[test]
fn test_fee_view_no_fee_returns_zero() {
    let TestSetup { client, .. } = setup();

    assert_eq!(client.get_fee(&id(1), &10_000i128), 0);
}

#[test]
fn test_fee_view_computes_correct_fee() {
    let TestSetup { client, contract_id, fee_deposit, .. } = setup();

    client.set_fee_deposit(&fee_deposit, &contract_id);
    client.set_default_fee_bps(&250u32, &contract_id);

    assert_eq!(client.get_fee(&id(1), &10_000i128), 250);
}

// ============================================================================
// Get Amount Before Fee Tests
// ============================================================================

#[test]
fn test_get_amount_before_fee_no_fee() {
    let TestSetup { client, .. } = setup();

    let result = client.get_amount_before_fee(&id(1), &10_000i128);
    assert_eq!(result, 10_000);
}

#[test]
fn test_get_amount_before_fee_roundtrip() {
    let TestSetup { client, contract_id, .. } = setup();

    client.set_default_fee_bps(&250u32, &contract_id); // 2.5%

    let original = 100_000i128;
    let fee = client.get_fee(&id(1), &original);
    let after_fee = original - fee;
    let recovered = client.get_amount_before_fee(&id(1), &after_fee);
    assert_eq!(recovered, original);
}

#[test]
fn test_get_amount_before_fee_full_fee_returns_zero() {
    let TestSetup { client, contract_id, .. } = setup();

    client.set_default_fee_bps(&10_000u32, &contract_id); // 100%

    let result = client.get_amount_before_fee(&id(1), &5_000i128);
    assert_eq!(result, 0);
}

#[test]
fn test_get_amount_before_fee_per_destination_override() {
    let TestSetup { client, contract_id, .. } = setup();

    client.set_default_fee_bps(&100u32, &contract_id); // 1% default
    client.set_fee_bps(&id(42), &Some(500u32), &contract_id); // 5% for id 42

    // 5% fee: after = 9_500 → before = 9_500 * 10_000 / 9_500 = 10_000
    let result = client.get_amount_before_fee(&id(42), &9_500i128);
    assert_eq!(result, 10_000);

    // Other destinations still use default 1%
    let result_default = client.get_amount_before_fee(&id(99), &9_900i128);
    assert_eq!(result_default, 10_000);
}
