// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;

use crate::Hasher;
use cryptography::utils::hash::Hasher as HasherTrait;
use crate::messages::artifact::Configuration;
use cryptography::context::Context;
use cryptography::utils::serialization::VSerializable;
use cryptography::VSerializable as VSer;
use sha3::Digest;

pub const MAX_TRUSTEES: usize = 12;
pub const MAX_CIPHERTEXT_WIDTH: usize = 10; 
pub const PROTOCOL_MANAGER_INDEX: usize = 1000;
pub const VERIFIER_INDEX: usize = 2000;
pub const NULL_TRUSTEE: usize = 1001;

// Hash type: using 64-byte SHA3-512 output
pub type Hash = crate::CryptographicHash;

/// Zero hash constant for comparisons
#[inline]
pub fn zero_hash() -> Hash {
    use sha3::digest::generic_array::GenericArray;
    GenericArray::from_slice(&[0u8; 64]).clone()
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

#[derive(Copy, Clone, PartialEq, Eq, VSer)]
pub struct ChannelHash(pub Hash);
impl std::fmt::Debug for ChannelHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChannelHash({})", dbg_hash(&self.0))
    }
}

#[derive(Copy, Clone, PartialEq, Eq, VSer)]
pub struct ChannelsHashes(pub THashes);
impl std::fmt::Debug for ChannelsHashes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChannelsHashes({})", dbg_hashes(&self.0))
    }
}

#[derive(Copy, Clone, PartialEq, Eq, VSer)]
pub struct SharesHash(pub Hash);
impl std::fmt::Debug for SharesHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SharesHash({})", dbg_hash(&self.0))
    }
}

#[derive(Copy, Clone, PartialEq, Eq, VSer)]
pub struct SharesHashes(pub THashes);
impl std::fmt::Debug for SharesHashes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SharesHashes({})", dbg_hashes(&self.0))
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

#[derive(Copy, Clone, PartialEq, Eq, VSer)]
pub struct DecryptionFactorsHash(pub Hash);
impl std::fmt::Debug for DecryptionFactorsHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DecryptionFactorsHash({})", dbg_hash(&self.0))
    }
}

#[derive(Copy, Clone, PartialEq, Eq, VSer)]
pub struct DecryptionFactorsHashes(pub THashes);
impl std::fmt::Debug for DecryptionFactorsHashes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DecryptionFactorsHashes({})", dbg_hashes(&self.0))
    }
}

#[derive(Copy, Clone, PartialEq, Eq, VSer)]
pub struct MixingHashes(pub THashes);
impl std::fmt::Debug for MixingHashes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MixingHashes({})", dbg_hashes(&self.0))
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
// 1-based: the elements of the array are 1-based trustee positions
pub type TrusteeSet = [usize; MAX_TRUSTEES];

// 1-based, the position in the mixing chain (note this is not the same as the
// position of the mixing trustee, since active trustees are set by the ballots artifact)
pub type MixNumber = usize;

pub type BatchNumber = u64;
// Seconds elapsed since the std::time::UNIX_EPOCH
pub type Timestamp = u64;

pub type THashes = [Hash; MAX_TRUSTEES];

///////////////////////////////////////////////////////////////////////////
// Debug
///////////////////////////////////////////////////////////////////////////

fn dbg_hash(h: &Hash) -> String {
    hex::encode(h)[0..10].to_string()
}
fn dbg_hashes<const N: usize>(hs: &[Hash; N]) -> String {
    let zero = zero_hash();
    hs.map(|h| {
        if h == zero {
            "-".to_string()
        } else {
            hex::encode(h)[0..10].to_string()
        }
    })
    .join(" ")
}
