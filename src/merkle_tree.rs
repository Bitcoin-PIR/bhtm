//! Append-only binary Merkle tree (Certificate Transparency RFC 6962 style).
//!
//! Each leaf is: H("muhash-leaf" ‖ height ‖ block_hash ‖ muhash_value)
//!
//! The tree uses a frontier-based append algorithm:
//!   - frontier[k] = root of the rightmost perfect subtree of size 2^k
//!   - frontier.len() = popcount(tree_size)
//!   - append merges the frontier bottom-up
//!
//! Tree root is computed by hashing frontier entries right-to-left.

use sha2::{Digest, Sha256};

/// Domain separator for leaf hashing.
const LEAF_DOMAIN: &[u8] = b"muhash-leaf";

/// The MuHash Merkle Tree: append-only, frontier-based.
///
/// After N insertions, the tree has N leaves. Tree root can be computed from
/// the frontier at any time. Frontier size is O(log N).
#[derive(Clone, Debug)]
pub struct MuHashMerkleTree {
    /// Frontier: right spine of the tree.
    /// frontier[k] = root of the rightmost complete subtree of size 2^k.
    frontier: Vec<[u8; 32]>,
    /// Total number of leaves inserted.
    size: u64,
    /// The root of the tree, updated on each append.
    root: [u8; 32],
}

/// Which side of the recomputed node a sibling hash occupies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiblingSide {
    Left,
    Right,
}

impl SiblingSide {
    pub fn as_str(self) -> &'static str {
        match self {
            SiblingSide::Left => "left",
            SiblingSide::Right => "right",
        }
    }
}

/// One bottom-up Merkle inclusion proof step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MerkleProofStep {
    pub side: SiblingSide,
    pub hash: [u8; 32],
}

impl MuHashMerkleTree {
    /// Create a new empty tree.
    pub fn new() -> Self {
        MuHashMerkleTree {
            frontier: Vec::new(),
            size: 0,
            root: [0u8; 32],
        }
    }

    /// Number of leaves in the tree.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Current Merkle root (hash of all leaves).
    pub fn root(&self) -> &[u8; 32] {
        &self.root
    }

    /// Current frontier (for checkpointing).
    pub fn frontier(&self) -> &[[u8; 32]] {
        &self.frontier
    }

    /// Compute the leaf hash for a given block.
    pub fn leaf_hash(height: u32, block_hash: &[u8; 32], muhash: &[u8; 384]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(LEAF_DOMAIN);
        hasher.update(&height.to_le_bytes());
        hasher.update(block_hash);
        hasher.update(muhash);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Append a leaf to the tree.
    pub fn append(&mut self, leaf: [u8; 32]) {
        let mut node = leaf;
        let mut i: usize = 0;
        let mut sz = self.size;

        // Merge frontier entries corresponding to set bits of old size
        while sz & 1 == 1 {
            node = hash_pair(&self.frontier[i], &node);
            i += 1;
            sz >>= 1;
        }

        // Remove consumed entries (indices 0..i-1) and insert new node
        self.frontier.drain(0..i);
        self.frontier.insert(0, node);

        self.size += 1;

        // Recompute root
        self.root = compute_root(&self.frontier);
    }

    /// Append a leaf for a specific block (computes leaf hash internally).
    pub fn append_block(&mut self, height: u32, block_hash: &[u8; 32], muhash: &[u8; 384]) {
        let leaf = Self::leaf_hash(height, block_hash, muhash);
        self.append(leaf);
    }
}

impl Default for MuHashMerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Hash a pair of 32-byte nodes: H(left ‖ right).
fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(left);
    hasher.update(right);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Compute the same append-tree root from a full leaf vector.
pub fn root_from_leaves(leaves: &[[u8; 32]]) -> [u8; 32] {
    if leaves.is_empty() {
        return [0u8; 32];
    }
    root_from_leaf_range(leaves)
}

/// Generate a bottom-up inclusion proof for `leaf_index`.
pub fn inclusion_proof(
    leaves: &[[u8; 32]],
    leaf_index: usize,
) -> Result<Vec<MerkleProofStep>, String> {
    if leaves.is_empty() {
        return Err("cannot prove a leaf in an empty tree".into());
    }
    if leaf_index >= leaves.len() {
        return Err(format!(
            "leaf index {} out of range for tree of size {}",
            leaf_index,
            leaves.len()
        ));
    }
    let mut proof = Vec::new();
    inclusion_proof_range(leaves, leaf_index, &mut proof);
    Ok(proof)
}

/// Verify a bottom-up inclusion proof against this tree shape and root.
pub fn verify_inclusion_proof(
    leaf: [u8; 32],
    leaf_index: usize,
    tree_size: usize,
    proof: &[MerkleProofStep],
    expected_root: [u8; 32],
) -> Result<(), String> {
    if tree_size == 0 {
        return Err("cannot verify a leaf in an empty tree".into());
    }
    if leaf_index >= tree_size {
        return Err(format!(
            "leaf index {} out of range for tree of size {}",
            leaf_index, tree_size
        ));
    }

    let mut proof_index = 0usize;
    let root = verify_inclusion_range(leaf, leaf_index, tree_size, proof, &mut proof_index)?;
    if proof_index != proof.len() {
        return Err(format!(
            "unused proof steps: consumed {}, got {}",
            proof_index,
            proof.len()
        ));
    }
    if root != expected_root {
        return Err(format!(
            "proof root mismatch: expected {}, got {}",
            hex::encode(expected_root),
            hex::encode(root)
        ));
    }
    Ok(())
}

fn root_from_leaf_range(leaves: &[[u8; 32]]) -> [u8; 32] {
    match leaves.len() {
        0 => [0u8; 32],
        1 => leaves[0],
        n => {
            let split = largest_power_of_two_less_than(n);
            let left = root_from_leaf_range(&leaves[..split]);
            let right = root_from_leaf_range(&leaves[split..]);
            hash_pair(&left, &right)
        }
    }
}

fn inclusion_proof_range(leaves: &[[u8; 32]], leaf_index: usize, proof: &mut Vec<MerkleProofStep>) {
    if leaves.len() == 1 {
        return;
    }

    let split = largest_power_of_two_less_than(leaves.len());
    if leaf_index < split {
        inclusion_proof_range(&leaves[..split], leaf_index, proof);
        proof.push(MerkleProofStep {
            side: SiblingSide::Right,
            hash: root_from_leaf_range(&leaves[split..]),
        });
    } else {
        inclusion_proof_range(&leaves[split..], leaf_index - split, proof);
        proof.push(MerkleProofStep {
            side: SiblingSide::Left,
            hash: root_from_leaf_range(&leaves[..split]),
        });
    }
}

fn verify_inclusion_range(
    node: [u8; 32],
    leaf_index: usize,
    tree_size: usize,
    proof: &[MerkleProofStep],
    proof_index: &mut usize,
) -> Result<[u8; 32], String> {
    if tree_size == 1 {
        return Ok(node);
    }

    let split = largest_power_of_two_less_than(tree_size);
    if leaf_index < split {
        let left = verify_inclusion_range(node, leaf_index, split, proof, proof_index)?;
        let step = proof
            .get(*proof_index)
            .ok_or_else(|| "proof ended before right sibling".to_string())?;
        *proof_index += 1;
        if step.side != SiblingSide::Right {
            return Err(format!(
                "proof step {} should be a right sibling",
                proof_index.saturating_sub(1)
            ));
        }
        Ok(hash_pair(&left, &step.hash))
    } else {
        let right = verify_inclusion_range(
            node,
            leaf_index - split,
            tree_size - split,
            proof,
            proof_index,
        )?;
        let step = proof
            .get(*proof_index)
            .ok_or_else(|| "proof ended before left sibling".to_string())?;
        *proof_index += 1;
        if step.side != SiblingSide::Left {
            return Err(format!(
                "proof step {} should be a left sibling",
                proof_index.saturating_sub(1)
            ));
        }
        Ok(hash_pair(&step.hash, &right))
    }
}

fn largest_power_of_two_less_than(n: usize) -> usize {
    debug_assert!(n >= 2);
    1usize << (usize::BITS as usize - 1 - (n - 1).leading_zeros() as usize)
}

/// Compute the Merkle root from the frontier.
/// Frontier entries are in order from newest (index 0) to oldest (highest index).
/// The root combines them with older subtrees on the LEFT.
fn compute_root(frontier: &[[u8; 32]]) -> [u8; 32] {
    if frontier.is_empty() {
        return [0u8; 32];
    }
    let mut root = frontier[0];
    for i in 1..frontier.len() {
        root = hash_pair(&frontier[i], &root);
    }
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_tree() {
        let tree = MuHashMerkleTree::new();
        assert_eq!(tree.size(), 0);
        assert_eq!(*tree.root(), [0u8; 32]);
    }

    #[test]
    fn test_single_leaf() {
        let mut tree = MuHashMerkleTree::new();
        let leaf = [0x11; 32];
        tree.append(leaf);
        assert_eq!(tree.size(), 1);
        // Root of a single-leaf tree is the leaf itself
        assert_eq!(*tree.root(), leaf);
    }

    #[test]
    fn test_two_leaves() {
        let mut tree = MuHashMerkleTree::new();
        let leaf0 = [0xAA; 32];
        let leaf1 = [0xBB; 32];
        tree.append(leaf0);
        tree.append(leaf1);

        let expected_root = hash_pair(&leaf0, &leaf1);
        assert_eq!(*tree.root(), expected_root);
    }

    #[test]
    fn test_three_leaves() {
        let mut tree = MuHashMerkleTree::new();
        let l0 = [0x00; 32];
        let l1 = [0x01; 32];
        let l2 = [0x02; 32];

        tree.append(l0);
        tree.append(l1);
        tree.append(l2);

        // Tree: (H(l0, l1), l2) → root = H(H(l0,l1), l2)
        let expected = hash_pair(&hash_pair(&l0, &l1), &l2);
        assert_eq!(*tree.root(), expected);
    }

    #[test]
    fn test_frontier_reconstruction() {
        // Build tree of 7 leaves, capture frontier, reconstruct
        let mut tree = MuHashMerkleTree::new();
        for i in 0u8..7 {
            tree.append([i; 32]);
        }
        let root_from_tree = *tree.root();

        // Reconstruct from frontier
        let reconstructed = compute_root(tree.frontier());
        assert_eq!(root_from_tree, reconstructed);
    }

    #[test]
    fn test_deterministic_leaf_hash() {
        let hash1 = MuHashMerkleTree::leaf_hash(0, &[0xAA; 32], &[0xBB; 384]);
        let hash2 = MuHashMerkleTree::leaf_hash(0, &[0xAA; 32], &[0xBB; 384]);
        assert_eq!(hash1, hash2);

        // Different height → different hash
        let hash3 = MuHashMerkleTree::leaf_hash(1, &[0xAA; 32], &[0xBB; 384]);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_large_tree() {
        let mut tree = MuHashMerkleTree::new();
        for i in 0u32..1000 {
            tree.append([(i % 256) as u8; 32]);
        }
        assert_eq!(tree.size(), 1000);
    }

    #[test]
    fn test_leaf_inclusion_proofs_roundtrip() {
        for size in 1usize..20 {
            let leaves = (0..size)
                .map(|i| {
                    let mut leaf = [0u8; 32];
                    leaf[0] = i as u8;
                    leaf[31] = (size - i) as u8;
                    leaf
                })
                .collect::<Vec<_>>();
            let root = root_from_leaves(&leaves);

            let mut append_tree = MuHashMerkleTree::new();
            for leaf in &leaves {
                append_tree.append(*leaf);
            }
            assert_eq!(root, *append_tree.root());

            for (idx, leaf) in leaves.iter().enumerate() {
                let proof = inclusion_proof(&leaves, idx).unwrap();
                verify_inclusion_proof(*leaf, idx, size, &proof, root).unwrap();
            }
        }
    }

    #[test]
    fn test_leaf_inclusion_proof_rejects_wrong_index() {
        let leaves = [[0u8; 32], [1u8; 32], [2u8; 32]];
        let root = root_from_leaves(&leaves);
        let proof = inclusion_proof(&leaves, 0).unwrap();
        let err = verify_inclusion_proof(leaves[0], 1, leaves.len(), &proof, root).unwrap_err();
        assert!(err.contains("should be"));
    }
}
