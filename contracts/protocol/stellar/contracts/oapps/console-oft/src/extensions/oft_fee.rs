use common_macros::{contract_error, contract_trait, only_role, storage};
use soroban_sdk::{assert_with_error, contractevent, token::TokenClient, Address, Env};
use utils::rbac::{RoleBasedAccessControl, AUTHORIZER};

/// Role for fee configuration (set_default_fee_bps, set_fee_bps).
pub const FEE_CONFIG_MANAGER_ROLE: &str = "FEE_CONFIG_MANAGER_ROLE";

/// Base fee in basis points (10,000 BPS = 100%)
/// Used as denominator in fee calculations
pub const BASE_FEE_BPS: u32 = 10_000;

// =========================================================================
// Storage
// =========================================================================

#[storage]
pub enum OFTFeeStorage {
    /// Default fee rate in basis points (0-10,000, where 10,000 = 100%)
    #[instance(u32)]
    #[default(0)]
    DefaultFeeBps,

    /// Destination-specific fee rates mapped by Destination ID
    #[persistent(u32)]
    FeeBps { id: u128 },

    /// Address where collected fees will be deposited
    #[instance(Address)]
    FeeDeposit,
}

// =========================================================================
// Errors
// =========================================================================

#[contract_error]
pub enum OFTFeeError {
    InvalidBps = 3100,
    SameValue,
}

// =========================================================================
// Events
// =========================================================================

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DefaultFeeBpsSet {
    pub fee_bps: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeBpsSet {
    pub id: u128,
    /// The fee rate in basis points, or None if the fee is removed
    pub fee_bps: Option<u32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeDepositSet {
    #[topic]
    pub fee_deposit: Address,
}

// =========================================================================
// Trait With Default Implementations
// =========================================================================

#[contract_trait]
pub trait OFTFee: OFTFeeInternal + RoleBasedAccessControl {
    // =========================================================================
    // Management Functions
    // =========================================================================

    /// Sets the default fee rate in basis points (0-10,000, where 10,000 = 100%).
    ///
    /// Setting to 0 removes the default fee from storage (effective rate becomes 0).
    ///
    /// * `operator` - The address that must have FEE_CONFIG_MANAGER_ROLE
    #[only_role(operator, FEE_CONFIG_MANAGER_ROLE)]
    fn set_default_fee_bps(env: &soroban_sdk::Env, default_fee_bps: u32, operator: &soroban_sdk::Address) {
        Self::__set_default_fee_bps(env, default_fee_bps);
    }

    /// Sets or removes the fee rate for a specific Destination ID.
    ///
    /// - `Some(0)`: explicitly sets zero fee for this Destination ID, overriding the default fee.
    /// - `None`: removes the per-Destination ID override; falls back to the default fee.
    ///
    /// # Arguments
    /// * `id` - The Destination ID
    /// * `fee_bps` - The fee rate (0-10,000), or None to remove the fee configuration
    /// * `operator` - The address that must have FEE_CONFIG_MANAGER_ROLE
    #[only_role(operator, FEE_CONFIG_MANAGER_ROLE)]
    fn set_fee_bps(env: &soroban_sdk::Env, id: u128, fee_bps: &Option<u32>, operator: &soroban_sdk::Address) {
        Self::__set_fee_bps(env, id, fee_bps);
    }

    /// Sets the address where collected fees will be deposited.
    ///
    /// # Arguments
    /// * `fee_deposit` - The address to deposit fees to
    /// * `operator` - The authorizer address
    #[only_role(operator, AUTHORIZER)]
    fn set_fee_deposit(env: &soroban_sdk::Env, fee_deposit: &soroban_sdk::Address, operator: &soroban_sdk::Address) {
        Self::__set_fee_deposit(env, fee_deposit);
    }

    // =========================================================================
    // View Functions
    // =========================================================================

    /// Calculates the fee for a given amount and Destination ID.
    /// Returns `(amount * fee_bps) / 10,000`, rounded down.
    fn get_fee(env: &soroban_sdk::Env, id: u128, amount: i128) -> i128 {
        Self::__get_fee(env, id, amount)
    }

    /// Given an amount after fee deduction, calculates the original amount before the fee.
    /// Inverse of `get_fee`: if `fee = get_fee(id, original)` and `after = original - fee`,
    /// then `get_amount_before_fee(id, after)` returns `original`.
    fn get_amount_before_fee(env: &soroban_sdk::Env, id: u128, amount_after_fee: i128) -> i128 {
        let fee_bps = Self::__effective_fee_bps(env, id);
        if fee_bps == BASE_FEE_BPS {
            return 0;
        }
        (amount_after_fee * BASE_FEE_BPS as i128) / (BASE_FEE_BPS - fee_bps) as i128
    }

    /// Returns the default fee rate in basis points (0 if unset).
    fn default_fee_bps(env: &soroban_sdk::Env) -> u32 {
        Self::__default_fee_bps(env)
    }

    /// Returns the fee rate for a specific Destination ID, if set.
    fn fee_bps(env: &soroban_sdk::Env, id: u128) -> Option<u32> {
        Self::__fee_bps(env, id)
    }

    /// Returns the fee deposit address.
    fn fee_deposit(env: &soroban_sdk::Env) -> soroban_sdk::Address {
        Self::__fee_deposit(env)
    }
}

/// Internal trait for OFT fee operations used by OFT hooks.
/// Contains only truly internal methods that are called from OFTFee implementations.
pub trait OFTFeeInternal {
    // =========================================================================
    // OFT Hooks
    // =========================================================================

    /// Charges the fee by transferring the fee amount from the sender to the fee deposit address.
    /// Used internally by `__debit` to collect the fee.
    ///
    /// # Arguments
    /// * `token` - The token address to transfer
    /// * `from` - The address to transfer fee from
    /// * `fee_amount` - The fee amount to transfer
    fn __charge_fee(env: &Env, token: &Address, from: &Address, fee_amount: i128) {
        if fee_amount != 0 {
            TokenClient::new(env, token).transfer(from, Self::__fee_deposit(env), &fee_amount);
        }
    }

    // =========================================================================
    // Management Functions
    // =========================================================================

    /// Sets the default fee rate in basis points.
    ///
    /// # Arguments
    /// * `default_fee_bps` - The default fee rate (0-10,000, where 10,000 = 100%). 0 removes the entry from storage.
    fn __set_default_fee_bps(env: &Env, default_fee_bps: u32) {
        assert_with_error!(env, default_fee_bps <= BASE_FEE_BPS, OFTFeeError::InvalidBps);
        if default_fee_bps == 0 {
            OFTFeeStorage::remove_default_fee_bps(env);
        } else {
            OFTFeeStorage::set_default_fee_bps(env, &default_fee_bps);
        }
        DefaultFeeBpsSet { fee_bps: default_fee_bps }.publish(env);
    }

    /// Sets or removes the fee rate for a specific Destination ID.
    ///
    /// - `Some(0)`: explicitly sets zero fee for this Destination ID, overriding the default fee.
    /// - `None`: removes the per-Destination ID override; falls back to the default fee.
    ///
    /// # Arguments
    /// * `id` - The Destination ID
    /// * `fee_bps` - The fee rate (0-10,000), or None to remove the fee configuration
    fn __set_fee_bps(env: &Env, id: u128, fee_bps: &Option<u32>) {
        assert_with_error!(env, fee_bps.is_none_or(|bps| bps <= BASE_FEE_BPS), OFTFeeError::InvalidBps);
        OFTFeeStorage::set_or_remove_fee_bps(env, id, fee_bps);
        FeeBpsSet { id, fee_bps: *fee_bps }.publish(env);
    }

    /// Sets the address where collected fees will be deposited.
    /// Called during construction to ensure the fee deposit is always initialized.
    ///
    /// # Arguments
    /// * `fee_deposit` - The address to deposit fees to
    fn __set_fee_deposit(env: &Env, fee_deposit: &Address) {
        assert_with_error!(env, OFTFeeStorage::fee_deposit(env).as_ref() != Some(fee_deposit), OFTFeeError::SameValue);
        OFTFeeStorage::set_fee_deposit(env, fee_deposit);
        FeeDepositSet { fee_deposit: fee_deposit.clone() }.publish(env);
    }

    // =========================================================================
    // View Functions
    // =========================================================================

    fn __get_fee(env: &Env, id: u128, amount: i128) -> i128 {
        let fee_bps = Self::__effective_fee_bps(env, id);
        (amount * fee_bps as i128) / BASE_FEE_BPS as i128
    }

    /// Returns the effective fee rate for a Destination ID (Destination ID-specific or default).
    fn __effective_fee_bps(env: &Env, id: u128) -> u32 {
        Self::__fee_bps(env, id).unwrap_or_else(|| Self::__default_fee_bps(env))
    }

    /// Returns the default fee rate in basis points (0 if unset).
    fn __default_fee_bps(env: &Env) -> u32 {
        OFTFeeStorage::default_fee_bps(env)
    }

    /// Returns the fee rate for a specific Destination ID, if set.
    fn __fee_bps(env: &Env, id: u128) -> Option<u32> {
        OFTFeeStorage::fee_bps(env, id)
    }

    /// Returns the fee deposit address
    fn __fee_deposit(env: &Env) -> Address {
        OFTFeeStorage::fee_deposit(env).unwrap()
    }
}
