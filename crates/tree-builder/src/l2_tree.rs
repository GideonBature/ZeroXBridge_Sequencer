use std::{array::TryFromSliceError, sync::Arc};

use accumulators::{
    hasher::stark_poseidon,
    mmr::{Proof, MMR},
    store::memory::InMemoryStore,
};

use crate::{error::TreeBuilderError, types::Result};

/// A builder for constructing Merkle trees and generating proofs
pub struct L2MerkleTreeBuilder {
    mmr: MMR,
}

impl L2MerkleTreeBuilder {
    fn decode_hex(hex_str: &str) -> Result<[u8; 32]> {
        let hex_to_decode = if hex_str.starts_with("0x") {
            &hex_str[2..]
        } else {
            hex_str
        };
        let mut bytes = hex::decode(hex_to_decode)?;

        // Pad with zeros if needed
        while bytes.len() < 32 {
            bytes.insert(0, 0);
        }
        // Truncate if too long
        if bytes.len() > 32 {
            bytes.truncate(32);
        }

        bytes
            .as_slice()
            .try_into()
            .map_err(|e: TryFromSliceError| TreeBuilderError::ConversionError(e.to_string()))
    }

    /// Creates a new L2MerkleTreeBuilder instance
    pub fn new() -> Self {
        let store = InMemoryStore::default();
        let store_rc = Arc::new(store);
        let hasher = Arc::new(stark_poseidon::StarkPoseidonHasher::new(None));

        Self {
            mmr: MMR::new(store_rc, hasher, None),
        }
    }

    /// Builds a Merkle tree from a list of commitment hashes
    pub async fn build_merkle(&mut self, leaves: Vec<[u8; 32]>) -> Result<()> {
        for leaf in leaves {
            self.mmr.append(format!("0x{}", hex::encode(leaf))).await?;
        }
        Ok(())
    }

    /// Gets the current Merkle root
    pub async fn get_root(&self) -> Result<[u8; 32]> {
        let bag = self.mmr.bag_the_peaks(None).await?;
        let elements_count = self.mmr.elements_count.get().await?;
        let root = self.mmr.calculate_root_hash(&bag, elements_count)?;
        Self::decode_hex(&root)
    }

    /// Generates a Merkle proof for a given leaf
    pub async fn get_proof(&self, leaf: [u8; 32]) -> Result<Option<Proof>> {
        let elements_count = self.mmr.elements_count.get().await?;
        let leaf_str = format!("0x{}", hex::encode(leaf));

        // Find the leaf index by scanning elements
        let mut leaf_index = None;
        for i in 1..=elements_count {
            if let Some(hash) = self
                .mmr
                .hashes
                .get(accumulators::store::SubKey::Usize(i))
                .await?
            {
                if hash == leaf_str {
                    leaf_index = Some(i);
                    break;
                }
            }
        }

        if let Some(idx) = leaf_index {
            let proof = self.mmr.get_proof(idx, None).await?;
            Ok(Some(proof))
        } else {
            Ok(None)
        }
    }

    /// Verifies a Merkle proof for a given leaf
    pub async fn verify_proof(&self, proof: Proof, leaf: [u8; 32]) -> Result<bool> {
        let leaf_str = format!("0x{}", hex::encode(leaf));
        Ok(self.mmr.verify_proof(proof, leaf_str, None).await?)
    }
}

impl Default for L2MerkleTreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::felt252_to_hex;

    #[tokio::test]
    async fn test_basic_tree_operations() -> Result<()> {
        let mut builder = L2MerkleTreeBuilder::new();

        // Test single leaf
        let leaf1 = [1u8; 32];
        builder.build_merkle(vec![leaf1]).await?;

        // Get proof for leaf1
        let proof1 = builder.get_proof(leaf1).await?;
        assert!(proof1.is_some(), "Should generate proof for existing leaf");
        assert!(
            builder.verify_proof(proof1.unwrap(), leaf1).await?,
            "Proof should be valid"
        );

        // Add second leaf
        let leaf2 = [2u8; 32];
        builder.build_merkle(vec![leaf2]).await?;

        // Verify both leaves have valid proofs
        for leaf in [leaf1, leaf2] {
            let proof = builder.get_proof(leaf).await?.unwrap();
            assert!(
                builder.verify_proof(proof, leaf).await?,
                "Proof should be valid for leaf {:?}",
                leaf
            );
        }

        // Test non-existent leaf
        let fake_leaf = [99u8; 32];
        assert!(
            builder.get_proof(fake_leaf).await?.is_none(),
            "Should not find proof for non-existent leaf"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_with_l2_values() -> Result<()> {
        let leaf_one = felt252_to_hex(
            "3085182978037364507644541379307921604860861694664657935759708330416374536741",
        )?;
        let leaf_two = felt252_to_hex(
            "1515056012081702936544604035253985638654900467413915026150760243646139951112",
        )?;
        let leaf_three = felt252_to_hex(
            "2323060256672561756159719169078931556938075970039758487114302926228175567841",
        )?;
        let leaf_four = felt252_to_hex(
            "884555293850013781657518953358027212692898536740606299472615094634234324840",
        )?;

        let commitment_hashes = vec![leaf_one, leaf_two, leaf_three, leaf_four];
        let expected_root = felt252_to_hex(
            "423282815349921591262243120076891478879135827696329377607682678064132796520",
        )?;

        let mut leaves = Vec::new();
        for hash_str in &commitment_hashes {
            let bytes = hex::decode(hash_str)?;
            if bytes.len() != 32 {
                return Err(TreeBuilderError::InvalidLeafHash(format!(
                    "Expected 32 bytes, got {}",
                    bytes.len()
                )));
            }
            let mut array = [0u8; 32];
            array.copy_from_slice(&bytes);
            leaves.push(array);
        }

        let mut builder = L2MerkleTreeBuilder::new();
        builder.build_merkle(leaves).await?;

        let root = builder.get_root().await?;
        let root_hex = hex::encode(root);

        assert_eq!(
            root_hex, expected_root,
            "Root hash does not match expected value"
        );

        Ok(())
    }
}
