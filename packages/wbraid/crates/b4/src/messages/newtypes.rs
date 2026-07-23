// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;

use crate::messages::artifact::Configuration;
use crate::Hasher;
use cryptography::context::Context;
use cryptography::utils::hash::Hasher as HasherTrait;
use cryptography::utils::serialization::VSerializable;
use cryptography::VSerializable as VSer;
use sha3::Digest;

pub const MAX_TRUSTEES: usize = 8;
pub const MAX_CIPHERTEXT_WIDTH: usize = 8;
pub const PROTOCOL_MANAGER_INDEX: usize = 1000;

// Hash type: using 64-byte SHA3-512 output
pub type Hash = crate::CryptographicHash;

/// Zero hash constant for comparisons
#[inline]
pub fn zero_hash() -> Hash {
    use sha3::digest::array::Array;
    Array([0u8; 64])
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
        let mut hasher = Hasher::hasher();
        hasher.update(&bytes);
        Ok(ConfigurationHash(hasher.finalize()))
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
