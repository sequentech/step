//! Trellis - PostgreSQL-integrated Merkle Tree Verification Service
//!
//! This crate provides an addon service to existing postgresql database that
//! maintains multiple independent Certificate Transparency-style merkle logs. It enables
//! cryptographic verification of append-only logs through inclusion and consistency proofs.
//!
//! # Proofs
//!
//! The HTTP endpoints provide cryptographic proofs that data entries:
//! - **Exist in the log** (inclusion proofs)
//! - **Were never removed or modified** (append-only consistency proofs)
//!
//! # Architecture
//!
//! - **Multiple Independent Logs**: Each log tracks different source tables
//! - **Continuous Processing**: Background batch processor merges data from configured sources
//! - **HTTP API**: HTTP endpoints to interact with the logs and request proofs
//! - **In-memory Proof Generation**: In-memory merkle trees for fast proof generation
//!
//! # Modules
//!
//! - [`service`]: HTTP server and batch processing logic
//! - [`tree`]: Merkle tree implementation with proof generation, based on the `ct-merkle` crate

use anyhow::Result;
use ct_merkle::{ConsistencyProof as CtConsistencyProof, InclusionProof as CtInclusionProof};
use ct_merkle::{HashableLeaf, RootHash};
use digest::Update;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, digest::Output};

/// HTTP service layer for merkle log verification
///
/// Contains the web server, client library, batch processor, and all HTTP route handlers.
/// This is the main interface for interacting with merkle logs over the network.
#[cfg(feature = "upstream-service")]
pub mod service;

/// Transactional PostgreSQL journal and proof processing.
pub mod journal;

/// Merkle tree data structures and proof generation
///
/// Provides the core merkle tree implementation with support for inclusion proofs,
/// consistency proofs.
pub mod tree;

// Re-export service layer components
#[cfg(feature = "upstream-service")]
pub use service::{
    ConsistencyQuery, InclusionQuery, MerkleState, get_consistency_proof, get_inclusion_proof,
    get_log_size, get_merkle_root, state::AppState,
};

/// A wrapper around a merkle inclusion proof with metadata needed for external verification
#[derive(Debug, Serialize, Deserialize)]
pub struct InclusionProof {
    /// The index of the leaf in the tree
    pub index: u64,
    /// The root hash at the time the proof was generated
    pub root: Vec<u8>,
    /// The inclusion proof path as raw bytes
    pub proof_bytes: Vec<u8>,
    /// The total number of leaves in the tree at the time of proof generation
    pub tree_size: u64,
}
impl InclusionProof {
    /// Verifies membership in the root carried by this proof.
    ///
    /// This does not authenticate the log. Use `verify_against` with an independently
    /// trusted checkpoint to establish membership in a particular log.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid sizes, indices, roots, or proof paths.
    pub fn verify(&self, hash: &[u8]) -> Result<(), ct_merkle::InclusionVerifError> {
        // ct-merkle uses doubled u64 node indices and panics outside this domain.
        if self.tree_size == 0 || self.tree_size > (1_u64 << 63) || self.index >= self.tree_size {
            return Err(ct_merkle::InclusionVerifError::MalformedProof);
        }
        // Create the leaf hash from the provided hash
        let leaf_hash = LeafHash::new(hash.to_vec());

        // Create a digest from our stored root bytes
        let mut digest = Output::<Sha256>::default();
        if self.root.len() != digest.len() {
            return Err(ct_merkle::InclusionVerifError::MalformedProof);
        }
        digest.copy_from_slice(&self.root);

        // Create the root hash with the digest and actual tree size
        let root_hash = RootHash::<Sha256>::new(digest, self.tree_size);

        // Create the inclusion proof from our stored bytes
        let proof = CtInclusionProof::<Sha256>::from_bytes(self.proof_bytes.clone());

        // Verify using root's verification method
        root_hash.verify_inclusion(&leaf_hash, self.index, &proof)
    }
    /// Verifies inclusion against an independently trusted root and tree size.
    ///
    /// # Errors
    /// Returns an error if the checkpoint differs or the proof is invalid.
    pub fn verify_against(
        &self,
        hash: &[u8],
        trusted_root: &[u8],
        trusted_tree_size: u64,
    ) -> Result<(), ct_merkle::InclusionVerifError> {
        if self.root != trusted_root || self.tree_size != trusted_tree_size {
            return Err(ct_merkle::InclusionVerifError::MalformedProof);
        }
        self.verify(hash)
    }
}

/// A wrapper around a merkle consistency proof that proves one tree is a prefix of another
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsistencyProof {
    /// The number of leaves in the old (smaller) tree
    pub old_tree_size: u64,
    /// The consistency proof path bytes
    pub proof_bytes: Vec<u8>,
    /// The root hash of the new (larger) tree
    pub new_root: Vec<u8>,
    /// The total number of leaves in the new tree
    pub new_tree_size: u64,
}

impl ConsistencyProof {
    /// Verifies extension of a trusted root and tree size using ct-merkle.
    ///
    /// # Errors
    ///
    /// - `MalformedProof`: if sizes disagree with the trusted checkpoint, the tree shrinks,
    ///   root lengths are invalid, or the proof bytes are malformed
    /// - `ConsistencyVerifError`: if the proof verification fails
    ///
    pub fn verify(
        &self,
        old_root: &[u8],
        old_tree_size: u64,
    ) -> Result<(), ct_merkle::ConsistencyVerifError> {
        // Sizes are part of the checkpoint, not assertions the prover may replace.
        if self.old_tree_size != old_tree_size
            || self.new_tree_size < old_tree_size
            || self.new_tree_size > (1_u64 << 63)
        {
            return Err(ct_merkle::ConsistencyVerifError::MalformedProof);
        }
        // Create digest from old root bytes
        let mut old_digest = Output::<Sha256>::default();
        if old_root.len() != old_digest.len() {
            return Err(ct_merkle::ConsistencyVerifError::MalformedProof);
        }
        old_digest.copy_from_slice(old_root);

        // Create old root hash with digest and size
        let old_root_ = RootHash::<Sha256>::new(old_digest, old_tree_size);

        // Create digest from new root bytes
        let mut new_digest = Output::<Sha256>::default();
        if self.new_root.len() != new_digest.len() {
            return Err(ct_merkle::ConsistencyVerifError::MalformedProof);
        }
        new_digest.copy_from_slice(&self.new_root);

        // Create new root hash with digest and size
        let new_root = RootHash::<Sha256>::new(new_digest, self.new_tree_size);

        // Create consistency proof from stored bytes
        let proof = CtConsistencyProof::<Sha256>::try_from_bytes(self.proof_bytes.clone())?;

        // Verify consistency
        new_root.verify_consistency(&old_root_, &proof)
    }
}

/// Represents a pre-computed hash value for the merkle tree.
///
/// Although the intent of this structure it to store pre-computed hashes,
/// the architecture of ct-merkle does not allow hashes to be pre-computed externally,
/// this means that even if a pre-computed hash is provided, ct-merkle will re-hash
/// it internally. This also means that it is possible to insert arbitrary binary data
/// to the tree, not just hashes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeafHash {
    /// The pre-computed SHA-256 hash of some data
    pub hash: Vec<u8>,
}

impl LeafHash {
    /// Creates a new `LeafHash` from a given hash value
    ///
    /// Note: Cannot be made `const` as `Vec<u8>` is not yet const-compatible
    #[allow(clippy::missing_const_for_fn)]
    #[must_use]
    pub fn new(hash: Vec<u8>) -> Self {
        Self { hash }
    }

    /// Returns the raw hash bytes
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.hash
    }
}

impl HashableLeaf for LeafHash {
    fn hash<H: Update>(&self, hasher: &mut H) {
        hasher.update(&self.hash);
    }
}

#[cfg(test)]
mod consistency_checkpoint_tests {
    use super::{ConsistencyProof, LeafHash, tree::CtMerkleTree};

    #[test]
    fn consistency_requires_the_trusted_size() {
        let mut tree = CtMerkleTree::new();
        tree.push(LeafHash::new(b"first".to_vec()));
        let one = tree.root();
        tree.push(LeafHash::new(b"second".to_vec()));
        let two = tree.root();
        let proof = ConsistencyProof {
            old_tree_size: 1,
            new_tree_size: 2,
            new_root: two.clone(),
            proof_bytes: tree
                .prove_consistency(&one)
                .expect("known historical root")
                .as_bytes()
                .to_vec(),
        };
        assert!(proof.verify(&one, 1).is_ok());
        assert!(proof.verify(&one, 2).is_err());
        let forged_empty = ConsistencyProof {
            old_tree_size: 0,
            new_tree_size: 3,
            new_root: one,
            proof_bytes: vec![],
        };
        assert!(forged_empty.verify(&two, 2).is_err());
        let rollback = ConsistencyProof {
            old_tree_size: 2,
            new_tree_size: 1,
            new_root: two.clone(),
            proof_bytes: vec![],
        };
        assert!(rollback.verify(&two, 2).is_err());
    }
}

#[cfg(test)]
mod inclusion_boundary_tests {
    use super::*;

    #[test]
    fn hostile_metadata_returns_errors_without_panicking() {
        for (index, tree_size) in [
            (1_u64 << 63, (1_u64 << 63) + 1),
            (0, u64::MAX),
            (u64::MAX, u64::MAX),
            (0, 0),
            (1, 1),
            ((1_u64 << 63) - 1, 1_u64 << 63),
            (0, 1_u64 << 63),
        ] {
            let proof = InclusionProof {
                index,
                tree_size,
                root: vec![0; 32],
                proof_bytes: vec![],
            };
            assert!(proof.verify(b"leaf").is_err());
        }
    }

    #[test]
    fn inclusion_requires_the_independent_checkpoint() {
        let hash = b"leaf";
        let mut tree = tree::CtMerkleTree::new();
        tree.push(LeafHash::new(hash.to_vec()));
        let proof = InclusionProof {
            index: 0,
            tree_size: 1,
            root: tree.root(),
            proof_bytes: vec![],
        };
        assert!(proof.verify_against(hash, &tree.root(), 1).is_ok());
        assert!(proof.verify_against(hash, &tree.root(), 2).is_err());
        tree.push(LeafHash::new(b"other".to_vec()));
        assert!(proof.verify_against(hash, &tree.root(), 2).is_err());
        assert!(proof.verify_against(hash, &[0; 32], 1).is_err());
    }
}
