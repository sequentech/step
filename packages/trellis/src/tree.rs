use crate::LeafHash;
use ct_merkle::ConsistencyProof;
use ct_merkle::InclusionProof;
use ct_merkle::mem_backed_tree::MemoryBackedTree;
use sha2::Sha256;
use std::collections::HashMap;

/// Error type for proof operations
#[derive(Debug)]
pub enum ProofError {
    /// The requested leaf hash was not found in the tree
    LeafNotFound,
    /// The requested root hash was not found in the tree's history
    RootNotFound,
    /// The leaf exists but wasn't present at the specified tree state
    LeafNotPresentAtTree,
    /// The old root must have a smaller tree size than the current root
    InvalidRootOrder,
    /// The tree rewind operation failed
    RewindError(RewindError),
    /// The requested size is too large to fit into a usize
    SizeTooLarge,
}

/// Error type for proof operations
#[derive(Debug)]
pub enum VerificationError {
    /// The requested leaf hash was not found in the tree
    LeafNotFound,
    /// The requested root hash was not found in the tree's history
    RootNotFound,
    /// The leaf exists but wasn't present at the specified tree state
    LeafNotPresentAtTree,
    /// The underlying tree returned an inclusion verification error
    CtInclusionError(ct_merkle::InclusionVerifError),
    /// The underlying tree returned a consistency verification error
    CtConsistencyError(ct_merkle::ConsistencyVerifError),
    /// The tree rewind operation failed
    RewindError(RewindError),
}

/// Error type for tree rewinding operations
#[derive(Debug)]
pub enum RewindError {
    /// The target root hash was not found in the tree's history
    RootNotFound,
    /// The tree's actual root doesn't match the expected root after rewinding
    UnexpectedRoot(Vec<u8>),
    /// The requested leaf index is too large to fit into a usize
    SizeTooLarge,
}

/// SHA-256 hash size in bytes
pub const LEAF_HASH_SIZE: u64 = 32;

/// A merkle tree implementation based on Certificate Transparency that maintains root history and provides proofs
pub struct CtMerkleTree {
    /// The underlying CT merkle tree implementation
    tree: MemoryBackedTree<Sha256, LeafHash>,
    /// Maps leaf hash to its position (index) in the tree
    leaf_hash_to_index: HashMap<Vec<u8>, u64>,
    /// Maps root hash to the tree size that produced it
    root_hash_to_size: HashMap<Vec<u8>, u64>,
}

impl CtMerkleTree {
    /// Creates a new empty merkle tree
    #[must_use]
    pub fn new() -> Self {
        Self {
            tree: MemoryBackedTree::new(),
            leaf_hash_to_index: HashMap::new(),
            root_hash_to_size: HashMap::new(),
        }
    }

    /// Adds a new leaf to the tree and updates the associated maps
    pub fn push(&mut self, leaf: LeafHash) {
        let idx = self.tree.len();
        self.leaf_hash_to_index.insert(leaf.hash.clone(), idx);
        self.tree.push(leaf);
        // for the moment we must checkpoint after each addition,
        // because in a rebuild scenario we do not know which roots have been published
        self.root_checkpoint();
    }

    // for the moment we must checkpoint after each addition,
    // because in a rebuild scenario we do not know which roots have been published
    // these checkpoints are currently part of update_with_entry, and
    // cannot yet be called separately.
    /// Set a root checkpoint for the current tree state
    fn root_checkpoint(&mut self) {
        let root = self.tree.root();
        self.root_hash_to_size
            .insert(root.as_bytes().to_vec(), self.tree.len());
    }

    /// Gets the current root hash
    #[must_use]
    pub fn root(&self) -> Vec<u8> {
        self.tree.root().as_bytes().to_vec()
    }

    /// Gets the current number of leaves in the tree
    #[must_use]
    pub fn len(&self) -> u64 {
        self.tree.len()
    }

    /// Returns true if the tree is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tree.len() == 0
    }

    /// Generates an inclusion proof for a given leaf hash against the current root
    ///
    /// # Errors
    ///
    /// - `ProofError`: if proof generation fails
    pub fn prove_inclusion(&self, hash: &[u8]) -> Result<InclusionProof<Sha256>, ProofError> {
        self.prove_inclusion_at_root(hash, &self.root())
    }

    /// Generates a consistency proof between the current tree and a historical root
    ///
    /// # Errors
    ///
    /// - `ProofError`: if proof generation fails
    pub fn prove_consistency(
        &self,
        old_root: &[u8],
    ) -> Result<ConsistencyProof<Sha256>, ProofError> {
        self.prove_consistency_between(old_root, &self.root())
    }

    /// Verifies an inclusion proof for a given leaf hash against the current root
    ///
    /// # Errors
    ///
    /// - `VerificationError`: if the proof verification fails
    pub fn verify_inclusion(
        &self,
        leaf_hash: &[u8],
        proof: &InclusionProof<Sha256>,
    ) -> Result<(), VerificationError> {
        self.verify_inclusion_at_root(leaf_hash, &self.root(), proof)
    }

    /// Verifies a consistency proof between a historical root and the current root
    ///
    /// # Errors
    ///
    /// - `VerificationError`:  if the proof verification fails
    pub fn verify_consistency(
        &self,
        old_root: &[u8],
        proof: &ConsistencyProof<Sha256>,
    ) -> Result<(), VerificationError> {
        self.verify_consistency_between(old_root, &self.root(), proof)
    }

    /// Gets the index for a given leaf hash
    #[must_use]
    pub fn get_index(&self, hash: &[u8]) -> Option<u64> {
        self.leaf_hash_to_index.get(hash).copied()
    }

    /// Gets the tree size for a given root hash
    #[must_use]
    pub fn get_size_for_root(&self, root: &[u8]) -> Option<u64> {
        self.root_hash_to_size.get(root).copied()
    }

    /// Returns a tree corresponding to the historical state at the given root hash.
    ///
    /// # Errors
    ///
    /// - `RewindError::RootNotFound`: if the root hash does not exist in the tree's history,
    /// - `RewindError::UnexpectedRoot`: if the rewound tree's root does not match the expected root.
    pub(crate) fn rewind(
        &self,
        historical_root: &[u8],
    ) -> Result<MemoryBackedTree<Sha256, LeafHash>, RewindError> {
        // Get size from root hash to size map
        let size = self
            .root_hash_to_size
            .get(historical_root)
            .ok_or(RewindError::RootNotFound)?;

        // Create new tree
        let mut historical_tree = MemoryBackedTree::new();

        let size_usize = usize::try_from(*size).map_err(|_| RewindError::SizeTooLarge)?;

        // Take first 'size' leaves from current tree
        for leaf in self.tree.items().iter().take(size_usize) {
            historical_tree.push(leaf.clone());
        }

        // Get root bytes for comparison
        let root_bytes = historical_tree.root().as_bytes().to_vec();

        // Verify we got the expected root
        if root_bytes != historical_root {
            return Err(RewindError::UnexpectedRoot(root_bytes));
        }

        Ok(historical_tree)
    }

    /// Generates an inclusion proof for a given leaf hash at a specific historical root state
    ///
    /// # Errors
    ///
    /// - `ProofError::LeafNotFound`: if the leaf hash does not exist in the tree,
    /// - `ProofError::LeafNotPresentAtTree`: if the leaf was added after the specified root.
    /// - `ProofError::RewindError`: if rewinding to the specified root fails.
    pub fn prove_inclusion_at_root(
        &self,
        leaf_hash: &[u8],
        root: &[u8],
    ) -> Result<InclusionProof<Sha256>, ProofError> {
        if root == self.root() {
            return self.prove_inclusion_for_tree(&self.tree, leaf_hash);
        }

        // Get historical tree state
        let historical_tree = self.rewind(root).map_err(ProofError::RewindError)?;

        self.prove_inclusion_for_tree(&historical_tree, leaf_hash)
    }

    /// Verifies an inclusion proof for a given leaf hash against a specific historical root
    ///
    /// # Errors
    ///
    /// - `VerificationError::LeafNotFound`: if the leaf hash does not exist in the tree,
    /// - `VerificationError::LeafNotPresentAtTree`: if the leaf was added after the specified root.
    /// - `VerificationError::RewindError`: if rewinding to the specified root fails.
    /// - `VerificationError::CtInclusionError`: if the underlying proof verification fails.
    pub fn verify_inclusion_at_root(
        &self,
        leaf_hash: &[u8],
        root: &[u8],
        proof: &InclusionProof<Sha256>,
    ) -> Result<(), VerificationError> {
        if root == self.root() {
            return self.verify_inclusion_for_tree(&self.tree, leaf_hash, proof);
        }

        // Get historical tree state
        let historical_tree = match self.rewind(root) {
            Ok(tree) => tree,
            Err(e) => return Err(VerificationError::RewindError(e)),
        };

        self.verify_inclusion_for_tree(&historical_tree, leaf_hash, proof)
    }

    /// Generates a consistency proof between any two historical roots
    ///
    /// If both roots are identical, returns an empty proof (trivially valid).
    ///
    /// # Errors
    ///
    /// - `ProofError::RootNotFound`: if either root does not exist in the tree's history,
    /// - `ProofError::InvalidRootOrder`: if the old root is from a tree that is larger than the new root's tree.
    /// - `ProofError::RewindError`: if rewinding to `new_root` fails
    ///
    /// # Panics
    ///
    /// Infallible: `new_size` >= `old_size`
    pub fn prove_consistency_between(
        &self,
        old_root: &[u8],
        new_root: &[u8],
    ) -> Result<ConsistencyProof<Sha256>, ProofError> {
        // Get sizes for both roots
        let old_size = *self
            .root_hash_to_size
            .get(old_root)
            .ok_or(ProofError::RootNotFound)?;
        let new_size = *self
            .root_hash_to_size
            .get(new_root)
            .ok_or(ProofError::RootNotFound)?;

        // New root must be from an equal or larger tree
        if new_size < old_size {
            return Err(ProofError::InvalidRootOrder);
        }

        // Calculate number of additions
        let num_additions: u64 = new_size
            .checked_sub(old_size)
            .expect("new_size >= old_size by above check");

        // Get tree state at new_root
        let new_tree = if new_root == self.root() {
            &self.tree
        } else {
            &self.rewind(new_root).map_err(ProofError::RewindError)?
        };

        let num_additions_usize =
            usize::try_from(num_additions).map_err(|_| ProofError::SizeTooLarge)?;

        // Generate proof from the historical tree state
        Ok(new_tree.prove_consistency(num_additions_usize))
    }

    /// Verifies a consistency proof between any two historical roots
    ///
    /// # Errors
    ///
    /// - `RootNotFound`: if the `old_root` was not found
    /// - `RewindError`: if rewinding to `new_root` failed
    /// - `CtConsistencyError`: if the underlying proof verification fails
    pub fn verify_consistency_between(
        &self,
        old_root: &[u8],
        new_root: &[u8],
        proof: &ConsistencyProof<Sha256>,
    ) -> Result<(), VerificationError> {
        // Get sizes for both roots
        let old_size = match self.root_hash_to_size.get(old_root) {
            Some(size) => *size,
            None => return Err(VerificationError::RootNotFound),
        };

        // Rewind to new root state if needed
        let rewound_tree;
        let new_tree = if new_root == self.root() {
            &self.tree
        } else {
            rewound_tree = match self.rewind(new_root) {
                Ok(tree) => tree,
                Err(e) => return Err(VerificationError::RewindError(e)),
            };
            &rewound_tree
        };

        // Create root hash from old root bytes
        let mut old_digest = sha2::digest::Output::<Sha256>::default();
        old_digest.copy_from_slice(old_root);
        let old_root_hash = ct_merkle::RootHash::new(old_digest, old_size);

        // Verify the proof using the new tree's root
        new_tree
            .root()
            .verify_consistency(&old_root_hash, proof)
            .map_err(VerificationError::CtConsistencyError)
    }

    /// Helper to generate inclusion proof for a given tree and leaf hash
    fn prove_inclusion_for_tree(
        &self,
        tree: &MemoryBackedTree<Sha256, LeafHash>,
        leaf_hash: &[u8],
    ) -> Result<InclusionProof<Sha256>, ProofError> {
        // Get index for the leaf
        let idx = self
            .leaf_hash_to_index
            .get(leaf_hash)
            .ok_or(ProofError::LeafNotFound)?;

        // Check if leaf is actually in the historical state
        if *idx >= tree.len() {
            return Err(ProofError::LeafNotPresentAtTree);
        }

        let idx_usize = usize::try_from(*idx).map_err(|_| ProofError::SizeTooLarge)?;

        Ok(tree.prove_inclusion(idx_usize))
    }

    /// Helper to verify inclusion proof for a given tree and leaf hash
    fn verify_inclusion_for_tree(
        &self,
        tree: &MemoryBackedTree<Sha256, LeafHash>,
        leaf_hash: &[u8],
        proof: &InclusionProof<Sha256>,
    ) -> Result<(), VerificationError> {
        // Get index for this leaf hash
        let idx = match self.leaf_hash_to_index.get(leaf_hash) {
            Some(idx) => *idx,
            None => return Err(VerificationError::LeafNotFound),
        };

        // Create leaf hash wrapper
        let leaf = LeafHash::new(leaf_hash.to_vec());

        // Check if leaf is in the historical state
        if idx >= tree.len() {
            return Err(VerificationError::LeafNotPresentAtTree);
        }

        let root = tree.root();
        root.verify_inclusion(&leaf, idx, proof)
            .map_err(VerificationError::CtInclusionError)
    }
}

impl Default for CtMerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tree() -> CtMerkleTree {
        let mut tree = CtMerkleTree::new();
        // Add a few leaves
        let leaf1 = LeafHash::new(vec![1, 2, 3]);
        let leaf2 = LeafHash::new(vec![4, 5, 6]);
        let leaf3 = LeafHash::new(vec![7, 8, 9]);

        tree.push(leaf1.clone());
        tree.push(leaf2.clone());
        tree.push(leaf3.clone());

        tree
    }

    #[test]
    fn test_valid_inclusion_proof() {
        let tree = create_test_tree();
        let leaf_hash = vec![1, 2, 3];

        // Generate proof for first leaf
        let proof = tree
            .prove_inclusion(&leaf_hash)
            .expect("Should create proof");

        // Verify the proof
        let result = tree.verify_inclusion(&leaf_hash, &proof);
        assert!(
            result.is_ok(),
            "Valid inclusion proof should verify successfully"
        );
    }

    #[test]
    fn test_invalid_inclusion_proof() {
        let tree = create_test_tree();
        let leaf_hash = vec![1, 2, 3];
        let wrong_hash = vec![9, 9, 9];

        // Generate proof for first leaf
        let proof = tree
            .prove_inclusion(&leaf_hash)
            .expect("Should create proof");

        // Try to verify proof with wrong leaf hash
        let result = tree.verify_inclusion(&wrong_hash, &proof);
        assert!(
            result.is_err(),
            "Invalid inclusion proof should fail verification"
        );
    }

    #[test]
    fn test_valid_consistency_proof() {
        let mut tree = CtMerkleTree::new();

        // Add first leaf and save root
        let leaf1 = LeafHash::new(vec![1, 2, 3]);
        tree.push(leaf1);
        let old_root = tree.root();

        // Add more leaves
        let leaf2 = LeafHash::new(vec![4, 5, 6]);
        tree.push(leaf2);

        // Generate and verify consistency proof
        let proof = tree
            .prove_consistency(&old_root)
            .expect("Should create proof");
        let result = tree.verify_consistency(&old_root, &proof);

        assert!(
            result.is_ok(),
            "Valid consistency proof should verify successfully"
        );
    }

    #[test]
    fn test_invalid_consistency_proof() {
        let mut tree = CtMerkleTree::new();

        // Add first leaf and save root
        let leaf1 = LeafHash::new(vec![1, 2, 3]);
        tree.push(leaf1);
        let first_root = tree.root();

        // Add second leaf to get a different root
        let leaf2 = LeafHash::new(vec![4, 5, 6]);
        tree.push(leaf2);

        // Create fake root
        let fake_root = vec![9; 32]; // Wrong root of correct length

        // Generate proof using first root
        let proof = tree
            .prove_consistency(&first_root)
            .expect("Should create proof");

        // Try to verify with fake root
        let result = tree.verify_consistency(&fake_root, &proof);

        assert!(
            result.is_err(),
            "Consistency proof with wrong root should fail verification"
        );
    }

    #[test]
    fn test_consistency_proof_with_current_root() {
        let mut tree = CtMerkleTree::new();

        // Add a few leaves to create a non-empty tree
        tree.push(LeafHash::new(vec![1, 2, 3]));
        tree.push(LeafHash::new(vec![4, 5, 6]));

        // Get current root
        let current_root = tree.root();

        // Generating a consistency proof for the same root should succeed with an empty proof
        let proof = tree
            .prove_consistency(&current_root)
            .expect("Same root should return valid proof");

        // Proof should be empty (trivially valid)
        assert!(
            proof.as_bytes().is_empty(),
            "Same root consistency proof should have empty path"
        );

        // Proof should verify successfully
        tree.verify_consistency(&current_root, &proof)
            .expect("Same root consistency proof should verify");
    }

    #[test]
    fn test_rewind() {
        let mut tree = CtMerkleTree::new();

        // Add three leaves and save roots after each
        let leaf1 = LeafHash::new(vec![1, 2, 3]);
        tree.push(leaf1.clone());
        let root1 = tree.root();

        let leaf2 = LeafHash::new(vec![4, 5, 6]);
        tree.push(leaf2.clone());
        let root2 = tree.root();

        let leaf3 = LeafHash::new(vec![7, 8, 9]);
        tree.push(leaf3);

        // Try to rewind to second root
        let historical_tree_2 = tree.rewind(&root2).expect("Should rewind to second root");
        assert_eq!(
            historical_tree_2.len(),
            2,
            "Historical tree should have 2 leaves"
        );
        assert_eq!(
            historical_tree_2.root().as_bytes().to_vec(),
            root2,
            "Historical tree should have correct root"
        );

        // Try to rewind to first root
        let historical_tree_1 = tree.rewind(&root1).expect("Should rewind to first root");
        assert_eq!(
            historical_tree_1.len(),
            1,
            "Historical tree should have 1 leaf"
        );
        assert_eq!(
            historical_tree_1.root().as_bytes().to_vec(),
            root1,
            "Historical tree should have correct root"
        );

        // Try to rewind to non-existent root
        let fake_root = vec![9; 32];
        assert!(matches!(
            tree.rewind(&fake_root),
            Err(RewindError::RootNotFound)
        ));
    }

    #[test]
    fn test_inclusion_at_root() {
        let mut tree = CtMerkleTree::new();

        // Add three leaves and save roots after each
        let leaf1_hash = vec![1, 2, 3];
        tree.push(LeafHash::new(leaf1_hash.clone()));
        let root1 = tree.root();

        let leaf2_hash = vec![4, 5, 6];
        tree.push(LeafHash::new(leaf2_hash.clone()));
        let root2 = tree.root();

        let leaf3_hash = vec![7, 8, 9];
        tree.push(LeafHash::new(leaf3_hash.clone()));

        // Test proving/verifying inclusion at various historical roots

        // For leaf1 at root1 (initial state)
        let proof1 = tree
            .prove_inclusion_at_root(&leaf1_hash, &root1)
            .expect("Should create proof");
        assert!(
            tree.verify_inclusion_at_root(&leaf1_hash, &root1, &proof1)
                .is_ok(),
            "Should verify"
        );

        // For leaf1 at root2 (still present)
        let proof2 = tree
            .prove_inclusion_at_root(&leaf1_hash, &root2)
            .expect("Should create proof");
        assert!(
            tree.verify_inclusion_at_root(&leaf1_hash, &root2, &proof2)
                .is_ok(),
            "Should verify"
        );

        // For leaf2 at root1 (wasn't present yet)
        assert!(matches!(
            tree.prove_inclusion_at_root(&leaf2_hash, &root1),
            Err(ProofError::LeafNotPresentAtTree)
        ));

        // For leaf2 at root2 (just added)
        let proof3 = tree
            .prove_inclusion_at_root(&leaf2_hash, &root2)
            .expect("Should create proof");
        assert!(
            tree.verify_inclusion_at_root(&leaf2_hash, &root2, &proof3)
                .is_ok(),
            "Should verify"
        );

        // For non-existent root
        let fake_root = vec![9; 32];
        assert!(matches!(
            tree.prove_inclusion_at_root(&leaf1_hash, &fake_root),
            Err(ProofError::RewindError(RewindError::RootNotFound))
        ));
        assert!(
            tree.verify_inclusion_at_root(&leaf1_hash, &fake_root, &proof1)
                .is_err(),
            "Should return false"
        );

        // For incorrect proof (using proof from different root)
        assert!(
            tree.verify_inclusion_at_root(&leaf1_hash, &root2, &proof1)
                .is_err(),
            "Should return false"
        );
    }

    #[test]
    fn test_consistency_between_roots() {
        let mut tree = CtMerkleTree::new();

        // Add three leaves and save roots after each
        let leaf1 = LeafHash::new(vec![1, 2, 3]);
        tree.push(leaf1);
        let root1 = tree.root();

        let leaf2 = LeafHash::new(vec![4, 5, 6]);
        tree.push(leaf2);
        let root2 = tree.root();

        let leaf3 = LeafHash::new(vec![7, 8, 9]);
        tree.push(leaf3);
        let root3 = tree.root();

        // Test valid consistency proofs between roots

        // Between root1 and root2
        let proof1 = tree
            .prove_consistency_between(&root1, &root2)
            .expect("Should create proof between root1 and root2");
        assert!(
            tree.verify_consistency_between(&root1, &root2, &proof1)
                .is_ok(),
            "Should verify between root1 and root2"
        );

        // Between root1 and root3
        let proof2 = tree
            .prove_consistency_between(&root1, &root3)
            .expect("Should create proof between root1 and root3");
        assert!(
            tree.verify_consistency_between(&root1, &root3, &proof2)
                .is_ok(),
            "Should verify between root1 and root3"
        );

        // Between root2 and root3
        let proof3 = tree
            .prove_consistency_between(&root2, &root3)
            .expect("Should create proof between root2 and root3");
        assert!(
            tree.verify_consistency_between(&root2, &root3, &proof3)
                .is_ok(),
            "Should verify between root2 and root3"
        );

        // Test invalid cases

        // Reversed order (new_root before old_root)
        assert!(matches!(
            tree.prove_consistency_between(&root2, &root1),
            Err(ProofError::InvalidRootOrder)
        ));

        // Same root should succeed with empty proof
        let same_root_proof = tree
            .prove_consistency_between(&root1, &root1)
            .expect("Same root should succeed");
        assert!(
            same_root_proof.as_bytes().is_empty(),
            "Same root consistency proof should have empty path"
        );
        // And verify successfully
        tree.verify_consistency_between(&root1, &root1, &same_root_proof)
            .expect("Same root consistency proof should verify");

        // Non-existent root
        let fake_root = vec![9; 32];
        assert!(matches!(
            tree.prove_consistency_between(&root1, &fake_root),
            Err(ProofError::RootNotFound)
        ));
        assert!(
            tree.verify_consistency_between(&root1, &fake_root, &proof1)
                .is_err(),
            "Should return false"
        );

        // Wrong proof (using proof between different roots)
        assert!(
            tree.verify_consistency_between(&root1, &root3, &proof1)
                .is_err(),
            "Should return false"
        );
    }
}
