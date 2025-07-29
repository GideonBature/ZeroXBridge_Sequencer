mod l1;
mod l2;

use core::array::{Array, ArrayTrait, SpanTrait};
use core::traits::{Into, TryInto};
use l1::verify_proof::verify_mmr_proof as verify_l1;
use l2::verify_proof::verify_proof as verify_l2;


/// Verifies a Merkle proof for a given leaf hash against an expected root hash
///
/// # Arguments
///
/// * `commitment_hash` (felt252) - The hash of the leaf node to verify.
/// * `proof` (Array<felt252>) - An array of sibling hashes.
/// * `new_root` (felt252) - The expected Merkle root hash.
///
/// # Returns
///
/// * `felt252` - The `new_root` if the verification is successful.
///
/// # Panics
///
/// * Panics with the message 'Computed root does not match' if the verification fails.
fn verify_commitment_in_root(
    commitment_hash: felt252, proof: Array<felt252>, new_root: felt252,
) -> felt252 {
    assert(!proof.is_empty(), 'Proof must not be empty');
    let proof_span: Span<felt252> = proof.span();

    let mut computed_root: felt252 = commitment_hash;
    for proof_element in proof_span {
        computed_root =
            if Into::<felt252, u256>::into(computed_root) < (*proof_element).into() {
                core::pedersen::pedersen(computed_root, *proof_element)
            } else {
                core::pedersen::pedersen(*proof_element, computed_root)
            };
    }

    assert(computed_root == new_root, 'Computed root does not match');

    computed_root
}


/// Entry point for verifying Merkle proofs on L1 or L2
///
/// This function takes a flat input array and decides whether to verify the proof
/// using the L1 or L2 method, based on the `mode` argument.
///
/// # Input Layout (Array<felt252>)
///
/// - `input[0]`: mode (1 = L1, 2 = L2)
/// - `input[1]`: root
/// - `input[2]`: leaf
/// - `input[3]`: leaf index
/// - `input[4]`: MMR size
/// - `input[5]`: number of siblings
/// - `input[6..]`: sibling hashes
/// - After siblings: number of peaks
/// - Followed by: peak hashes
///
/// # Returns
///
/// - `0` if the proof is valid
/// - `1` if the proof is invalid
///
/// # Panics
///
/// - If the input is malformed or the mode is invalid
///
/// # Notes
///
/// Cairo requires `match` arms to start from 0, so we remap:
/// - mode 1 (L1) → 0
/// - mode 2 (L2) → 1
fn main(input: Array<felt252>) -> felt252 {
    assert(input.len() >= 7, 'Invalid input length');

    let mode = *input.at(0);
    let root = *input.at(1);
    let leaf = *input.at(2);

    let leaf_index: u32 = (*input.at(3)).try_into().unwrap();
    let mmr_size: u32 = (*input.at(4)).try_into().unwrap();
    let num_siblings: usize = (*input.at(5)).try_into().unwrap();

    let siblings_start = 6;
    let siblings_end = siblings_start + num_siblings;
    let num_peaks: usize = (*input.at(siblings_end)).try_into().unwrap();
    let peaks_start = siblings_end + 1;
    let peaks_end = peaks_start + num_peaks;

    let span = input.span();

    let siblings_span = span.slice(siblings_start, siblings_end);
    let peaks_span = span.slice(peaks_start, peaks_end);

    // this span conversion is neccesary for L1 proof verifier
    let mut siblings_array = ArrayTrait::<felt252>::new();
    for s in siblings_span {
        siblings_array.append(*s);
    }

    let mut peaks_array = ArrayTrait::<felt252>::new();
    for p in peaks_span {
        peaks_array.append(*p);
    }

    // pattern matching in cairo requires values to be sequential starting from 0.
    // So we map:
    //   mode = 1 (L1) => 0
    //   mode = 2 (L2) => 1
    // same way we count indexes from 0 upwards in arrays
    let mode_index: usize = mode.try_into().unwrap() - 1;

    let is_valid = match mode_index {
        0 => verify_l1(leaf, leaf_index, siblings_array, peaks_array, mmr_size, root),
        1 => verify_l2(
            leaf,
            leaf_index.try_into().unwrap(),
            siblings_span,
            root,
            mmr_size.into(),
            peaks_array,
        ),
        _ => panic!(),
    };

    // Return 0 for true, 1 for false
    if is_valid {
        0
    } else {
        1
    }
}
#[test]
#[available_gas(4000000)]
fn test_main_l1_and_l2_dispatch() {
    let leaf = 100;
    let leaf_index = 0;
    let root = leaf;
    let mmr_size = 1;

    let mode_l2 = 2;
    let mut input_l2 = ArrayTrait::<felt252>::new();
    input_l2.append(mode_l2);
    input_l2.append(root);       // root
    input_l2.append(leaf);       // leaf
    input_l2.append(leaf_index); // leaf index
    input_l2.append(mmr_size);   // mmr size
    input_l2.append(0);          // num_siblings
    input_l2.append(0);          // num_peaks

    let result_l2 = main(input_l2);
    assert(result_l2 == 0, 'L2 main dispatch failed');

    let mode_l1 = 1;
    let mut input_l1 = ArrayTrait::<felt252>::new();
    input_l1.append(mode_l1);
    input_l1.append(root);       // root
    input_l1.append(leaf);       // leaf
    input_l1.append(leaf_index); // leaf index
    input_l1.append(mmr_size);   // mmr size
    input_l1.append(0);          // num_siblings
    input_l1.append(0);          // num_peaks

    let result_l1 = main(input_l1);
    assert(result_l1 == 0, 'L1 main dispatch failed');
}

#[cfg(test)]
mod tests {
    use super::l2::verify_proof::{MmrProof, verify_proof, verify_proof_legacy};
    use super::verify_commitment_in_root;

    // Helper function to build a simple 4-leaf tree for tests
    fn build_test_tree() -> (felt252, felt252, felt252, felt252, felt252, felt252, felt252) {
        let leaf_0: felt252 = 10;
        let leaf_1: felt252 = 20;
        let leaf_2: felt252 = 30;
        let leaf_3: felt252 = 40;

        let h_01 = core::pedersen::pedersen(leaf_0, leaf_1);
        let h_23 = core::pedersen::pedersen(leaf_2, leaf_3);
        let root = core::pedersen::pedersen(h_01, h_23);

        (leaf_0, leaf_1, leaf_2, leaf_3, h_01, h_23, root)
    }

    #[test]
    #[available_gas(5000000)]
    fn test_correct_proof_passes() {
        let (leaf_0, leaf_1, leaf_2, leaf_3, h_01, h_23, root) = build_test_tree();

        let mut proof_0: Array = ArrayTrait::<felt252>::new();
        proof_0.append(leaf_1); // Sibling at level 0
        proof_0.append(h_23); // Sibling at level 1
        let verified_root_0 = verify_commitment_in_root(leaf_0, proof_0, root);
        assert(verified_root_0 == root, 'Leaf 0 verification failed');

        let mut proof_3: Array = ArrayTrait::<felt252>::new();
        proof_3.append(leaf_2); // Sibling at level 0
        proof_3.append(h_01); // Sibling at level 1
        let verified_root_3 = verify_commitment_in_root(leaf_3, proof_3, root);
        assert(verified_root_3 == root, 'Leaf 3 verification failed');
    }

    #[test]
    #[available_gas(2000000)]
    #[should_panic(expected: ('Computed root does not match',))]
    fn test_wrong_leaf_fails() {
        let (_, leaf_1, _, _, _, h_23, root) = build_test_tree();
        let mut proof_0 = ArrayTrait::<felt252>::new();
        proof_0.append(leaf_1);
        proof_0.append(h_23);

        let wrong_leaf = 999;
        verify_commitment_in_root(wrong_leaf, proof_0, root);
    }

    #[test]
    #[available_gas(2000000)]
    #[should_panic(expected: ('Computed root does not match',))]
    fn test_wrong_sibling_fails() {
        let (leaf_0, _, _, _, _, h_23, root) = build_test_tree();
        let mut proof_0_wrong = ArrayTrait::<felt252>::new();
        proof_0_wrong.append(998); // Wrong sibling for leaf 0
        proof_0_wrong.append(h_23);

        verify_commitment_in_root(leaf_0, proof_0_wrong, root);
    }

    #[test]
    #[available_gas(2000000)]
    #[should_panic(expected: ('Computed root does not match',))]
    fn test_wrong_root_fails() {
        let (leaf_0, leaf_1, _, _, _, h_23, _) = build_test_tree();
        let mut proof_0 = ArrayTrait::<felt252>::new();
        proof_0.append(leaf_1);
        proof_0.append(h_23);

        let wrong_root = 777;
        verify_commitment_in_root(leaf_0, proof_0, wrong_root);
    }

    #[test]
    #[available_gas(2000000)]
    #[should_panic]
    fn test_empty_proof_fails() {
        // Only run this test if tree height > 0
        let (leaf_0, _, _, _, _, _, root) = build_test_tree();
        let mut empty_proof = ArrayTrait::<felt252>::new();

        verify_commitment_in_root(leaf_0, empty_proof, root);
    }

    #[test]
    #[available_gas(3000000)]
    fn test_l2_verify_proof_integration() {
        // Test single element MMR case
        let leaf = 42;
        let leaf_index = 0;
        let proof: MmrProof = array![].span();
        let root = leaf; // For single element MMR, root equals leaf
        let merkle_size = 1;
        let peaks = array![leaf];

        let is_valid = verify_proof(leaf, leaf_index, proof, root, merkle_size, peaks);
        assert(is_valid, 'L2 verify_proof failed');
    }


    #[test]
    #[available_gas(2000000)]
    #[should_panic]
    fn test_malformed_proof_fails() {
        let (leaf_0, leaf_1, _, _, _, _, root) = build_test_tree();
        let mut short_proof = ArrayTrait::<felt252>::new();
        short_proof.append(leaf_1); // Only one element provided

        verify_commitment_in_root(leaf_0, short_proof, root);
    }
}
