use soroban_sdk::{BytesN, Env, U256};
use utils::buffer_writer::BufferWriter;

/// All signatures follow the EIP191 standard for message signing.
/// This implements EIP712 which structures signed data
/// into domain-separated, typed data. The prefix 0x1901 ensures
/// signed messages cannot be confused with regular Ethereum transactions.
const EIP191_PREFIX_FOR_EIP712: [u8; 2] = [0x19, 0x01];

/// keccak256("SignMerkleRoot(bytes32 seed,bytes32 merkleRoot,uint256 expiry)")
const SIGN_MERKLE_ROOT_TYPE_HASH: [u8; 32] = [
    0x64, 0x2e, 0xd5, 0xd2, 0xb7, 0x7b, 0xc7, 0xcc, 0xb9, 0x8e, 0x10, 0xda, 0x4c, 0x02, 0xd7, 0xcd,
    0x82, 0x31, 0x22, 0x8d, 0xa4, 0x22, 0x2a, 0x9f, 0x88, 0xa8, 0x0c, 0x15, 0x54, 0x50, 0x74, 0xed,
];

/// Pre-computed EIP-712 domain separator
/// keccak256(abi.encode(EIP712DOMAIN_TYPE_HASH, keccak256("OneSig"), keccak256("0.0.1"), 1, 0xdEaD))
const EIP712_DOMAIN_SEPARATOR: [u8; 32] = [
    0x94, 0xc2, 0x89, 0x89, 0x17, 0x0e, 0xb4, 0xdc, 0x31, 0x35, 0x91, 0x74, 0xb9, 0x11, 0x5c, 0x11,
    0x6a, 0x8f, 0xaf, 0xa6, 0x7b, 0x5a, 0xda, 0xcc, 0x57, 0x0c, 0xa5, 0x83, 0xeb, 0x96, 0xd6, 0x57,
];

/// Computes the EIP-712 style digest for merkle root verification
///
/// # Arguments
/// * `env` - The Soroban environment
/// * `seed` - The seed value for this OneSig instance
/// * `merkle_root` - The merkle root to sign
/// * `expiry` - The expiry timestamp
///
/// # Returns
/// The 32-byte digest hash to be signed
pub fn build_eip712_digest(
    env: &Env,
    seed: &BytesN<32>,
    merkle_root: &BytesN<32>,
    expiry: u64,
) -> BytesN<32> {
    // Build payload: keccak256(abi.encode(SIGN_MERKLE_ROOT_TYPE_HASH, seed, merkleRoot, expiry))
    // According to EIP-712 and Solidity's abi.encode, the encoding is:
    // typeHash (32 bytes) || seed (32 bytes) || merkleRoot (32 bytes) || expiry (32 bytes as uint256)
    let mut payload_writer = BufferWriter::new(env);
    let payload = payload_writer
        .write_array(&SIGN_MERKLE_ROOT_TYPE_HASH) // Type hash (32 bytes)
        .write_bytes_n(seed) // Seed (32 bytes)
        .write_bytes_n(merkle_root) // Merkle root (32 bytes)
        .write_u256(U256::from_u128(env, expiry as u128)) // Expiry (32 bytes as uint256)
        .to_bytes();
    // Hash payload (returns Hash<32>)
    // This is the structHash: keccak256(typeHash || encodeData(struct))
    let payload_hash = env.crypto().keccak256(&payload);

    // Build digest: keccak256(EIP191_PREFIX || DOMAIN_SEPARATOR || payload_hash)
    // According to EIP-712: keccak256(0x19 || 0x01 || domainSeparator || structHash)
    let mut digest_writer = BufferWriter::new(env);
    let digest_data = digest_writer
        .write_array(&EIP191_PREFIX_FOR_EIP712) // EIP-191 prefix (2 bytes)
        .write_array(&EIP712_DOMAIN_SEPARATOR) // Domain separator (32 bytes)
        .write_array(&payload_hash.to_array()) // Payload hash (32 bytes)
        .to_bytes();
    env.crypto().keccak256(&digest_data).into()
}

#[cfg(test)]
mod tests {
    use super::{BufferWriter, EIP712_DOMAIN_SEPARATOR, SIGN_MERKLE_ROOT_TYPE_HASH};
    use soroban_sdk::{Bytes, Env, U256};

    fn keccak(env: &Env, input: &[u8]) -> [u8; 32] {
        env.crypto()
            .keccak256(&Bytes::from_slice(env, input))
            .to_array()
    }

    #[test]
    fn test_sign_merkle_root_type_hash_matches_source_string() {
        let env = Env::default();

        let computed = keccak(
            &env,
            b"SignMerkleRoot(bytes32 seed,bytes32 merkleRoot,uint256 expiry)",
        );

        assert_eq!(computed, SIGN_MERKLE_ROOT_TYPE_HASH);
    }

    #[test]
    fn test_eip712_domain_separator_matches_formula() {
        let env = Env::default();

        let type_hash = keccak(
            &env,
            b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
        );
        let name_hash = keccak(&env, b"OneSig");
        let version_hash = keccak(&env, b"0.0.1");

        // Solidity ABI-encoding for address is 32 bytes, left padded.
        let mut verifying_contract = [0u8; 32];
        verifying_contract[30] = 0xde;
        verifying_contract[31] = 0xad;

        let encoded = BufferWriter::new(&env)
            .write_array(&type_hash)
            .write_array(&name_hash)
            .write_array(&version_hash)
            .write_u256(U256::from_u128(&env, 1u128))
            .write_array(&verifying_contract)
            .to_bytes();

        let computed = env.crypto().keccak256(&encoded).to_array();
        assert_eq!(computed, EIP712_DOMAIN_SEPARATOR);
    }
}
