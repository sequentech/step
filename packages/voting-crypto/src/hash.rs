// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Standard application hashes; protocol hashes are owned by wbraid.

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256, Sha512};
use std::{io::Read, path::Path};

/// SHA-512 digest bytes used by signed application records.
pub const HASH_LENGTH_BYTES: usize = 64;
pub type Hash = [u8; HASH_LENGTH_BYTES];

/// A digest with fixed-length Borsh bytes and JSON byte-array encoding.
#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, PartialEq, Eq, std::hash::Hash)]
pub struct HashWrapper(Hash);

impl HashWrapper {
    /// Wrap an application digest.
    pub fn new(hash: Hash) -> Self {
        Self(hash)
    }
    /// Return digest bytes.
    pub fn into_inner(self) -> Hash {
        self.0
    }
    /// Return digest bytes.
    pub fn to_inner(&self) -> Hash {
        self.0
    }
}

impl Serialize for HashWrapper {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for HashWrapper {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes = <Vec<u8> as Deserialize>::deserialize(deserializer)?;
        let hash = bytes.try_into().map_err(|bytes: Vec<u8>| {
            serde::de::Error::custom(format!("expected 64 digest bytes, got {}", bytes.len()))
        })?;
        Ok(Self(hash))
    }
}

/// SHA-512 of application bytes.
pub fn hash_to_array(bytes: &[u8]) -> Result<Hash> {
    Ok(Sha512::digest(bytes).into())
}
/// FIPS 202 SHA3-512 for explicitly versioned ballot trackers.
/// Application/configuration hashes above retain their original SHA-512 identity.
pub fn hash_sha3_512(bytes: &[u8]) -> Hash {
    use sha3::Digest as _;
    sha3::Sha3_512::digest(bytes).into()
}

/// SHA-512 as a byte vector.
pub fn hash(bytes: &[u8]) -> Result<Vec<u8>> {
    Ok(Sha512::digest(bytes).to_vec())
}
/// SHA-512 in unpadded base64.
pub fn hash_b64(bytes: &[u8]) -> Result<String> {
    Ok(STANDARD_NO_PAD.encode(Sha512::digest(bytes)))
}
/// SHA-256 of application bytes.
pub fn hash_sha256(bytes: &[u8]) -> Result<Vec<u8>> {
    Ok(Sha256::digest(bytes).to_vec())
}
/// Stream a file through SHA-256 without loading it into memory.
pub fn hash_sha256_file(path: &Path) -> Result<Vec<u8>> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let size = file.read(&mut buffer)?;
        if size == 0 {
            break;
        }
        hasher.update(&buffer[..size]);
    }
    Ok(hasher.finalize().to_vec())
}
