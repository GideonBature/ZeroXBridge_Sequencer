// Use starknet_crypto for a secure, Cairo-compatible Poseidon implementation
use starknet_crypto::{Felt, PoseidonHasher};

/// Represents the data structure for minting tokens on L2, using types compatible with Cairo's felt252
#[derive(Debug, Clone)]
pub struct MintData {
    /// Starknet address of the recipient (represents a felt252 in Cairo)
    pub recipient: Felt,
    /// USD amount to mint
    pub amount: u128,
    /// Transaction nonce
    pub nonce: u64,
    /// Block timestamp
    pub timestamp: u64,
}

impl MintData {
    /// Creates a new MintData instance
    pub fn new(recipient: Felt, amount: u128, nonce: u64, timestamp: u64) -> Self {
        Self {
            recipient,
            amount,
            nonce,
            timestamp,
        }
    }

    /// Converts MintData to a vector of Felts for hashing
    /// This follows Cairo's pattern for preparing data for Poseidon hash
    pub fn to_field_elements(&self) -> Vec<Felt> {
        vec![
            self.recipient,
            Felt::from(self.amount),
            Felt::from(self.nonce),
            Felt::from(self.timestamp),
        ]
    }
}

/// Computes a Poseidon hash over the given inputs to create a deposit commitment hash
/// compatible with Cairo contracts on Starknet.
///
/// This implementation follows the same pattern as Cairo's Poseidon hash:
/// 1. Create a hash state with `PoseidonHasher::new()`
/// 2. Add elements with `.update()`
/// 3. Finalize with `.finalize()`
///
/// The hash should match the commitment hash that the L2 contract computes and verifies.
///
/// # Arguments
/// * `recipient` - Starknet address of the recipient (represented as Felt/felt252)
/// * `amount` - USD amount to mint
/// * `nonce` - Transaction nonce
/// * `timestamp` - Block timestamp
///
/// # Returns
/// A `Felt` (felt252) representing the Poseidon hash of the input values
pub fn compute_poseidon_commitment_hash(
    recipient: Felt,
    amount: u128,
    nonce: u64,
    timestamp: u64,
) -> Felt {
    // Create MintData structure and convert to field elements
    let mint_data = MintData::new(recipient, amount, nonce, timestamp);
    let elements = mint_data.to_field_elements();

    // Use the starknet-crypto Poseidon implementation
    // This follows the same pattern as Cairo's Poseidon hash
    // 1. Create a new hasher
    let mut hasher = PoseidonHasher::new();

    // 2. Update with each element - similar to Cairo's update_with pattern
    for element in elements {
        hasher.update(element);
    }

    // 3. Finalize the hash - similar to Cairo's finalize() method
    hasher.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;
    use starknet_crypto::poseidon_hash_many;

    #[test]
    fn test_poseidon_hash_consistency() {
        // This test verifies that our hash function produces consistent results
        // using the real cryptographic Poseidon implementation

        // Create a test recipient address
        let recipient = Felt::from_dec_str("123456789012345678901234567890").unwrap();
        let amount: u128 = 1000000;
        let nonce: u64 = 42;
        let timestamp: u64 = 1650000000;

        let hash1 = compute_poseidon_commitment_hash(recipient, amount, nonce, timestamp);
        let hash2 = compute_poseidon_commitment_hash(recipient, amount, nonce, timestamp);

        assert_eq!(hash1, hash2, "Hash function should be deterministic");
    }

    #[test]
    fn test_poseidon_hash_direct_api() {
        // This test verifies that our implementation correctly uses the starknet-crypto Poseidon API
        // It manually computes the hash using direct poseidon_hash_many and compares with our function

        // Create a test recipient address
        let recipient = Felt::from_dec_str("123456789012345678901234567890").unwrap();
        let amount: u128 = 1000000;
        let nonce: u64 = 42;
        let timestamp: u64 = 1650000000;

        // Our implementation using the stateful hasher
        let hash1 = compute_poseidon_commitment_hash(recipient, amount, nonce, timestamp);

        // Direct calculation using poseidon_hash_many
        let elements = vec![
            recipient,
            Felt::from(amount),
            Felt::from(nonce),
            Felt::from(timestamp),
        ];
        let hash2 = poseidon_hash_many(&elements);

        assert_eq!(
            hash1, hash2,
            "Hash calculations should match between methods"
        );
    }

    #[test]
    fn test_poseidon_hash_direct_vs_stateful() {
        // This test verifies that our implementation matches both ways of using the Poseidon API

        // Create a test recipient address
        let recipient = Felt::from_dec_str("123456789012345678901234567890").unwrap();
        let amount: u128 = 1000000;
        let nonce: u64 = 42;
        let timestamp: u64 = 1650000000;

        // Calculate hash using our stateful PoseidonHasher implementation
        let hash1 = compute_poseidon_commitment_hash(recipient, amount, nonce, timestamp);

        // Calculate hash using direct poseidon_hash_many function
        let elements = vec![
            recipient,
            Felt::from(amount),
            Felt::from(nonce),
            Felt::from(timestamp),
        ];
        let hash2 = poseidon_hash_many(&elements);

        // Both methods should produce the same result
        assert_eq!(
            hash1, hash2,
            "Stateful hash should match direct hash_many for same inputs"
        );

        // Note: We initially had a test here comparing the stateful hash with a sequential pairwise approach
        // (like poseidon_hash(poseidon_hash(poseidon_hash(a, b), c), d)), but that actually produces a
        // different result than hashing all elements at once with poseidon_hash_many. This is expected
        // behavior for cryptographic hash functions.
        //
        // If Cairo contracts use the sequential method, we would need to match that approach exactly.
    }

    // TODO: Add test cases with actual expected outputs from Cairo contracts when available
    // #[test]
    // fn test_poseidon_hash_against_cairo_output() {
    //     // This test should verify output against known Cairo contract outputs
    //     // We need to collect real output values from the Cairo contract
    //
    //     // Test vector 1
    //     {
    //         let recipient = Felt::from_hex("0x01234567890abcdef01234567890abcdef01234567890abcdef01234567890abc").unwrap();
    //         let amount: u128 = 1000000;
    //         let nonce: u64 = 42;
    //         let timestamp: u64 = 1650000000;
    //
    //         let hash = compute_poseidon_commitment_hash(recipient, amount, nonce, timestamp);
    //
    //         // Replace with actual expected hash from Cairo contract
    //         let expected_hash = Felt::from_hex("0x...").unwrap();
    //         assert_eq!(hash, expected_hash, "Hash should match Cairo contract output");
    //     }
    // }
}
