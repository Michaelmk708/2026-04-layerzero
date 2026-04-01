//! Ownership & delegate sync integration tests for Console OFT.
//!
//! Verifies that:
//! - Single-step `transfer_ownership` is disabled.
//! - `renounce_ownership` is disabled.
//! - `set_delegate` is disabled.
//! - 2-step ownership transfer works and auto-syncs the endpoint delegate.

use crate::integration_tests::setup::{setup, TestSetup};
use crate::errors::OFTError;
use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, Env, IntoVal,
};
use utils::errors::OwnableError;

// ============================================================================
// Helpers
// ============================================================================

fn get_delegate(_env: &Env, setup: &crate::integration_tests::setup::ChainSetup<'_>) -> Option<Address> {
    setup.endpoint.delegate(&setup.oft.address)
}

// ============================================================================
// Disabled operations
// ============================================================================

#[test]
fn test_transfer_ownership_disabled() {
    let TestSetup { env, chain_a, .. } = setup();

    let new_owner = Address::generate(&env);
    env.mock_auths(&[MockAuth {
        address: &chain_a.owner,
        invoke: &MockAuthInvoke {
            contract: &chain_a.oft.address,
            fn_name: "transfer_ownership",
            args: (&new_owner,).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    let res = chain_a.oft.try_transfer_ownership(&new_owner);
    assert_eq!(res.err().unwrap().ok().unwrap(), OFTError::Disabled.into());
}

#[test]
fn test_renounce_ownership_disabled() {
    let TestSetup { env, chain_a, .. } = setup();

    env.mock_auths(&[MockAuth {
        address: &chain_a.owner,
        invoke: &MockAuthInvoke {
            contract: &chain_a.oft.address,
            fn_name: "renounce_ownership",
            args: ().into_val(&env),
            sub_invokes: &[],
        },
    }]);
    let res = chain_a.oft.try_renounce_ownership();
    assert_eq!(res.err().unwrap().ok().unwrap(), OFTError::Disabled.into());
}

#[test]
fn test_set_delegate_disabled() {
    let TestSetup { env, chain_a, .. } = setup();

    let new_delegate: Option<Address> = Some(Address::generate(&env));
    env.mock_auths(&[MockAuth {
        address: &chain_a.owner,
        invoke: &MockAuthInvoke {
            contract: &chain_a.oft.address,
            fn_name: "set_delegate",
            args: (&new_delegate, &chain_a.owner).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    let res = chain_a.oft.try_set_delegate(&new_delegate, &chain_a.owner);
    assert_eq!(res.err().unwrap().ok().unwrap(), OFTError::Disabled.into());
}

// ============================================================================
// 2-step ownership transfer with delegate sync
// ============================================================================

#[test]
fn test_two_step_transfer_syncs_delegate() {
    let TestSetup { env, chain_a, .. } = setup();
    let new_owner = Address::generate(&env);
    let ttl = 1000u32;

    // Verify initial state: owner == delegate
    assert_eq!(chain_a.oft.owner(), Some(chain_a.owner.clone()));
    assert_eq!(get_delegate(&env, &chain_a), Some(chain_a.owner.clone()));

    // Step 1: begin_ownership_transfer (owner initiates)
    env.mock_auths(&[MockAuth {
        address: &chain_a.owner,
        invoke: &MockAuthInvoke {
            contract: &chain_a.oft.address,
            fn_name: "begin_ownership_transfer",
            args: (&new_owner, &ttl).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    chain_a.oft.begin_ownership_transfer(&new_owner, &ttl);

    // Owner and delegate unchanged during pending state
    assert_eq!(chain_a.oft.owner(), Some(chain_a.owner.clone()));
    assert_eq!(chain_a.oft.pending_owner(), Some(new_owner.clone()));
    assert_eq!(get_delegate(&env, &chain_a), Some(chain_a.owner.clone()));

    // Step 2: accept_ownership (new owner accepts)
    env.mock_auths(&[MockAuth {
        address: &new_owner,
        invoke: &MockAuthInvoke {
            contract: &chain_a.oft.address,
            fn_name: "accept_ownership",
            args: ().into_val(&env),
            sub_invokes: &[MockAuthInvoke {
                contract: &chain_a.endpoint.address,
                fn_name: "set_delegate",
                args: (&chain_a.oft.address, &Some(new_owner.clone())).into_val(&env),
                sub_invokes: &[],
            }],
        },
    }]);
    chain_a.oft.accept_ownership();

    // Verify: owner == delegate == new_owner
    assert_eq!(chain_a.oft.owner(), Some(new_owner.clone()));
    assert_eq!(chain_a.oft.pending_owner(), None);
    assert_eq!(get_delegate(&env, &chain_a), Some(new_owner));
}

#[test]
fn test_accept_ownership_fails_without_pending_transfer() {
    let TestSetup { env, chain_a, .. } = setup();

    let random = Address::generate(&env);
    env.mock_auths(&[MockAuth {
        address: &random,
        invoke: &MockAuthInvoke {
            contract: &chain_a.oft.address,
            fn_name: "accept_ownership",
            args: ().into_val(&env),
            sub_invokes: &[],
        },
    }]);
    let res = chain_a.oft.try_accept_ownership();
    assert_eq!(res.err().unwrap().ok().unwrap(), OwnableError::NoPendingTransfer.into());
}

#[test]
fn test_initial_owner_equals_delegate() {
    let TestSetup { env, chain_a, .. } = setup();

    let owner = chain_a.oft.owner().unwrap();
    let delegate = get_delegate(&env, &chain_a).unwrap();
    assert_eq!(owner, delegate);
}
