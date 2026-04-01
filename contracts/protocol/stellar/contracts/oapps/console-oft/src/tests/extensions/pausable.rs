extern crate std;

use crate::extensions::pausable::{OFTPausable, OFTPausableError, OFTPausableInternal};
use crate::extensions::pausable::{PAUSER_ROLE, UNPAUSER_ROLE};
use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};
use utils::auth::Auth;
use utils::rbac::{grant_role_no_auth, RoleBasedAccessControl};

// ============================================================================
// Test Contract
// ============================================================================

#[contract]
struct PausableTestContract;

impl Auth for PausableTestContract {
    fn authorizer(env: &Env) -> Option<Address> {
        Some(env.current_contract_address())
    }
}

impl OFTPausableInternal for PausableTestContract {}

#[contractimpl(contracttrait)]
impl OFTPausable for PausableTestContract {}

#[contractimpl(contracttrait)]
impl RoleBasedAccessControl for PausableTestContract {}

#[contractimpl]
impl PausableTestContract {
    /// Test-only: grants PAUSER_ROLE and UNPAUSER_ROLE to the contract.
    pub fn init_roles(env: Env) {
        let contract_id = env.current_contract_address();
        grant_role_no_auth(&env, &contract_id, &Symbol::new(&env, PAUSER_ROLE), &contract_id);
        grant_role_no_auth(&env, &contract_id, &Symbol::new(&env, UNPAUSER_ROLE), &contract_id);
    }

    /// Test-only: calls __assert_not_paused for the given Destination ID.
    pub fn assert_not_paused(env: Env, id: u32) {
        <Self as OFTPausableInternal>::__assert_not_paused(&env, id as u128);
    }
}

// ============================================================================
// Test Setup
// ============================================================================

struct TestSetup {
    #[allow(dead_code)]
    env: Env,
    client: PausableTestContractClient<'static>,
    contract_id: Address,
}

fn setup() -> TestSetup {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let contract_id = env.register(PausableTestContract, ());
    let client = PausableTestContractClient::new(&env, &contract_id);
    client.init_roles();
    TestSetup { env, client, contract_id }
}

fn id_0() -> u128 {
    0u128
}

// ============================================================================
// Initial State Tests
// ============================================================================

#[test]
fn test_initial_state_is_not_paused() {
    let TestSetup { client, .. } = setup();

    assert!(!client.default_paused());
    assert!(!client.is_paused(&id_0()));
}

// ============================================================================
// Set default paused / set_paused Tests
// ============================================================================

#[test]
fn test_set_default_paused_sets_paused() {
    let TestSetup { client, contract_id, .. } = setup();

    client.set_default_paused(&true, &contract_id);
    assert!(client.default_paused());
    assert!(client.is_paused(&id_0()));
}

#[test]
fn test_set_default_paused_unpause_after_pausing() {
    let TestSetup { client, contract_id, .. } = setup();

    client.set_default_paused(&true, &contract_id);
    assert!(client.default_paused());

    client.set_default_paused(&false, &contract_id);
    assert!(!client.default_paused());
    assert!(!client.is_paused(&id_0()));
}

#[test]
fn test_set_default_paused_idempotent_when_unpaused() {
    let TestSetup { client, contract_id, .. } = setup();

    // Initially not paused, set_default_paused(false) should fail (idempotent)
    let res = client.try_set_default_paused(&false, &contract_id);
    assert_eq!(res.err().unwrap().ok().unwrap(), OFTPausableError::PauseStateIdempotent.into());
}

#[test]
fn test_set_default_paused_idempotent_when_paused() {
    let TestSetup { client, contract_id, .. } = setup();

    client.set_default_paused(&true, &contract_id);

    // Already paused, set_default_paused(true) again should fail
    let res = client.try_set_default_paused(&true, &contract_id);
    assert_eq!(res.err().unwrap().ok().unwrap(), OFTPausableError::PauseStateIdempotent.into());
}

// ============================================================================
// Per-id set_paused Tests
// ============================================================================

#[test]
fn test_set_paused_per_id_override() {
    let TestSetup { client, contract_id, .. } = setup();

    let id_100 = 100u128;
    client.set_paused(&id_100, &Some(true), &contract_id);

    // Destination ID 100 is paused via override; default still unpaused; Destination ID 0 uses default
    assert!(!client.default_paused());
    assert!(client.is_paused(&id_100));
    assert!(!client.is_paused(&id_0()));
    assert_eq!(client.pause_config(&id_100), Some(true));
}

#[test]
fn test_set_paused_per_id_unpaused_override_when_default_paused() {
    let TestSetup { client, contract_id, .. } = setup();

    let id_100 = 100u128;
    client.set_default_paused(&true, &contract_id);

    // Override Destination ID 100 to unpaused while default is paused
    client.set_paused(&id_100, &Some(false), &contract_id);

    assert!(client.default_paused());
    assert!(!client.is_paused(&id_100));
    assert!(client.is_paused(&id_0()));
}

#[test]
fn test_set_paused_per_id_removal_falls_back_to_default() {
    let TestSetup { client, contract_id, .. } = setup();

    let id_100 = 100u128;

    // Set per-ID override, then remove it
    client.set_paused(&id_100, &Some(true), &contract_id);
    assert!(client.is_paused(&id_100));
    assert_eq!(client.pause_config(&id_100), Some(true));

    client.set_paused(&id_100, &None, &contract_id);
    assert!(!client.is_paused(&id_100));
    assert_eq!(client.pause_config(&id_100), None);
}

// ============================================================================
// Assert Not Paused Tests
// ============================================================================

#[test]
fn test_assert_not_paused_succeeds_when_not_paused() {
    let TestSetup { client, .. } = setup();

    client.assert_not_paused(&0);
}

#[test]
fn test_assert_not_paused_fails_when_paused() {
    let TestSetup { client, contract_id, .. } = setup();

    client.set_default_paused(&true, &contract_id);

    let res = client.try_assert_not_paused(&0);
    assert_eq!(res.err().unwrap().ok().unwrap(), OFTPausableError::Paused.into());
}

// ============================================================================
// Toggle Tests
// ============================================================================

#[test]
fn test_set_default_paused_toggle_multiple_times() {
    let TestSetup { client, contract_id, .. } = setup();

    for _ in 0..3 {
        assert!(!client.default_paused());
        client.set_default_paused(&true, &contract_id);
        assert!(client.default_paused());
        client.set_default_paused(&false, &contract_id);
    }
    assert!(!client.default_paused());
}
