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
    /// Verifies this proof against the given leaf hash using ct-merkle's proof verification
    ///
    /// # Errors
    ///
    /// Returns an error if the root hash length is invalid.
    pub fn verify(&self, hash: &[u8]) -> Result<(), ct_merkle::InclusionVerifError> {
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
        /*match root_hash.verify_inclusion(&leaf_hash, self.index, &proof) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }*/
        root_hash.verify_inclusion(&leaf_hash, self.index, &proof)
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
    /// Verifies this proof using ct-merkle's consistency proof verification
    ///
    /// # Errors
    ///
    /// - `MalformedProof`: if the old or new root hash lengths are invalid, or if the proof bytes are malformed
    /// - `ConsistencyVerifError`: if the proof verification fails
    ///
    pub fn verify(&self, old_root: &[u8]) -> Result<(), ct_merkle::ConsistencyVerifError> {
        // Create digest from old root bytes
        let mut old_digest = Output::<Sha256>::default();
        if old_root.len() != old_digest.len() {
            return Err(ct_merkle::ConsistencyVerifError::MalformedProof);
        }
        old_digest.copy_from_slice(old_root);

        // Create old root hash with digest and size
        let old_root_ = RootHash::<Sha256>::new(old_digest, self.old_tree_size);

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
