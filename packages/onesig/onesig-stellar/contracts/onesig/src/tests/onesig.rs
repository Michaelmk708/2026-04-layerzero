use super::helpers::{
    assert_latest_auth, create_onesig_for_onesig_tests, create_onesig_with_defaults,
    generate_ed25519_keypair, generate_secp256k1_keypair, generate_signer, new_executor_key,
    secp256k1_sign, secp256k1_signer_address,
};
use crate::{
    eip712::build_eip712_digest,
    errors::OneSigError,
    interfaces::{Call, Sender, SenderKey, Transaction, TransactionAuthData},
    onesig::OneSigClient,
};
use soroban_sdk::{
    auth::{Context, ContractExecutable, CreateContractHostFnContext},
    testutils::{Address as _, Events, Ledger},
    vec, Address, BytesN, Env, IntoVal, Map, Symbol, Val, Vec,
};
use utils::errors::MultiSigError;

fn setup<'a>() -> (Env, Address, OneSigClient<'a>) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = create_onesig_for_onesig_tests(&env);
    let client = OneSigClient::new(&env, &contract_id);

    (env, contract_id, client)
}

#[test]
fn test_set_seed() {
    let (env, _contract_id, client) = setup();

    let seed = BytesN::from_array(&env, &[1u8; 32]);

    // Set seed via contract self-call (mocked auth)
    client.set_seed(&seed);

    // Verify seed was set
    let retrieved_seed = client.seed();
    assert_eq!(retrieved_seed, seed);
}

#[test]
fn test_seed_set_event() {
    let (env, contract_id, client) = setup();

    let seed = BytesN::from_array(&env, &[1u8; 32]);

    // Set seed
    client.set_seed(&seed);

    // Verify SeedSet event was emitted
    assert_eq!(
        env.events().all(),
        vec![
            &env,
            (
                contract_id.clone(),
                (Symbol::new(&env, "seed_set"),).into_val(&env),
                Map::<Symbol, Val>::from_array(
                    &env,
                    [(Symbol::new(&env, "seed"), seed.into_val(&env)),]
                )
                .into_val(&env),
            ),
        ]
    );
}

#[test]
fn test_set_seed_auth_verification() {
    let (env, contract_id, client) = setup();

    let seed = BytesN::from_array(&env, &[2u8; 32]);

    client.set_seed(&seed);

    assert_latest_auth(&env, &contract_id, "set_seed", (&seed,).into_val(&env));
}

#[test]
fn test_seed_getter() {
    let (env, _contract_id, client) = setup();

    // Initially seed should be zero
    let initial_seed = client.seed();
    assert_eq!(initial_seed, BytesN::from_array(&env, &[0u8; 32]));

    // Set a seed
    let seed = BytesN::from_array(&env, &[0x42u8; 32]);
    client.set_seed(&seed);

    // Verify getter returns the set seed
    let retrieved_seed = client.seed();
    assert_eq!(retrieved_seed, seed);
}

#[test]
fn test_encode_leaf() {
    let (env, _contract_id, client) = setup();

    // Set onesig_id for consistent encoding
    // Note: onesig_id is stored, but we need to set it via storage directly in tests
    // For now, test with default value (0)

    let nonce = 1u64;
    let target_contract = Address::generate(&env);
    let func = Symbol::new(&env, "test_func");
    let args = vec![&env];

    let call = Call {
        to: target_contract.clone(),
        func: func.clone(),
        args: args.clone(),
    };

    // Encode leaf - should not panic
    let leaf = client.encode_leaf(&nonce, &call);

    // Leaf should be 32 bytes
    assert_eq!(leaf.len(), 32);

    // Encoding same data should produce same leaf
    let leaf2 = client.encode_leaf(&nonce, &call);
    assert_eq!(leaf, leaf2);

    // Different nonce should produce different leaf
    let leaf3 = client.encode_leaf(&2u64, &call);
    assert_ne!(leaf, leaf3);

    // Different call should produce different leaf
    let different_call = Call {
        to: Address::generate(&env),
        func: Symbol::new(&env, "different_func"),
        args: vec![&env],
    };
    let leaf4 = client.encode_leaf(&nonce, &different_call);
    assert_ne!(leaf, leaf4);
}

#[test]
fn test_encode_leaf_with_different_functions() {
    let (env, _contract_id, client) = setup();

    let nonce = 0u64;
    let call1 = Call {
        to: Address::generate(&env),
        func: Symbol::new(&env, "func1"),
        args: vec![&env],
    };
    let call2 = Call {
        to: Address::generate(&env),
        func: Symbol::new(&env, "func2"),
        args: vec![&env],
    };

    let leaf1 = client.encode_leaf(&nonce, &call1);
    let leaf2 = client.encode_leaf(&nonce, &call2);
    assert_eq!(leaf1.len(), 32);
    assert_eq!(leaf2.len(), 32);
    // Different calls should produce different leaves
    assert_ne!(leaf1, leaf2);
}

#[test]
fn test_can_execute_transaction_permissionless() {
    let (env, _contract_id, client) = setup();

    // Default should be permissionless (executor_required = false)
    // Any sender should be able to execute when executor_required is false
    let sender_key = new_executor_key(&env);
    let sender = SenderKey::Executor(sender_key);
    assert!(client.can_execute_transaction(&sender));
}

#[test]
fn test_can_execute_transaction_executor_required_with_executor() {
    let (env, contract_id, client) = setup();

    let executor_client = crate::interfaces::ExecutorClient::new(&env, &contract_id);

    // Add an executor first
    let executor_key = new_executor_key(&env);
    executor_client.set_executor(&executor_key, &true);

    // Now set executor_required to true
    executor_client.set_executor_required(&true);

    // Executor should be able to execute
    let executor_sender = SenderKey::Executor(executor_key.clone());
    assert!(client.can_execute_transaction(&executor_sender));

    // Non-executor should not be able to execute
    let non_executor_key = new_executor_key(&env);
    let non_executor_sender = SenderKey::Executor(non_executor_key);
    assert!(!client.can_execute_transaction(&non_executor_sender));
}

#[test]
fn test_can_execute_transaction_executor_required_with_signer() {
    let (env, contract_id, client) = setup();

    let executor_client = crate::interfaces::ExecutorClient::new(&env, &contract_id);

    // Add a signer (signers are stored as BytesN<20>)
    let signer_bytes = generate_signer(&env, 1);
    client.set_signer(&signer_bytes, &true);

    // Add an executor first
    let executor_key = new_executor_key(&env);
    executor_client.set_executor(&executor_key, &true);

    // Now set executor_required to true
    executor_client.set_executor_required(&true);

    // Executor should be able to execute
    let executor_sender = SenderKey::Executor(executor_key.clone());
    assert!(client.can_execute_transaction(&executor_sender));

    // Registered signer should also be able to execute
    let signer_sender = SenderKey::Signer(signer_bytes.clone());
    assert!(client.can_execute_transaction(&signer_sender));

    // Unregistered signer should be rejected
    let non_signer_bytes = generate_signer(&env, 99);
    let non_signer_sender = SenderKey::Signer(non_signer_bytes);
    assert!(!client.can_execute_transaction(&non_signer_sender));
}

#[test]
fn test_onesig_id_getter() {
    let (_env, _contract_id, client) = setup();

    // Initially should return 0
    let onesig_id = client.onesig_id();
    assert_eq!(onesig_id, 0);
}

#[test]
fn test_address_to_bytes32_contract_address() {
    let (env, contract_id, client) = setup();

    // Test that encode_leaf works with contract addresses
    // This indirectly tests address_to_bytes32 which now uses strkey decoding
    let nonce = 0u64;
    // Use the contract_id from setup - this is a contract address
    let call = Call {
        to: contract_id,
        func: Symbol::new(&env, "test"),
        args: vec![&env],
    };

    // Should not panic - contract address should be handled correctly via strkey decoding
    let leaf = client.encode_leaf(&nonce, &call);
    assert_eq!(leaf.len(), 32);

    // Verify that the same contract address produces the same leaf
    let leaf2 = client.encode_leaf(&nonce, &call);
    assert_eq!(leaf, leaf2);
}

#[test]
fn test_address_to_bytes32_with_different_contracts() {
    let (env, contract_id, client) = setup();

    // Test that different contract addresses produce different leaves
    let nonce = 0u64;

    let contract1 = contract_id;
    let contract2 = Address::generate(&env);

    let call1 = Call {
        to: contract1,
        func: Symbol::new(&env, "test"),
        args: vec![&env],
    };

    let call2 = Call {
        to: contract2,
        func: Symbol::new(&env, "test"),
        args: vec![&env],
    };

    let leaf1 = client.encode_leaf(&nonce, &call1);
    let leaf2 = client.encode_leaf(&nonce, &call2);

    // Different contract addresses should produce different leaves
    assert_ne!(leaf1, leaf2);
}

#[test]
fn test_verify_transaction_proof_invalid_proof() {
    let (env, _contract_id, client) = setup();

    let _nonce = 0u64;
    let call = Call {
        to: Address::generate(&env),
        func: Symbol::new(&env, "dummy"),
        args: vec![&env],
    };
    let invalid_proof = vec![&env, BytesN::from_array(&env, &[0u8; 32])];
    let invalid_root = BytesN::from_array(&env, &[1u8; 32]);

    let transaction = Transaction {
        call,
        proof: invalid_proof,
    };

    // Should return InvalidProofOrNonce error
    let res = client.try_verify_transaction_proof(&invalid_root, &transaction);
    assert_eq!(
        res.err().unwrap().ok().unwrap(),
        OneSigError::InvalidProofOrNonce.into()
    );
}

#[test]
fn test_verify_merkle_root_expired() {
    let (env, _contract_id, client) = setup();

    let merkle_root = BytesN::from_array(&env, &[1u8; 32]);
    let expiry = 0u64; // Expired (timestamp 0)
    let signatures = vec![&env];

    // Set a seed first
    let seed = BytesN::from_array(&env, &[1u8; 32]);
    client.set_seed(&seed);

    // Advance ledger timestamp to make expiry invalid
    env.ledger().set_timestamp(1000);

    let res = client.try_verify_merkle_root(&merkle_root, &expiry, &signatures);
    assert_eq!(
        res.err().unwrap().ok().unwrap(),
        OneSigError::MerkleRootExpired.into()
    );
}

#[test]
fn test_verify_merkle_root_not_expired_but_invalid_signatures() {
    let (env, _contract_id, client) = setup();

    // Set seed
    let seed = BytesN::from_array(&env, &[1u8; 32]);
    client.set_seed(&seed);

    // Register one signer, but provide a valid-format signature from a different signer key.
    // This should fail with SignerNotFound rather than panicking.
    let registered_signer_key = generate_secp256k1_keypair();
    let registered_signer_address = secp256k1_signer_address(&env, &registered_signer_key);
    client.set_signer(&registered_signer_address, &true);
    client.set_threshold(&1);

    let unregistered_signer_key = generate_secp256k1_keypair();

    let merkle_root = BytesN::from_array(&env, &[1u8; 32]);
    let expiry = 9999u64; // Future timestamp

    // Set ledger timestamp to current time
    env.ledger().set_timestamp(1000);

    let digest = build_eip712_digest(&env, &seed, &merkle_root, expiry);
    let invalid_signature = secp256k1_sign(&env, &unregistered_signer_key, &digest);
    let signatures = vec![&env, invalid_signature];

    let res = client.try_verify_merkle_root(&merkle_root, &expiry, &signatures);
    assert_eq!(
        res.err().unwrap().ok().unwrap(),
        MultiSigError::SignerNotFound.into()
    );
}

#[test]
fn test_verify_transaction_proof_wrong_nonce() {
    let (env, _contract_id, client) = setup();

    // Current nonce is 0, but we'll try to verify with nonce 1
    // This should fail because verify_transaction_proof uses current_nonce
    let call = Call {
        to: Address::generate(&env),
        func: Symbol::new(&env, "dummy"),
        args: vec![&env],
    };
    // Create a proof that would work for nonce 1, but current nonce is 0
    let proof = vec![&env, BytesN::from_array(&env, &[0u8; 32])];
    let root = BytesN::from_array(&env, &[1u8; 32]);

    let transaction = Transaction { call, proof };

    // Note: This test verifies that verify_transaction_proof uses current_nonce
    // and doesn't allow skipping nonces
    let res = client.try_verify_transaction_proof(&root, &transaction);
    assert_eq!(
        res.err().unwrap().ok().unwrap(),
        OneSigError::InvalidProofOrNonce.into()
    );
}

#[test]
fn test_verify_transaction_proof_correct_calls_wrong_proof() {
    let (env, _contract_id, client) = setup();

    // Create valid call
    let call = Call {
        to: Address::generate(&env),
        func: Symbol::new(&env, "test"),
        args: vec![&env],
    };

    // Create a proof that doesn't match the call
    let wrong_proof = vec![&env, BytesN::from_array(&env, &[0xFFu8; 32])];
    let root = BytesN::from_array(&env, &[1u8; 32]);

    let transaction = Transaction {
        call,
        proof: wrong_proof,
    };

    // Should return InvalidProofOrNonce because proof doesn't match
    let res = client.try_verify_transaction_proof(&root, &transaction);
    assert_eq!(
        res.err().unwrap().ok().unwrap(),
        OneSigError::InvalidProofOrNonce.into()
    );
}

#[test]
fn test_verify_merkle_root_insufficient_signatures() {
    let (env, _contract_id, client) = setup();

    // Set seed
    let seed = BytesN::from_array(&env, &[1u8; 32]);
    client.set_seed(&seed);

    // Set up signers and threshold = 2
    let signer1 = generate_signer(&env, 1);
    let signer2 = generate_signer(&env, 2);
    client.set_signer(&signer1, &true);
    client.set_signer(&signer2, &true);
    client.set_threshold(&2);

    let merkle_root = BytesN::from_array(&env, &[1u8; 32]);
    let expiry = 9999u64; // Future timestamp

    // Only provide 1 signature when threshold is 2
    // The signature recovery will fail, causing SignatureError
    let mut signatures = vec![&env];
    let sig_bytes = [0u8; 65];
    signatures.push_back(BytesN::from_array(&env, &sig_bytes));

    // Set ledger timestamp to current time
    env.ledger().set_timestamp(1000);

    // Signature recovery fails (invalid format)
    let res = client.try_verify_merkle_root(&merkle_root, &expiry, &signatures);
    assert_eq!(
        res.err().unwrap().ok().unwrap(),
        MultiSigError::SignatureError.into()
    );
}

// Panics with Crypto::InvalidInput (host error) because signature format is invalid
#[test]
#[should_panic]
fn test_verify_merkle_root_invalid_signature_format() {
    let (env, _contract_id, client) = setup();

    // Set seed
    let seed = BytesN::from_array(&env, &[1u8; 32]);
    client.set_seed(&seed);

    // Set up signers and threshold = 1
    let signer1 = generate_signer(&env, 1);
    client.set_signer(&signer1, &true);
    client.set_threshold(&1);

    let merkle_root = BytesN::from_array(&env, &[1u8; 32]);
    let expiry = 9999u64; // Future timestamp

    // Create an invalid signature format (all 0xFF)
    // This will fail at the crypto level before signature recovery
    let mut signatures = vec![&env];
    let sig_bytes = [0xFFu8; 65];
    signatures.push_back(BytesN::from_array(&env, &sig_bytes));

    // Set ledger timestamp to current time
    env.ledger().set_timestamp(1000);

    client.verify_merkle_root(&merkle_root, &expiry, &signatures);
}

#[test]
fn test_encode_leaf_with_empty_args() {
    let (env, _contract_id, client) = setup();

    let nonce = 0u64;
    let call = Call {
        to: Address::generate(&env),
        func: Symbol::new(&env, "test"),
        args: vec![&env],
    };

    // Should not panic with empty args
    let leaf = client.encode_leaf(&nonce, &call);
    assert_eq!(leaf.len(), 32);

    // Encoding same call should produce same leaf
    let leaf2 = client.encode_leaf(&nonce, &call);
    assert_eq!(leaf, leaf2);
}

#[test]
fn test_nonce_getter_consistency() {
    let (_env, _contract_id, client) = setup();

    // Initially nonce should be 0
    assert_eq!(client.nonce(), 0);

    // Nonce should remain 0 after multiple reads
    assert_eq!(client.nonce(), 0);
    assert_eq!(client.nonce(), 0);
}

#[test]
fn test_verify_transaction_proof_dummy_call() {
    let (env, _contract_id, client) = setup();

    // Test with a dummy call - should still validate proof structure
    let call = Call {
        to: Address::generate(&env),
        func: Symbol::new(&env, "dummy"),
        args: vec![&env],
    };
    let proof = vec![&env, BytesN::from_array(&env, &[0u8; 32])];
    let root = BytesN::from_array(&env, &[1u8; 32]);

    let transaction = Transaction { call, proof };

    // Proof won't match, testing that proof validation works correctly
    let res = client.try_verify_transaction_proof(&root, &transaction);
    assert_eq!(
        res.err().unwrap().ok().unwrap(),
        OneSigError::InvalidProofOrNonce.into()
    );
}

// ============================================================================
// Tests for __check_auth and calls_from_contexts
// ============================================================================

#[test]
fn test_check_auth_empty_contexts() {
    let env = Env::default();

    // Generate a real secp256k1 keypair for the signer
    let signing_key = generate_secp256k1_keypair();
    let signer_address = secp256k1_signer_address(&env, &signing_key);

    // Set up contract with known seed and the real signer
    let seed = BytesN::from_array(&env, &[42u8; 32]);
    let signers = vec![&env, signer_address.clone()];

    let contract_id = create_onesig_with_defaults(
        &env,
        Some(0u64),
        Some(seed.clone()),
        Some(signers),
        Some(1u32),
        None,
        Some(false),
    );

    let client = OneSigClient::new(&env, &contract_id);

    env.ledger().set_timestamp(1000);

    // Empty contexts - use a dummy call for the leaf encoding
    let dummy_call = Call {
        to: contract_id.clone(),
        func: Symbol::new(&env, "dummy"),
        args: vec![&env],
    };
    let auth_contexts: Vec<Context> = vec![&env];

    // Encode the leaf with a dummy call
    let leaf = client.encode_leaf(&0u64, &dummy_call);
    let merkle_root = leaf.clone();
    let proof: Vec<BytesN<32>> = vec![&env];

    let expiry = 9999u64;
    let digest = build_eip712_digest(&env, &seed, &merkle_root, expiry);
    let signature = secp256k1_sign(&env, &signing_key, &digest);

    let auth_data = TransactionAuthData {
        merkle_root,
        expiry,
        proof,
        signatures: vec![&env, signature],
        sender: Sender::Permissionless,
    };

    let payload = BytesN::from_array(&env, &[0u8; 32]);

    // Should fail with InvalidAuthContext (empty contexts no longer allowed)
    let result = env.try_invoke_contract_check_auth::<crate::errors::OneSigError>(
        &contract_id,
        &payload,
        auth_data.into_val(&env),
        &auth_contexts,
    );

    assert_eq!(
        result.unwrap_err(),
        Ok(crate::errors::OneSigError::InvalidAuthContext)
    );
    assert_eq!(client.nonce(), 0);
}

#[test]
fn test_check_auth_non_executor_rejected_when_executor_required() {
    use super::helpers::ed25519_public_key;
    use ed25519_dalek::Signer;

    let env = Env::default();

    // Generate secp256k1 keypair for the signer (for merkle root signatures)
    let signing_key = generate_secp256k1_keypair();
    let signer_address = secp256k1_signer_address(&env, &signing_key);

    // Generate ed25519 keypairs - one for a valid executor, one for a non-executor
    let valid_executor_keypair = generate_ed25519_keypair();
    let valid_executor_key = ed25519_public_key(&env, &valid_executor_keypair);

    let non_executor_keypair = generate_ed25519_keypair();
    let non_executor_key = ed25519_public_key(&env, &non_executor_keypair);

    // Set up contract with executor_required = true
    let seed = BytesN::from_array(&env, &[42u8; 32]);
    let signers = vec![&env, signer_address.clone()];
    let executors = vec![&env, valid_executor_key.clone()]; // Only valid_executor is an executor

    let contract_id = create_onesig_with_defaults(
        &env,
        Some(0u64),
        Some(seed.clone()),
        Some(signers),
        Some(1u32),
        Some(executors),
        Some(true), // executor_required = true
    );

    let client = OneSigClient::new(&env, &contract_id);

    env.ledger().set_timestamp(1000);

    // Create a valid call
    let new_seed = BytesN::from_array(&env, &[99u8; 32]);
    let call = Call {
        to: contract_id.clone(),
        func: Symbol::new(&env, "set_seed"),
        args: vec![&env, new_seed.into_val(&env)],
    };

    let leaf = client.encode_leaf(&0u64, &call);
    let merkle_root = leaf.clone();
    let proof: Vec<BytesN<32>> = vec![&env];

    let expiry = 9999u64;
    let digest = build_eip712_digest(&env, &seed, &merkle_root, expiry);
    let signature = secp256k1_sign(&env, &signing_key, &digest);

    // Create auth context
    let auth_context = Context::Contract(soroban_sdk::auth::ContractContext {
        contract: contract_id.clone(),
        fn_name: Symbol::new(&env, "set_seed"),
        args: vec![&env, new_seed.into_val(&env)],
    });
    let auth_contexts = vec![&env, auth_context];

    // Sign the payload with non-executor's ed25519 key
    let payload = BytesN::from_array(&env, &[0u8; 32]);
    let sender_sig_bytes = non_executor_keypair.sign(&payload.to_array());
    let sender_signature: BytesN<64> = BytesN::from_array(&env, &sender_sig_bytes.to_bytes());

    let auth_data = TransactionAuthData {
        merkle_root,
        expiry,
        proof,
        signatures: vec![&env, signature],
        sender: Sender::Executor(non_executor_key, sender_signature),
    };

    // Should fail with OnlyExecutorOrSigner because non_executor_key is not an executor
    let result = env.try_invoke_contract_check_auth::<crate::errors::OneSigError>(
        &contract_id,
        &payload,
        auth_data.into_val(&env),
        &auth_contexts,
    );

    assert!(result.is_err(), "Non-executor should be rejected");
    assert_eq!(
        result.unwrap_err(),
        Ok(crate::errors::OneSigError::OnlyExecutorOrSigner)
    );
}

#[test]
fn test_check_auth_executor_accepted_when_executor_required() {
    use super::helpers::ed25519_public_key;
    use ed25519_dalek::Signer;

    let env = Env::default();

    // Generate secp256k1 keypair for the signer
    let signing_key = generate_secp256k1_keypair();
    let signer_address = secp256k1_signer_address(&env, &signing_key);

    // Generate ed25519 keypair for the executor
    let executor_keypair = generate_ed25519_keypair();
    let executor_key = ed25519_public_key(&env, &executor_keypair);

    // Set up contract with executor_required = true
    let seed = BytesN::from_array(&env, &[42u8; 32]);
    let signers = vec![&env, signer_address.clone()];
    let executors = vec![&env, executor_key.clone()];

    let contract_id = create_onesig_with_defaults(
        &env,
        Some(0u64),
        Some(seed.clone()),
        Some(signers),
        Some(1u32),
        Some(executors),
        Some(true), // executor_required = true
    );

    let client = OneSigClient::new(&env, &contract_id);

    env.ledger().set_timestamp(1000);

    // Create a valid call
    let new_seed = BytesN::from_array(&env, &[99u8; 32]);
    let call = Call {
        to: contract_id.clone(),
        func: Symbol::new(&env, "set_seed"),
        args: vec![&env, new_seed.into_val(&env)],
    };

    let leaf = client.encode_leaf(&0u64, &call);
    let merkle_root = leaf.clone();
    let proof: Vec<BytesN<32>> = vec![&env];

    let expiry = 9999u64;
    let digest = build_eip712_digest(&env, &seed, &merkle_root, expiry);
    let signature = secp256k1_sign(&env, &signing_key, &digest);

    // Create auth context
    let auth_context = Context::Contract(soroban_sdk::auth::ContractContext {
        contract: contract_id.clone(),
        fn_name: Symbol::new(&env, "set_seed"),
        args: vec![&env, new_seed.into_val(&env)],
    });
    let auth_contexts = vec![&env, auth_context];

    // Sign the payload with executor's ed25519 key
    let payload = BytesN::from_array(&env, &[0u8; 32]);
    let sender_sig_bytes = executor_keypair.sign(&payload.to_array());
    let sender_signature: BytesN<64> = BytesN::from_array(&env, &sender_sig_bytes.to_bytes());

    let auth_data = TransactionAuthData {
        merkle_root,
        expiry,
        proof,
        signatures: vec![&env, signature],
        sender: Sender::Executor(executor_key, sender_signature),
    };

    // Should succeed because executor_key is a valid executor
    let result = env.try_invoke_contract_check_auth::<crate::errors::OneSigError>(
        &contract_id,
        &payload,
        auth_data.into_val(&env),
        &auth_contexts,
    );

    assert!(result.is_ok(), "Executor should be accepted: {:?}", result);
    assert_eq!(client.nonce(), 1);
}

#[test]
fn test_check_auth_signer_allowed_when_executor_required() {
    let env = Env::default();

    let signing_key = generate_secp256k1_keypair();
    let signer_address = secp256k1_signer_address(&env, &signing_key);
    let executor_key = new_executor_key(&env);

    let seed = BytesN::from_array(&env, &[42u8; 32]);
    let signers = vec![&env, signer_address.clone()];
    let executors = vec![&env, executor_key.clone()];

    let contract_id = create_onesig_with_defaults(
        &env,
        Some(0u64),
        Some(seed.clone()),
        Some(signers),
        Some(1u32),
        Some(executors),
        Some(true),
    );

    let client = OneSigClient::new(&env, &contract_id);

    env.ledger().set_timestamp(1000);

    let new_seed = BytesN::from_array(&env, &[99u8; 32]);
    let call = Call {
        to: contract_id.clone(),
        func: Symbol::new(&env, "set_seed"),
        args: vec![&env, new_seed.into_val(&env)],
    };

    let leaf = client.encode_leaf(&0u64, &call);
    let merkle_root = leaf.clone();
    let proof: Vec<BytesN<32>> = vec![&env];

    let expiry = 9999u64;
    let digest = build_eip712_digest(&env, &seed, &merkle_root, expiry);
    let signature = secp256k1_sign(&env, &signing_key, &digest);

    let auth_context = Context::Contract(soroban_sdk::auth::ContractContext {
        contract: contract_id.clone(),
        fn_name: Symbol::new(&env, "set_seed"),
        args: vec![&env, new_seed.into_val(&env)],
    });
    let auth_contexts = vec![&env, auth_context];

    let payload = BytesN::from_array(&env, &[0u8; 32]);
    let signer_signature = secp256k1_sign(&env, &signing_key, &payload);

    let auth_data = TransactionAuthData {
        merkle_root,
        expiry,
        proof,
        signatures: vec![&env, signature],
        sender: Sender::Signer(signer_signature),
    };

    let result = env.try_invoke_contract_check_auth::<crate::errors::OneSigError>(
        &contract_id,
        &payload,
        auth_data.into_val(&env),
        &auth_contexts,
    );

    assert!(
        result.is_ok(),
        "Signer should be accepted when executor required"
    );
    assert_eq!(client.nonce(), 1);
}

#[test]
fn test_check_auth_unregistered_signer_rejected_with_only_executor_or_signer() {
    let env = Env::default();

    let registered_signing_key = generate_secp256k1_keypair();
    let registered_signer_address = secp256k1_signer_address(&env, &registered_signing_key);

    let unregistered_signing_key = generate_secp256k1_keypair();
    let executor_key = new_executor_key(&env);

    let seed = BytesN::from_array(&env, &[42u8; 32]);
    let signers = vec![&env, registered_signer_address];
    let executors = vec![&env, executor_key];

    let contract_id = create_onesig_with_defaults(
        &env,
        Some(0u64),
        Some(seed.clone()),
        Some(signers),
        Some(1u32),
        Some(executors),
        Some(true),
    );

    let client = OneSigClient::new(&env, &contract_id);

    env.ledger().set_timestamp(1000);

    let new_seed = BytesN::from_array(&env, &[99u8; 32]);
    let call = Call {
        to: contract_id.clone(),
        func: Symbol::new(&env, "set_seed"),
        args: vec![&env, new_seed.into_val(&env)],
    };

    let leaf = client.encode_leaf(&0u64, &call);
    let merkle_root = leaf.clone();
    let proof: Vec<BytesN<32>> = vec![&env];

    let expiry = 9999u64;
    let digest = build_eip712_digest(&env, &seed, &merkle_root, expiry);
    let registered_signature = secp256k1_sign(&env, &registered_signing_key, &digest);

    let auth_context = Context::Contract(soroban_sdk::auth::ContractContext {
        contract: contract_id.clone(),
        fn_name: Symbol::new(&env, "set_seed"),
        args: vec![&env, new_seed.into_val(&env)],
    });
    let auth_contexts = vec![&env, auth_context];

    let payload = BytesN::from_array(&env, &[0u8; 32]);
    let unregistered_signer_signature = secp256k1_sign(&env, &unregistered_signing_key, &payload);

    let auth_data = TransactionAuthData {
        merkle_root,
        expiry,
        proof,
        signatures: vec![&env, registered_signature],
        sender: Sender::Signer(unregistered_signer_signature),
    };

    let result = env.try_invoke_contract_check_auth::<crate::errors::OneSigError>(
        &contract_id,
        &payload,
        auth_data.into_val(&env),
        &auth_contexts,
    );

    assert!(result.is_err(), "Unregistered signer should be rejected");
    assert_eq!(
        result.unwrap_err(),
        Ok(crate::errors::OneSigError::OnlyExecutorOrSigner)
    );
    assert_eq!(client.nonce(), 0);
}

#[test]
fn test_check_auth_rejects_non_contract_context() {
    let env = Env::default();

    let signing_key = generate_secp256k1_keypair();
    let signer_address = secp256k1_signer_address(&env, &signing_key);

    let seed = BytesN::from_array(&env, &[42u8; 32]);
    let signers = vec![&env, signer_address.clone()];

    let contract_id = create_onesig_with_defaults(
        &env,
        Some(0u64),
        Some(seed.clone()),
        Some(signers),
        Some(1u32),
        None,
        Some(false),
    );

    let client = OneSigClient::new(&env, &contract_id);

    env.ledger().set_timestamp(1000);

    let new_seed = BytesN::from_array(&env, &[99u8; 32]);
    let call = Call {
        to: contract_id.clone(),
        func: Symbol::new(&env, "set_seed"),
        args: vec![&env, new_seed.into_val(&env)],
    };

    let leaf = client.encode_leaf(&0u64, &call);
    let merkle_root = leaf.clone();
    let proof: Vec<BytesN<32>> = vec![&env];

    let expiry = 9999u64;
    let digest = build_eip712_digest(&env, &seed, &merkle_root, expiry);
    let signature = secp256k1_sign(&env, &signing_key, &digest);

    let invalid_context = Context::CreateContractHostFn(CreateContractHostFnContext {
        executable: ContractExecutable::Wasm(BytesN::from_array(&env, &[7u8; 32])),
        salt: BytesN::from_array(&env, &[1u8; 32]),
    });
    let auth_contexts = vec![&env, invalid_context];

    let payload = BytesN::from_array(&env, &[0u8; 32]);
    let auth_data = TransactionAuthData {
        merkle_root,
        expiry,
        proof,
        signatures: vec![&env, signature],
        sender: Sender::Permissionless,
    };

    let result = env.try_invoke_contract_check_auth::<crate::errors::OneSigError>(
        &contract_id,
        &payload,
        auth_data.into_val(&env),
        &auth_contexts,
    );

    assert!(result.is_err(), "Non-contract context should be rejected");
    assert_eq!(
        result.unwrap_err(),
        Ok(crate::errors::OneSigError::NonContractInvoke)
    );
}

#[test]
fn test_check_auth_invalid_executor_signature() {
    use super::helpers::ed25519_public_key;
    use ed25519_dalek::Signer;

    let env = Env::default();

    // Generate secp256k1 keypair for the signer
    let signing_key = generate_secp256k1_keypair();
    let signer_address = secp256k1_signer_address(&env, &signing_key);

    // Generate ed25519 keypair for the executor
    let executor_keypair = generate_ed25519_keypair();
    let executor_key = ed25519_public_key(&env, &executor_keypair);

    // Set up contract with executor_required = true
    let seed = BytesN::from_array(&env, &[42u8; 32]);
    let signers = vec![&env, signer_address.clone()];
    let executors = vec![&env, executor_key.clone()];

    let contract_id = create_onesig_with_defaults(
        &env,
        Some(0u64),
        Some(seed.clone()),
        Some(signers),
        Some(1u32),
        Some(executors),
        Some(true), // executor_required = true
    );

    let client = OneSigClient::new(&env, &contract_id);

    env.ledger().set_timestamp(1000);

    // Create a valid call
    let new_seed = BytesN::from_array(&env, &[99u8; 32]);
    let call = Call {
        to: contract_id.clone(),
        func: Symbol::new(&env, "set_seed"),
        args: vec![&env, new_seed.into_val(&env)],
    };

    let leaf = client.encode_leaf(&0u64, &call);
    let merkle_root = leaf.clone();
    let proof: Vec<BytesN<32>> = vec![&env];

    let expiry = 9999u64;
    let digest = build_eip712_digest(&env, &seed, &merkle_root, expiry);
    let signature = secp256k1_sign(&env, &signing_key, &digest);

    // Create auth context
    let auth_context = Context::Contract(soroban_sdk::auth::ContractContext {
        contract: contract_id.clone(),
        fn_name: Symbol::new(&env, "set_seed"),
        args: vec![&env, new_seed.into_val(&env)],
    });
    let auth_contexts = vec![&env, auth_context];

    // Sign a DIFFERENT payload to create an invalid signature
    let wrong_payload = BytesN::from_array(&env, &[0xFFu8; 32]);
    let sender_sig_bytes = executor_keypair.sign(&wrong_payload.to_array());
    let invalid_sender_signature: BytesN<64> =
        BytesN::from_array(&env, &sender_sig_bytes.to_bytes());

    // The actual payload that will be verified
    let payload = BytesN::from_array(&env, &[0u8; 32]);

    let auth_data = TransactionAuthData {
        merkle_root,
        expiry,
        proof,
        signatures: vec![&env, signature],
        sender: Sender::Executor(executor_key, invalid_sender_signature), // Wrong signature for payload
    };

    // ed25519_verify fails and returns a crypto error
    let result = env.try_invoke_contract_check_auth::<crate::errors::OneSigError>(
        &contract_id,
        &payload,
        auth_data.into_val(&env),
        &auth_contexts,
    );

    // Verify it failed with a crypto error (ed25519 verification failure)
    assert!(result.is_err(), "Invalid executor signature should fail");
    // The error is a host-level crypto error, not a contract error
    let err = result.unwrap_err();
    // Err variant contains the raw error from the host
    assert!(
        err.is_err(),
        "Should be a host error (Crypto), not a contract error: {:?}",
        err
    );
    // Nonce should not be incremented
    assert_eq!(client.nonce(), 0);
}

#[test]
fn test_check_auth_executor_required_but_no_sender() {
    let env = Env::default();

    // Generate secp256k1 keypair for the signer
    let signing_key = generate_secp256k1_keypair();
    let signer_address = secp256k1_signer_address(&env, &signing_key);

    // Generate ed25519 keypair for the executor
    let executor_keypair = generate_ed25519_keypair();
    let executor_key = super::helpers::ed25519_public_key(&env, &executor_keypair);

    // Set up contract with executor_required = true
    let seed = BytesN::from_array(&env, &[42u8; 32]);
    let signers = vec![&env, signer_address.clone()];
    let executors = vec![&env, executor_key.clone()];

    let contract_id = create_onesig_with_defaults(
        &env,
        Some(0u64),
        Some(seed.clone()),
        Some(signers),
        Some(1u32),
        Some(executors),
        Some(true), // executor_required = true
    );

    let client = OneSigClient::new(&env, &contract_id);

    env.ledger().set_timestamp(1000);

    // Create a valid call
    let new_seed = BytesN::from_array(&env, &[99u8; 32]);
    let call = Call {
        to: contract_id.clone(),
        func: Symbol::new(&env, "set_seed"),
        args: vec![&env, new_seed.into_val(&env)],
    };

    let leaf = client.encode_leaf(&0u64, &call);
    let merkle_root = leaf.clone();
    let proof: Vec<BytesN<32>> = vec![&env];

    let expiry = 9999u64;
    let digest = build_eip712_digest(&env, &seed, &merkle_root, expiry);
    let signature = secp256k1_sign(&env, &signing_key, &digest);

    // Create auth context
    let auth_context = Context::Contract(soroban_sdk::auth::ContractContext {
        contract: contract_id.clone(),
        fn_name: Symbol::new(&env, "set_seed"),
        args: vec![&env, new_seed.into_val(&env)],
    });
    let auth_contexts = vec![&env, auth_context];

    // Don't provide sender or sender_signature (both are None)
    let auth_data = TransactionAuthData {
        merkle_root,
        expiry,
        proof,
        signatures: vec![&env, signature],
        sender: Sender::Permissionless, // Missing sender
    };

    let payload = BytesN::from_array(&env, &[0u8; 32]);

    // Should fail with ExecutorRequired because sender is None when executor_required = true
    let result = env.try_invoke_contract_check_auth::<crate::errors::OneSigError>(
        &contract_id,
        &payload,
        auth_data.into_val(&env),
        &auth_contexts,
    );

    assert!(
        result.is_err(),
        "Should fail when executor required but no sender provided"
    );
    // ExecutorRequired = 13
    assert_eq!(
        result.unwrap_err(),
        Ok(crate::errors::OneSigError::OnlyExecutorOrSigner)
    );
}

#[test]
fn test_check_auth_mismatched_calls_rejected() {
    let env = Env::default();

    // Generate a real secp256k1 keypair for the signer
    let signing_key = generate_secp256k1_keypair();
    let signer_address = secp256k1_signer_address(&env, &signing_key);

    let seed = BytesN::from_array(&env, &[42u8; 32]);
    let signers = vec![&env, signer_address.clone()];

    let contract_id = create_onesig_with_defaults(
        &env,
        Some(0u64),
        Some(seed.clone()),
        Some(signers),
        Some(1u32),
        None,
        Some(false),
    );

    let client = OneSigClient::new(&env, &contract_id);

    env.ledger().set_timestamp(1000);

    // Create merkle proof for call A
    let seed_a = BytesN::from_array(&env, &[0xAAu8; 32]);
    let call_a = Call {
        to: contract_id.clone(),
        func: Symbol::new(&env, "set_seed"),
        args: vec![&env, seed_a.into_val(&env)],
    };
    let leaf = client.encode_leaf(&0u64, &call_a);
    let merkle_root = leaf.clone();
    let proof: Vec<BytesN<32>> = vec![&env];

    let expiry = 9999u64;
    let digest = build_eip712_digest(&env, &seed, &merkle_root, expiry);
    let signature = secp256k1_sign(&env, &signing_key, &digest);

    // But provide auth context for call B (different seed value)
    let seed_b = BytesN::from_array(&env, &[0xBBu8; 32]);
    let auth_context = Context::Contract(soroban_sdk::auth::ContractContext {
        contract: contract_id.clone(),
        fn_name: Symbol::new(&env, "set_seed"),
        args: vec![&env, seed_b.into_val(&env)], // Different from what's in merkle proof
    });
    let auth_contexts = vec![&env, auth_context];

    let auth_data = TransactionAuthData {
        merkle_root,
        expiry,
        proof,
        signatures: vec![&env, signature],
        sender: Sender::Permissionless,
    };

    let payload = BytesN::from_array(&env, &[0u8; 32]);

    // Should fail because the derived calls from context don't match the merkle proof
    let result = env.try_invoke_contract_check_auth::<crate::errors::OneSigError>(
        &contract_id,
        &payload,
        auth_data.into_val(&env),
        &auth_contexts,
    );

    assert!(result.is_err(), "Mismatched calls should be rejected");
    assert_eq!(
        result.unwrap_err(),
        Ok(crate::errors::OneSigError::InvalidProofOrNonce)
    );
}

#[test]
fn test_check_auth_multiple_calls() {
    let env = Env::default();

    let signing_key = generate_secp256k1_keypair();
    let signer_address = secp256k1_signer_address(&env, &signing_key);

    let seed = BytesN::from_array(&env, &[42u8; 32]);
    let signers = vec![&env, signer_address.clone()];

    let contract_id = create_onesig_with_defaults(
        &env,
        Some(0u64),
        Some(seed.clone()),
        Some(signers),
        Some(1u32),
        None,
        Some(false),
    );

    let client = OneSigClient::new(&env, &contract_id);

    env.ledger().set_timestamp(1000);

    // Create a single call for the leaf, but we'll provide multiple auth contexts to test rejection
    let seed1 = BytesN::from_array(&env, &[1u8; 32]);
    let seed2 = BytesN::from_array(&env, &[2u8; 32]);
    let call = Call {
        to: contract_id.clone(),
        func: Symbol::new(&env, "set_seed"),
        args: vec![&env, seed1.into_val(&env)],
    };

    let leaf = client.encode_leaf(&0u64, &call);
    let merkle_root = leaf.clone();
    let proof: Vec<BytesN<32>> = vec![&env];

    let expiry = 9999u64;
    let digest = build_eip712_digest(&env, &seed, &merkle_root, expiry);
    let signature = secp256k1_sign(&env, &signing_key, &digest);

    // Create matching auth contexts
    let auth_context1 = Context::Contract(soroban_sdk::auth::ContractContext {
        contract: contract_id.clone(),
        fn_name: Symbol::new(&env, "set_seed"),
        args: vec![&env, seed1.into_val(&env)],
    });
    let auth_context2 = Context::Contract(soroban_sdk::auth::ContractContext {
        contract: contract_id.clone(),
        fn_name: Symbol::new(&env, "set_seed"),
        args: vec![&env, seed2.into_val(&env)],
    });
    let auth_contexts = vec![&env, auth_context1, auth_context2];

    let auth_data = TransactionAuthData {
        merkle_root,
        expiry,
        proof,
        signatures: vec![&env, signature],
        sender: Sender::Permissionless,
    };

    let payload = BytesN::from_array(&env, &[0u8; 32]);

    // Should fail with InvalidAuthContext (multiple contexts no longer allowed)
    let result = env.try_invoke_contract_check_auth::<crate::errors::OneSigError>(
        &contract_id,
        &payload,
        auth_data.into_val(&env),
        &auth_contexts,
    );

    assert_eq!(
        result.unwrap_err(),
        Ok(crate::errors::OneSigError::InvalidAuthContext)
    );
    assert_eq!(client.nonce(), 0);
}

#[test]
fn test_check_auth_nonce_increment() {
    let env = Env::default();

    // Generate a real secp256k1 keypair for the signer
    let signing_key = generate_secp256k1_keypair();
    let signer_address = secp256k1_signer_address(&env, &signing_key);

    // Set up contract with known seed and the real signer
    let seed = BytesN::from_array(&env, &[42u8; 32]);
    let signers = vec![&env, signer_address.clone()];

    let contract_id = create_onesig_with_defaults(
        &env,
        Some(0u64),         // onesig_id
        Some(seed.clone()), // seed
        Some(signers),      // signers
        Some(1u32),         // threshold = 1
        None,               // executors (empty)
        Some(false),        // executor_required = false
    );

    let client = OneSigClient::new(&env, &contract_id);

    // Verify initial nonce is 0
    assert_eq!(client.nonce(), 0);

    // Set ledger timestamp for expiry check
    env.ledger().set_timestamp(1000);

    // Execute multiple transactions and verify nonce increments each time
    for expected_nonce in 0u64..3u64 {
        // Verify current nonce before transaction
        assert_eq!(
            client.nonce(),
            expected_nonce,
            "Nonce should be {} before transaction",
            expected_nonce
        );

        // Create a unique call for each iteration
        let mut seed_bytes = [0u8; 32];
        seed_bytes[0] = (expected_nonce + 1) as u8;
        let new_seed: BytesN<32> = BytesN::from_array(&env, &seed_bytes);
        let call = Call {
            to: contract_id.clone(),
            func: Symbol::new(&env, "set_seed"),
            args: vec![&env, new_seed.into_val(&env)],
        };

        // Encode the leaf with current nonce
        let leaf = client.encode_leaf(&expected_nonce, &call);

        // For a single-leaf merkle tree, root = leaf and proof is empty
        let merkle_root = leaf.clone();
        let proof: Vec<BytesN<32>> = vec![&env];

        // Expiry in the future
        let expiry = 9999u64 + expected_nonce;

        // Build the EIP712 digest and sign it
        let digest = build_eip712_digest(&env, &seed, &merkle_root, expiry);
        let signature = secp256k1_sign(&env, &signing_key, &digest);

        // Create the auth context matching the call
        let auth_context = Context::Contract(soroban_sdk::auth::ContractContext {
            contract: contract_id.clone(),
            fn_name: Symbol::new(&env, "set_seed"),
            args: vec![&env, new_seed.into_val(&env)],
        });
        let auth_contexts = vec![&env, auth_context];

        // Create TransactionAuthData with valid signature and proof
        let auth_data = TransactionAuthData {
            merkle_root: merkle_root.clone(),
            expiry,
            proof,
            signatures: vec![&env, signature],
            sender: Sender::Permissionless,
        };

        let payload = BytesN::from_array(&env, &[0u8; 32]);

        // Call __check_auth - this should succeed and increment the nonce
        let result = env.try_invoke_contract_check_auth::<crate::errors::OneSigError>(
            &contract_id,
            &payload,
            auth_data.into_val(&env),
            &auth_contexts,
        );

        // Verify __check_auth succeeded
        assert!(
            result.is_ok(),
            "Expected __check_auth to succeed for nonce {}, got: {:?}",
            expected_nonce,
            result
        );

        // Verify nonce was incremented
        assert_eq!(
            client.nonce(),
            expected_nonce + 1,
            "Nonce should be {} after transaction {}",
            expected_nonce + 1,
            expected_nonce
        );
    }

    // Final verification - nonce should be 3 after 3 successful transactions
    assert_eq!(
        client.nonce(),
        3,
        "Final nonce should be 3 after 3 successful transactions"
    );
}

#[test]
fn test_version() {
    let (env, _contract_id, client) = setup();

    let version = client.version();
    assert_eq!(version, soroban_sdk::String::from_str(&env, "0.0.1"));
}

#[test]
fn test_leaf_encoding_version() {
    let (_env, _contract_id, client) = setup();

    let leaf_encoding_version = client.leaf_encoding_version();
    assert_eq!(leaf_encoding_version, 1);
}
