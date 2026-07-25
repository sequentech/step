// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;

use super::artifact::Configuration;
use cryptography::context::Context;
use cryptography::utils::serialization::VSerializable;
use cryptography::VSerializable as VSer;

pub const MAX_TRUSTEES: usize = 8;
pub const MAX_CIPHERTEXT_WIDTH: usize = 8;
pub const PROTOCOL_MANAGER_INDEX: usize = 1000;

use cryptography::utils::hash::Hasher as HasherTrait;

/// The hasher instance as defined by the cryptography library (SHA3-512).
pub type Hasher = cryptography::context::CryptographicHasher;

/// The 64-byte hash output type (SHA3-512).
pub type CryptographicHash = sha3::digest::Output<Hasher>;

/// Shorthand used throughout the protocol types.
pub type Hash = CryptographicHash;

/// Hash bytes to produce a [`CryptographicHash`] (§3.4).
///
/// Uses the library's global default hasher rather than threading through a
/// `Context`, because the output type is a fixed protocol format that must not
/// vary per context instantiation.
pub fn hash_bytes(bytes: &[u8]) -> CryptographicHash {
    use sha3::Digest;
    let mut hasher = Hasher::hasher();
    hasher.update(bytes);
    hasher.finalize()
}

/// Zero hash constant for comparisons and tests.
#[inline]
pub fn zero_hash() -> Hash {
    // A 64-byte all-zero GenericArray, without importing sha3 directly.
    Default::default()
}

///////////////////////////////////////////////////////////////////////////
// Newtypes
///////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, PartialEq, Eq, VSer)]
pub struct ConfigurationHash(pub Hash);

impl ConfigurationHash {
    pub fn from_configuration<C: Context>(
        configuration: &Configuration<C>,
    ) -> Result<ConfigurationHash> {
        let bytes = configuration.ser();
        Ok(ConfigurationHash(hash_bytes(&bytes)))
    }
}
impl std::fmt::Debug for ConfigurationHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ConfigurationHash({})", dbg_hash(&self.0))
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, VSer)]
pub struct SharesHash(pub Hash);
impl std::fmt::Debug for SharesHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SharesHash({})", dbg_hash(&self.0))
    }
}

#[derive(Copy, Clone, PartialEq, Eq, VSer)]
pub struct PublicKeyHash(pub Hash);
impl std::fmt::Debug for PublicKeyHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PublicKeyHash({})", dbg_hash(&self.0))
    }
}

// The ciphertexts hash is used to refer to ballots and mix artifacts.
// This allows accessing either one when pointing to a source of
// ciphertexts (ballots or mix). The same typed hash is propagated
// all the way from Ballots to DecryptionFactors predicates.
#[derive(Copy, Clone, PartialEq, Eq, VSer)]
pub struct CiphertextsHash(pub Hash);
impl std::fmt::Debug for CiphertextsHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CiphertextsHash({})", dbg_hash(&self.0))
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, VSer)]
pub struct DecryptionFactorsHash(pub Hash);
impl std::fmt::Debug for DecryptionFactorsHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DecryptionFactorsHash({})", dbg_hash(&self.0))
    }
}

#[derive(Copy, Clone, PartialEq, Eq, VSer)]
pub struct PlaintextsHash(pub Hash);
impl std::fmt::Debug for PlaintextsHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PlaintextsHash({})", dbg_hash(&self.0))
    }
}

///////////////////////////////////////////////////////////////////////////
// Type aliases
///////////////////////////////////////////////////////////////////////////

// 0-based
pub type TrusteePosition = usize;
// 1-based
pub type Threshold = usize;
pub type TrusteeCount = usize;

// 1-based trustee index of a message sender (see crates/braid/v0.6_spec.md §4.3)
pub type TrusteeIndex = usize;

// Seconds elapsed since the std::time::UNIX_EPOCH
pub type Timestamp = u64;

///////////////////////////////////////////////////////////////////////////
// Debug
///////////////////////////////////////////////////////////////////////////

fn dbg_hash(h: &Hash) -> String {
    hex::encode(h)[0..10].to_string()
}
