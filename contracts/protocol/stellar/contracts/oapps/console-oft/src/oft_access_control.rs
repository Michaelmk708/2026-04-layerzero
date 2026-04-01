//! Console-specific access control: ownership, auth, and RBAC.
//!
//! - Delegate is permanently synced with the owner.
//! - `transfer_ownership` (single-step) and `renounce_ownership` are disabled.
//! - Only 2-step transfer via `begin_ownership_transfer` + `accept_ownership`.
//! - `accept_ownership` auto-syncs the endpoint delegate to the new owner.

use super::{OFTArgs, OFTClient, OFT};
use crate::errors::OFTError;
use common_macros::contract_impl;
use endpoint_v2::LayerZeroEndpointV2Client;
use oapp::oapp_core::OAppCore;
use soroban_sdk::{panic_with_error, Address, Env};
use utils::{
    auth::Auth,
    ownable::{Ownable, OwnableInitializer},
    rbac::RoleBasedAccessControl,
};

// =========================================================================
// Ownable & Auth implementation for OFT
// =========================================================================

impl OwnableInitializer for OFT {}

#[contract_impl]
impl Auth for OFT {
    fn authorizer(env: &soroban_sdk::Env) -> Option<Address> {
        Self::owner(env)
    }
}

#[contract_impl(contracttrait)]
impl Ownable for OFT {
    /// Disabled one-step ownership transfer.
    fn transfer_ownership(env: &soroban_sdk::Env, _new_owner: &soroban_sdk::Address) {
        panic_with_error!(env, OFTError::Disabled);
    }

    /// Disabled renounce ownership.
    fn renounce_ownership(env: &soroban_sdk::Env) {
        panic_with_error!(env, OFTError::Disabled);
    }

    /// Accepts ownership and syncs the delegate on the endpoint to the new owner.
    fn accept_ownership(env: &soroban_sdk::Env) {
        OwnableDefault::accept_ownership(env);

        let new_owner = Self::owner(env).unwrap();
        LayerZeroEndpointV2Client::new(env, &Self::endpoint(env))
            .set_delegate(&env.current_contract_address(), &Some(new_owner));
    }
}

// =========================================================================
// RoleBasedAccessControl implementation
// =========================================================================

#[contract_impl(contracttrait)]
impl RoleBasedAccessControl for OFT {}

// =========================================================================
// OAppCore override — set_delegate disabled
// =========================================================================

#[contract_impl(contracttrait)]
impl OAppCore for OFT {
    /// Disabled set delegate.
    fn set_delegate(
        env: &soroban_sdk::Env,
        _delegate: &Option<soroban_sdk::Address>,
        _operator: &soroban_sdk::Address,
    ) {
        panic_with_error!(env, OFTError::Disabled);
    }
}

// =========================================================================
// Helper: access default Ownable implementation
// =========================================================================

/// Helper type to call the default `accept_ownership` logic before applying
/// Console-specific post-acceptance hooks (delegate sync).
/// Reads from the same `OwnableStorage` — no duplication of logic.
struct OwnableDefault;

impl Ownable for OwnableDefault {}

impl Auth for OwnableDefault {
    fn authorizer(env: &Env) -> Option<Address> {
        <Self as Ownable>::owner(env)
    }
}
