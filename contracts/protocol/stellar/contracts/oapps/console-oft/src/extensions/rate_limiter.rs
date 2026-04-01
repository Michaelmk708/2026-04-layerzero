use crate as oft;
use common_macros::{contract_error, contract_trait, only_role, storage};
use soroban_sdk::{assert_with_error, contractevent, contracttype, Address, Env};
use utils::rbac::RoleBasedAccessControl;

/// Role required for all rate limiter management operations.
pub const RATE_LIMITER_MANAGER_ROLE: &str = "RATE_LIMITER_MANAGER_ROLE";

/// ID of the default rate limit configuration.
pub const DEFAULT_ID: u128 = 0;

/// Sentinel value indicating unlimited capacity (no rate limiting applied).
pub const UNLIMITED_AMOUNT: i128 = i128::MAX;

// =========================================================================
// Types
// =========================================================================

/// Global configuration for the rate limiter.
///
/// - `use_global_state`: when true, all IDs share config and state from DEFAULT_ID (0)
/// - `is_globally_disabled`: when true, all rate limit checks are bypassed
#[contracttype]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RateLimitGlobalConfig {
    pub use_global_state: bool,
    pub is_globally_disabled: bool,
}

/// Per-Destination ID configuration: flags, limits, and windows.
///
/// Stored per Destination ID. If no config is stored for an ID, it inherits from DEFAULT_ID (0).
/// If DEFAULT_ID has no config either, `RateLimitConfig::default()` is used.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RateLimitConfig {
    /// Whether outbound (send) rate limiting is enabled
    pub outbound_enabled: bool,
    /// Whether inbound (receive) rate limiting is enabled
    pub inbound_enabled: bool,
    /// Whether inbound flow releases outbound capacity and vice versa
    pub net_accounting_enabled: bool,
    /// Whether per-address exemptions are checked for this Destination ID
    pub address_exemption_enabled: bool,
    /// Maximum outbound (send) capacity in outbound window period
    pub outbound_limit: i128,
    /// Maximum inbound (receive) capacity in inbound window period
    pub inbound_limit: i128,
    /// Time window (in seconds) over which outbound usage fully decays back to zero
    pub outbound_window: u64,
    /// Time window (in seconds) over which inbound usage fully decays back to zero
    pub inbound_window: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            outbound_enabled: true,
            inbound_enabled: true,
            net_accounting_enabled: true,
            address_exemption_enabled: false,
            outbound_limit: 0,
            inbound_limit: 0,
            outbound_window: 0,
            inbound_window: 0,
        }
    }
}

/// Mutable usage counters and timestamp for a rate-limited Destination ID.
#[contracttype]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RateLimitState {
    pub outbound_usage: i128,
    pub inbound_usage: i128,
    pub last_updated: u64,
}

/// Decayed usage and available capacity for a single Destination ID.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RateLimitUsages {
    pub outbound_usage: i128,
    pub outbound_available_amount: i128,
    pub inbound_usage: i128,
    pub inbound_available_amount: i128,
}

// =========================================================================
// Storage
// =========================================================================

#[storage]
enum RateLimitStorage {
    #[instance(RateLimitGlobalConfig)]
    #[default(Default::default())]
    GlobalConfig,

    #[persistent(RateLimitConfig)]
    RateLimitConfigs { id: u128 },

    #[persistent(RateLimitState)]
    #[default(Default::default())]
    RateLimitStates { id: u128 },

    #[persistent(bool)]
    #[default(false)]
    AddressExemptions { user: Address },
}

// =========================================================================
// Errors
// =========================================================================

#[contract_error]
pub enum RateLimitError {
    ExemptionStateIdempotent = 3120,
    InboundLimitNegative,
    InboundUsageNegative,
    InvalidAmount,
    LastUpdatedInFuture,
    OutboundLimitNegative,
    OutboundUsageNegative,
    RateLimitExceeded,
}

// =========================================================================
// Events
// =========================================================================

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RateLimitConfigUpdated {
    #[topic]
    pub id: u128,
    pub config: Option<RateLimitConfig>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RateLimitStateUpdated {
    #[topic]
    pub id: u128,
    pub state: RateLimitState,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RateLimitGlobalConfigUpdated {
    pub global_config: Option<RateLimitGlobalConfig>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RateLimitExemptionUpdated {
    #[topic]
    pub user: Address,
    pub is_exempt: bool,
}

// =========================================================================
// Public Trait — mirrors EVM RateLimiterRBACUpgradeable
// =========================================================================

/// Rate limiter extension — controls token flow per Destination ID with rolling window decay.
///
/// **Default state:** Closed (outbound+inbound enabled, limits=0). All transfers are blocked
/// until the admin explicitly sets non-zero limits via `set_rate_limit_config`.
///
/// **Config resolution:** Each Destination ID can override the DEFAULT_ID (0) config, or
/// inherit it. When `use_global_state` is true, all Destination IDs share config and state from DEFAULT_ID.
///
/// **Decay model:** Usage decays linearly over the configured window:
///   `current_usage = max(0, in_flight - limit * time_elapsed / window)`
///   `available = max(0, limit - current_usage)`
///
/// **Net accounting:** When enabled, inbound flow releases outbound capacity and vice versa.
#[contract_trait]
pub trait RateLimiter: RateLimiterInternal + RoleBasedAccessControl {
    // === Management (RBAC-guarded) ===

    /// Sets or removes the global config (use_global_state, is_globally_disabled).
    ///
    /// - `Some(config)` — stores global config.
    /// - `None` — removes global config (reverts to default: both flags `false`).
    #[only_role(operator, RATE_LIMITER_MANAGER_ROLE)]
    fn set_rate_limit_global_config(
        env: &soroban_sdk::Env,
        global_config: &Option<oft::rate_limiter::RateLimitGlobalConfig>,
        operator: &soroban_sdk::Address,
    ) {
        Self::__set_rate_limit_global_config(env, global_config);
    }

    /// Sets or removes rate limit configuration for a Destination ID.
    ///
    /// - `Some(config)` — validates and stores config for this Destination ID.
    /// - `None` — removes per-Destination ID config (falls back to DEFAULT_ID).
    #[only_role(operator, RATE_LIMITER_MANAGER_ROLE)]
    fn set_rate_limit_config(
        env: &soroban_sdk::Env,
        id: u128,
        config: &Option<oft::rate_limiter::RateLimitConfig>,
        operator: &soroban_sdk::Address,
    ) {
        Self::__set_rate_limit_config(env, id, config);
    }

    /// Directly sets rate limit state (usage, last_updated) for admin correction.
    #[only_role(operator, RATE_LIMITER_MANAGER_ROLE)]
    fn set_rate_limit_state(
        env: &soroban_sdk::Env,
        id: u128,
        state: &oft::rate_limiter::RateLimitState,
        operator: &soroban_sdk::Address,
    ) {
        Self::__set_rate_limit_state(env, id, state);
    }

    /// Sets per-address exemption from rate limiting.
    ///
    /// Exempt addresses bypass rate limit checks entirely (when `address_exemption_enabled`
    /// is true in the config).
    #[only_role(operator, RATE_LIMITER_MANAGER_ROLE)]
    fn set_rate_limit_exemption(
        env: &soroban_sdk::Env,
        user: &soroban_sdk::Address,
        is_exempt: bool,
        operator: &soroban_sdk::Address,
    ) {
        Self::__set_rate_limit_exemption(env, user, is_exempt);
    }

    /// Snapshots decayed usage to storage for the given Destination ID.
    #[only_role(operator, RATE_LIMITER_MANAGER_ROLE)]
    fn checkpoint_rate_limit(env: &soroban_sdk::Env, id: u128, operator: &soroban_sdk::Address) {
        Self::__checkpoint_rate_limit(env, id);
    }

    // === Views ===

    /// Returns the global rate limit configuration.
    fn get_rate_limit_global_config(env: &soroban_sdk::Env) -> oft::rate_limiter::RateLimitGlobalConfig {
        RateLimitStorage::global_config(env)
    }

    /// Returns the raw per-Destination ID state and config (not resolved through inheritance).
    ///
    /// Config is `None` when no per-Destination ID config is stored.
    fn rate_limits(
        env: &soroban_sdk::Env,
        id: u128,
    ) -> (oft::rate_limiter::RateLimitState, Option<oft::rate_limiter::RateLimitConfig>) {
        (RateLimitStorage::rate_limit_states(env, id), RateLimitStorage::rate_limit_configs(env, id))
    }

    /// Returns whether the given address is exempt from rate limiting.
    fn is_rate_limit_exemption(env: &soroban_sdk::Env, user: &soroban_sdk::Address) -> bool {
        RateLimitStorage::address_exemptions(env, user)
    }

    /// Returns the current decayed usage and available capacity for the given Destination ID.
    ///
    /// If globally disabled, returns `UNLIMITED_AMOUNT` for available amounts.
    /// If a direction is not enabled, returns `UNLIMITED_AMOUNT` for that direction's available amount.
    fn get_rate_limit_usages(env: &soroban_sdk::Env, id: u128) -> oft::rate_limiter::RateLimitUsages {
        let (_state_id, state, config) = Self::__get_rate_limit_state_and_config(env, id);

        let mut usages = Self::__get_rate_limit_usages(env, &config, &state);
        let global = RateLimitStorage::global_config(env);
        if global.is_globally_disabled {
            usages.outbound_available_amount = UNLIMITED_AMOUNT;
            usages.inbound_available_amount = UNLIMITED_AMOUNT;
        } else {
            if !config.outbound_enabled {
                usages.outbound_available_amount = UNLIMITED_AMOUNT;
            }
            if !config.inbound_enabled {
                usages.inbound_available_amount = UNLIMITED_AMOUNT;
            }
        }

        usages
    }
}

// =========================================================================
// Internal Trait — mirrors EVM RateLimiterBaseUpgradeable internals
// =========================================================================

/// Internal trait for rate limiter operations.
///
/// Provides the flow hooks (`__outflow` / `__inflow`) called by `OFTInternal::__debit` and
/// `OFTInternal::__credit`, as well as the core decay calculation and config resolution.
pub trait RateLimiterInternal {
    // === OFT Hooks ===

    /// Hook called from `__debit` — consumes outbound capacity and releases inbound (if net accounting).
    fn __outflow(env: &Env, id: u128, from: &Address, amount: i128) {
        Self::__apply_rate_limit(env, id, from, amount, true);
    }

    /// Hook called from `__credit` — consumes inbound capacity and releases outbound (if net accounting).
    fn __inflow(env: &Env, id: u128, to: &Address, amount: i128) {
        Self::__apply_rate_limit(env, id, to, amount, false);
    }

    // === Core Logic ===

    /// Applies rate limit for a single flow direction.
    fn __apply_rate_limit(env: &Env, id: u128, user: &Address, amount: i128, is_outflow: bool) {
        assert_with_error!(env, amount >= 0, RateLimitError::InvalidAmount);

        if RateLimitStorage::global_config(env).is_globally_disabled {
            return;
        }

        let (state_id, mut state, config) = Self::__get_rate_limit_state_and_config(env, id);

        let (forward_enabled, backward_enabled) = if is_outflow {
            (config.outbound_enabled, config.inbound_enabled)
        } else {
            (config.inbound_enabled, config.outbound_enabled)
        };

        if (!forward_enabled && (!backward_enabled || !config.net_accounting_enabled))
            || (config.address_exemption_enabled && RateLimitStorage::has_address_exemptions(env, user))
        {
            return;
        }

        let usages = Self::__get_rate_limit_usages(env, &config, &state);

        let (mut forward_usage, forward_available, mut backward_usage) = if is_outflow {
            (usages.outbound_usage, usages.outbound_available_amount, usages.inbound_usage)
        } else {
            (usages.inbound_usage, usages.inbound_available_amount, usages.outbound_usage)
        };

        if forward_enabled {
            assert_with_error!(env, amount <= forward_available, RateLimitError::RateLimitExceeded);
            forward_usage += amount;
        }

        if backward_enabled && config.net_accounting_enabled {
            backward_usage = (backward_usage - amount).max(0);
        }

        let (new_outbound_usage, new_inbound_usage) =
            if is_outflow { (forward_usage, backward_usage) } else { (backward_usage, forward_usage) };

        state.outbound_usage = new_outbound_usage;
        state.inbound_usage = new_inbound_usage;
        state.last_updated = env.ledger().timestamp();

        RateLimitStorage::set_rate_limit_states(env, state_id, &state);
    }

    /// Resolves the state and config for a given Destination ID.
    ///
    /// Returns `(state_id, state, config)` where:
    /// - `state_id`: Effective storage key (may differ from `id` when `use_global_state` is true)
    /// - `state`: Current state from storage (returned by value; caller should copy to a `mut` local)
    /// - `config`: Resolved config (per-Destination ID override, or inherited from DEFAULT_ID)
    ///
    /// Resolution rules:
    /// - If `use_global_state`: all Destination IDs share DEFAULT_ID state and config
    /// - If per-Destination ID config exists: use it
    /// - Otherwise: inherit config from DEFAULT_ID (or default if DEFAULT_ID config absent)
    fn __get_rate_limit_state_and_config(env: &Env, id: u128) -> (u128, RateLimitState, RateLimitConfig) {
        let effective_id = if RateLimitStorage::global_config(env).use_global_state { DEFAULT_ID } else { id };
        let state = RateLimitStorage::rate_limit_states(env, effective_id);
        let config = Self::__effective_config(env, effective_id);
        (effective_id, state, config)
    }

    /// Resolves the effective config for a given Destination ID.
    ///
    /// If a per-Destination ID config exists, uses it.
    /// Otherwise falls back to DEFAULT_ID config, or `RateLimitConfig::default()` if absent.
    fn __effective_config(env: &Env, id: u128) -> RateLimitConfig {
        RateLimitStorage::rate_limit_configs(env, id)
            .unwrap_or_else(|| RateLimitStorage::rate_limit_configs(env, DEFAULT_ID).unwrap_or_default())
    }

    // === Decay Calculation ===

    /// Calculates decayed outbound and inbound usages from the resolved config + state.
    fn __get_rate_limit_usages(env: &Env, config: &RateLimitConfig, state: &RateLimitState) -> RateLimitUsages {
        let (outbound_usage, outbound_available_amount) = Self::__get_rate_limit_usage(
            env,
            state.last_updated,
            state.outbound_usage,
            config.outbound_limit,
            config.outbound_window,
        );
        let (inbound_usage, inbound_available_amount) = Self::__get_rate_limit_usage(
            env,
            state.last_updated,
            state.inbound_usage,
            config.inbound_limit,
            config.inbound_window,
        );
        RateLimitUsages { outbound_usage, outbound_available_amount, inbound_usage, inbound_available_amount }
    }

    /// Calculates the decayed usage and available capacity for a single direction.
    ///
    /// Decay formula: `decay = limit * time_elapsed / window` (pro-rata, saturating on overflow).
    /// A window of 0 is treated as 1 to avoid division by zero.
    fn __get_rate_limit_usage(
        env: &Env,
        last_updated: u64,
        amount_in_flight: i128,
        limit: i128,
        window: u64,
    ) -> (i128, i128) {
        let now = env.ledger().timestamp();
        assert_with_error!(env, now >= last_updated, RateLimitError::LastUpdatedInFuture);
        let time_since_last_update = (now - last_updated) as i128;
        let effective_window = if window == 0 { 1i128 } else { window as i128 };

        let decay = limit.saturating_mul(time_since_last_update) / effective_window;
        let current_usage = (amount_in_flight - decay).max(0);
        let available_amount = (limit - current_usage).max(0);
        (current_usage, available_amount)
    }

    // === Checkpoint ===

    fn __checkpoint_rate_limit(env: &Env, id: u128) {
        let (state_id, mut state, config) = Self::__get_rate_limit_state_and_config(env, id);
        let usages = Self::__get_rate_limit_usages(env, &config, &state);

        state.outbound_usage = usages.outbound_usage;
        state.inbound_usage = usages.inbound_usage;
        state.last_updated = env.ledger().timestamp();

        RateLimitStorage::set_rate_limit_states(env, state_id, &state);
    }

    // === Internal Setters ===

    fn __set_rate_limit_global_config(env: &Env, global_config: &Option<RateLimitGlobalConfig>) {
        RateLimitStorage::set_or_remove_global_config(env, global_config);
        RateLimitGlobalConfigUpdated { global_config: global_config.clone() }.publish(env);
    }

    fn __set_rate_limit_config(env: &Env, id: u128, config: &Option<RateLimitConfig>) {
        if let Some(cfg) = config {
            assert_with_error!(env, cfg.outbound_limit >= 0, RateLimitError::OutboundLimitNegative);
            assert_with_error!(env, cfg.inbound_limit >= 0, RateLimitError::InboundLimitNegative);
        }
        RateLimitStorage::set_or_remove_rate_limit_configs(env, id, config);
        RateLimitConfigUpdated { id, config: config.clone() }.publish(env);
    }

    fn __set_rate_limit_state(env: &Env, id: u128, state: &RateLimitState) {
        assert_with_error!(env, state.last_updated <= env.ledger().timestamp(), RateLimitError::LastUpdatedInFuture);
        assert_with_error!(env, state.outbound_usage >= 0, RateLimitError::OutboundUsageNegative);
        assert_with_error!(env, state.inbound_usage >= 0, RateLimitError::InboundUsageNegative);

        RateLimitStorage::set_rate_limit_states(env, id, state);
        RateLimitStateUpdated { id, state: state.clone() }.publish(env);
    }

    fn __set_rate_limit_exemption(env: &Env, user: &Address, is_exempt: bool) {
        let current = RateLimitStorage::address_exemptions(env, user);
        assert_with_error!(env, current != is_exempt, RateLimitError::ExemptionStateIdempotent);

        if is_exempt {
            RateLimitStorage::set_address_exemptions(env, user, &true);
        } else {
            RateLimitStorage::remove_address_exemptions(env, user);
        }
        RateLimitExemptionUpdated { user: user.clone(), is_exempt }.publish(env);
    }
}
