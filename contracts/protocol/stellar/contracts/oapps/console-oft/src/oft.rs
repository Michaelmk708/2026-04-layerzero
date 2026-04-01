//! Console OFT — Omnichain Fungible Token with RBAC, pausable, fee, and rate limiter extensions.
//!
//! Supports two operating modes:
//!
//! - **LockUnlock**: Locks tokens on send (transfer to contract), unlocks on receive.
//! - **MintBurn**: Burns tokens on send, mints on receive via a configurable `Mintable` contract.
//!
//! ## Extension Hooks (applied in `__debit` / `__credit`)
//!
//! **Send path (`__debit`):**
//!   1. Pause check (`__assert_not_paused`) — reverts if paused for the destination ID
//!   2. Token debit (burn or lock `amount_received_ld`)
//!   3. Rate limit outflow (`__outflow`) — consumes outbound capacity, releases inbound if net accounting
//!   4. Fee charge (`__charge_fee`) — transfers fee to the fee deposit address
//!
//! **Receive path (`__credit`):**
//!   1. Rate limit inflow (`__inflow`) — consumes inbound capacity, releases outbound if net accounting
//!   2. Token credit (mint or unlock `amount_ld`)
//!   3. No pause check, no fee

use crate::{
    self as oft,
    extensions::{
        oft_fee::{OFTFee, OFTFeeInternal},
        pausable::{OFTPausable, OFTPausableInternal},
        rate_limiter::{RateLimiter, RateLimiterInternal, UNLIMITED_AMOUNT},
    },
    oft_types::{lock_unlock, mint_burn, OftType},
};
use common_macros::{contract_impl, storage, ttl_configurable, ttl_extendable, upgradeable};
use oapp_macros::oapp;
use oft_core::{
    assert_nonnegative_amount, impl_oft_lz_receive, utils as oft_utils, OFTCore, OFTError, OFTFeeDetail, OFTInternal,
    OFTLimit, OFTReceipt, SendParam,
};
use soroban_sdk::{assert_with_error, contract, vec, Address, Bytes, Env, Vec};

// =========================================================================
// Storage
// =========================================================================

#[storage]
enum OFTStorage {
    #[instance(OftType)]
    OftType,
}

// =========================================================================
// OFT Contract
// =========================================================================

#[contract]
#[ttl_configurable]
#[ttl_extendable]
#[upgradeable(no_migration, rbac)]
#[oapp(custom = [core])]
pub struct OFT;

// LzReceiveInternal implementation using default OFT receive logic
impl_oft_lz_receive!(OFT);

#[contract_impl]
impl OFT {
    pub fn __constructor(
        env: &Env,
        token: &Address,
        shared_decimals: u32,
        oft_type: OftType,
        endpoint: &Address,
        delegate: &Address,
        fee_deposit: &Address,
    ) {
        Self::__initialize_oft(env, token, shared_decimals, delegate, endpoint, delegate);
        OFTStorage::set_oft_type(env, &oft_type);
        Self::__set_fee_deposit(env, fee_deposit);
    }

    /// Returns the OFT type with its target address and configuration.
    pub fn oft_type(env: &Env) -> OftType {
        OFTStorage::oft_type(env).unwrap()
    }
}

/// OFTCore trait implementation for console OFT with extensions
#[contract_impl(contracttrait)]
impl OFTCore for OFT {
    fn quote_oft(env: &Env, _from: &Address, send_param: &SendParam) -> (OFTLimit, Vec<OFTFeeDetail>, OFTReceipt) {
        assert_nonnegative_amount(env, send_param);

        let dst_id = send_param.dst_eid as u128;

        // 1. Rate limit capacity, accounting for pause
        let mut max_amount_ld = if Self::is_paused(env, dst_id) {
            0
        } else {
            Self::get_rate_limit_usages(env, dst_id).outbound_available_amount
        };

        // 2. Back-calculate max sendable amount accounting for fees.
        // If the rate limit is unlimited, that means there is no rate limiting applied.
        if max_amount_ld != UNLIMITED_AMOUNT {
            max_amount_ld = Self::get_amount_before_fee(env, dst_id, max_amount_ld);
        }

        let oft_limit = OFTLimit { min_amount_ld: 0, max_amount_ld };

        // 3. Compute receipt
        let (amount_sent_ld, amount_received_ld) =
            Self::__debit_view(env, send_param.amount_ld, send_param.min_amount_ld, send_param.dst_eid);
        let oft_receipt = OFTReceipt { amount_sent_ld, amount_received_ld };

        // 4. Fee details
        let fee_details = if amount_sent_ld > amount_received_ld {
            vec![
                env,
                OFTFeeDetail {
                    fee_amount_ld: amount_sent_ld - amount_received_ld,
                    description: Bytes::from_slice(env, b"Fee"),
                },
            ]
        } else {
            vec![env]
        };

        (oft_limit, fee_details, oft_receipt)
    }
}

/// OFT behavior for Console OFT with extension hooks
impl OFTInternal for OFT {
    /// Calculates the amounts for a send without executing token operations.
    ///
    /// UI should ensure amounts are dust-free to avoid unintended fees.
    fn __debit_view(env: &Env, amount_ld: i128, min_amount_ld: i128, dst_eid: u32) -> (i128, i128) {
        let fee = Self::__get_fee(env, dst_eid as u128, amount_ld);
        let conversion_rate = Self::__decimal_conversion_rate(env);
        let amount_received_ld = oft_utils::remove_dust(amount_ld - fee, conversion_rate);

        assert_with_error!(env, amount_received_ld >= min_amount_ld, OFTError::SlippageExceeded);

        (amount_ld, amount_received_ld)
    }

    /// Executes the full send-side debit with all extension hooks.
    ///
    /// Flow: pause check → token debit → rate limit outflow → fee charge.
    fn __debit(env: &Env, sender: &Address, amount_ld: i128, min_amount_ld: i128, dst_eid: u32) -> (i128, i128) {
        Self::__assert_not_paused(env, dst_eid as u128);

        let (amount_sent_ld, amount_received_ld) = match Self::oft_type(env) {
            OftType::LockUnlock => {
                lock_unlock::debit::<Self>(env, &Self::token(env), sender, amount_ld, min_amount_ld, dst_eid)
            }
            OftType::MintBurn(_mintable) => {
                mint_burn::debit::<Self>(env, &Self::token(env), sender, amount_ld, min_amount_ld, dst_eid)
            }
        };

        Self::__outflow(env, dst_eid as u128, sender, amount_received_ld);

        let fee = amount_sent_ld - amount_received_ld;
        Self::__charge_fee(env, &Self::token(env), sender, fee);

        (amount_sent_ld, amount_received_ld)
    }

    /// Executes the full receive-side credit with extension hooks.
    ///
    /// Flow: rate limit inflow → token credit.
    ///
    /// No pause check on receive — this is intentional to prevent in-flight token lockups
    /// when a chain is paused mid-transfer. Matches EVM where `whenNotPaused` is only on
    /// `_debit`, not `_credit`.
    fn __credit(env: &Env, to: &Address, amount_ld: i128, src_eid: u32) -> i128 {
        Self::__inflow(env, src_eid as u128, to, amount_ld);

        match Self::oft_type(env) {
            OftType::LockUnlock => lock_unlock::credit::<Self>(env, &Self::token(env), to, amount_ld, src_eid),
            OftType::MintBurn(mintable) => mint_burn::credit::<Self>(env, &mintable, to, amount_ld, src_eid),
        }
    }
}

// =========================================================================
// Extension Trait Implementations
// =========================================================================

/// Pausable extension — per-Destination ID pause/unpause with separate PAUSER/UNPAUSER roles.
/// Default state: unpaused. Only enforced on send path (`__debit`), not receive (`__credit`).
#[contract_impl(contracttrait)]
impl OFTPausable for OFT {}
impl OFTPausableInternal for OFT {}

/// Fee extension — proportional fee on outbound transfers.
/// Default state: 0 BPS (no fee). Fees transferred to the fee deposit address.
#[contract_impl(contracttrait)]
impl OFTFee for OFT {}
impl OFTFeeInternal for OFT {}

/// Rate limiter extension — rolling window decay with net accounting.
/// Default state: closed (limits=0, outbound+inbound enabled). Admin must set limits to open.
#[contract_impl(contracttrait)]
impl RateLimiter for OFT {}
impl RateLimiterInternal for OFT {}

// Console-specific access control (ownership, auth, RBAC)
#[path = "oft_access_control.rs"]
mod oft_access_control;
