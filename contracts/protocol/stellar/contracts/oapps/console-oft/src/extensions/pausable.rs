use common_macros::{contract_error, contract_trait, storage};
use soroban_sdk::{assert_with_error, contractevent, Env, Symbol};
use utils::rbac::{ensure_role, RoleBasedAccessControl};

/// Role for pausing the contract.
pub const PAUSER_ROLE: &str = "PAUSER_ROLE";

/// Role for unpausing the contract.
pub const UNPAUSER_ROLE: &str = "UNPAUSER_ROLE";

// =========================================================================
// Storage
// =========================================================================

#[storage]
pub enum OFTPausableStorage {
    #[instance(bool)]
    #[default(false)]
    DefaultPaused,

    #[persistent(bool)]
    PauseConfigs { id: u128 },
}

// =========================================================================
// Errors
// =========================================================================

#[contract_error]
pub enum OFTPausableError {
    Paused = 3110,
    PauseStateIdempotent,
}

// =========================================================================
// Events
// =========================================================================

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DefaultPauseSet {
    pub paused: bool,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseSet {
    pub id: u128,
    pub paused: Option<bool>,
}

// =========================================================================
// Trait With Default Implementations
// =========================================================================

#[contract_trait]
pub trait OFTPausable: OFTPausableInternal + RoleBasedAccessControl {
    // =========================================================================
    // Management Functions
    // =========================================================================

    /// Sets the global default pause state.
    ///
    /// Destinations without a per-Destination ID override will inherit this default.
    /// Requires `PAUSER_ROLE` if pausing, `UNPAUSER_ROLE` if unpausing.
    ///
    /// # Arguments
    /// * `paused` - `true` to pause, `false` to unpause
    /// * `operator` - The address that must have PAUSER_ROLE or UNPAUSER_ROLE
    fn set_default_paused(env: &soroban_sdk::Env, paused: bool, operator: &soroban_sdk::Address) {
        operator.require_auth();
        match paused {
            true => ensure_role::<Self>(env, &Symbol::new(env, PAUSER_ROLE), operator),
            false => ensure_role::<Self>(env, &Symbol::new(env, UNPAUSER_ROLE), operator),
        };
        Self::__set_default_paused(env, paused);
    }

    /// Sets the paused state for a single Destination ID.
    ///
    /// - `Some(true)` — set per-Destination ID override to paused
    /// - `Some(false)` — set per-Destination ID override to unpaused
    /// - `None` — remove the per-Destination ID override, fall back to global default
    ///
    /// Caller must have `PAUSER_ROLE` if the effective state resolves to paused,
    /// or `UNPAUSER_ROLE` if it resolves to unpaused.
    ///
    /// # Arguments
    /// * `id` - The Destination ID
    /// * `paused` - The pause override, or None to remove the override
    /// * `operator` - The address that must have the required role(s)
    fn set_paused(env: &soroban_sdk::Env, id: u128, paused: Option<bool>, operator: &soroban_sdk::Address) {
        operator.require_auth();

        let default_paused = Self::default_paused(env);
        let effective_paused = paused.unwrap_or(default_paused);

        if effective_paused {
            ensure_role::<Self>(env, &Symbol::new(env, PAUSER_ROLE), operator);
        } else {
            ensure_role::<Self>(env, &Symbol::new(env, UNPAUSER_ROLE), operator);
        }

        Self::__set_paused(env, id, paused);
    }

    // =========================================================================
    // View Functions
    // =========================================================================

    /// Returns the effective paused state for the given Destination ID.
    ///
    /// Resolves per-Destination ID override if set, otherwise returns the global default.
    fn is_paused(env: &soroban_sdk::Env, id: u128) -> bool {
        Self::__is_paused(env, id)
    }

    /// Returns the global default pause state (used when no per-Destination ID override is set).
    fn default_paused(env: &soroban_sdk::Env) -> bool {
        OFTPausableStorage::default_paused(env)
    }

    /// Returns the raw per-Destination ID pause override, or `None` if no override is set.
    fn pause_config(env: &soroban_sdk::Env, id: u128) -> Option<bool> {
        OFTPausableStorage::pause_configs(env, id)
    }
}

/// Internal trait for pausable operations used by OFT hooks.
/// Contains only truly internal methods that are called from OFTPausable implementations.
pub trait OFTPausableInternal {
    // =========================================================================
    // OFT Hooks
    // =========================================================================

    /// Asserts that the OFT is not paused for the given Destination ID, panics otherwise.
    /// Used internally by `__debit` (send path) to enforce pause state.
    ///
    /// # Errors
    /// * `Paused` - If the OFT is currently paused for the given Destination ID.
    fn __assert_not_paused(env: &Env, id: u128) {
        assert_with_error!(env, !Self::__is_paused(env, id), OFTPausableError::Paused);
    }

    // =========================================================================
    // Management Functions
    // =========================================================================

    /// Sets the global default pause state. Reverts if already in the requested state.
    ///
    /// # Arguments
    /// * `paused` - `true` to pause, `false` to unpause
    fn __set_default_paused(env: &Env, paused: bool) {
        assert_with_error!(
            env,
            OFTPausableStorage::default_paused(env) != paused,
            OFTPausableError::PauseStateIdempotent
        );
        if paused {
            OFTPausableStorage::set_default_paused(env, &true);
        } else {
            OFTPausableStorage::remove_default_paused(env);
        }
        DefaultPauseSet { paused }.publish(env);
    }

    /// Sets the paused state for a single Destination ID.
    ///
    /// # Arguments
    /// * `id` - The Destination ID
    /// * `paused` - The pause override, or None to remove the override
    fn __set_paused(env: &Env, id: u128, paused: Option<bool>) {
        OFTPausableStorage::set_or_remove_pause_configs(env, id, &paused);
        PauseSet { id, paused }.publish(env);
    }

    // =========================================================================
    // View Functions
    // =========================================================================

    /// Returns the effective paused state for the given Destination ID (per-Destination ID override or global default).
    fn __is_paused(env: &Env, id: u128) -> bool {
        OFTPausableStorage::pause_configs(env, id).unwrap_or_else(|| OFTPausableStorage::default_paused(env))
    }
}
