// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use base64::{engine::general_purpose, Engine as _};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{
    de, de::SeqAccess, de::Visitor, Deserialize, Deserializer, Serialize,
    Serializer,
};
use sha2::Sha256;
use sha2::Sha512;
use sha3::Shake256;
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;

use crate::util::StrandError;

/// Sha-512 hashes are 64 bytes.
pub const STRAND_HASH_LENGTH_BYTES: usize = 64;
/// Sha-512 hashes are 64 byte arrays: [u8; 64].
pub type Hash = [u8; STRAND_HASH_LENGTH_BYTES];

// Create a new struct that wraps the Hash type
#[derive(
    BorshSerialize, BorshDeserialize, Clone, PartialEq, Hash, Eq, Debug,
)]
pub struct HashWrapper {
    inner: Hash,
}

impl HashWrapper {
    // Provide methods to work with HashWrapper as needed
    pub fn new(hash: Hash) -> Self {
        HashWrapper { inner: hash }
    }

    pub fn into_inner(self) -> Hash {
        self.inner
    }

    pub fn to_inner(self) -> Hash {
        self.inner.clone()
    }
}

impl Serialize for HashWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.inner)
    }
}

impl<'de> Deserialize<'de> for HashWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct HashVisitor;

        impl<'de> Visitor<'de> for HashVisitor {
            type Value = HashWrapper;

            fn expecting(
                &self,
                formatter: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                formatter.write_str("a byte array")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<HashWrapper, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let mut inner = [0; STRAND_HASH_LENGTH_BYTES];
                for (i, byte) in inner.iter_mut().enumerate() {
                    *byte = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(i, &self))?;
                }
                Ok(HashWrapper { inner })
            }
        }

        deserializer.deserialize_byte_buf(HashVisitor)
    }
}

pub(crate) type Hasher = Sha512;
pub(crate) use sha2::Digest;

/// Single entry point for all hashing, returns a vector.
pub fn hash(bytes: &[u8]) -> Result<Vec<u8>, StrandError> {
    let mut hasher = hasher();
    curve25519_dalek::digest::Update::update(&mut hasher, bytes);
    Ok(hasher.finalize().to_vec())
}

pub fn hash_sha256(bytes: &[u8]) -> Result<Vec<u8>, StrandError> {
    let mut hasher = Sha256::new();
    curve25519_dalek::digest::Update::update(&mut hasher, bytes);
    Ok(hasher.finalize().to_vec())
}

pub fn hash_sha256_file(path: &PathBuf) -> Result<Vec<u8>, StrandError> {
    let mut file =
        File::open(path).map_err(|e| StrandError::SerializationError(e))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|e| StrandError::SerializationError(e))?;
        if bytes_read == 0 {
            break;
        }
        curve25519_dalek::digest::Update::update(
            &mut hasher,
            &buffer[..bytes_read],
        );
    }

    Ok(hasher.finalize().to_vec())
}

/// Single entry point for all hashing, returns an array.
pub fn hash_to_array(bytes: &[u8]) -> Result<Hash, StrandError> {
    let mut hasher = hasher();
    curve25519_dalek::digest::Update::update(&mut hasher, bytes);
    let ret: Hash = hasher.finalize().into();
    Ok(ret)
}
/// Single access point for all hashing.
pub(crate) fn hasher() -> Hasher {
    Sha512::new()
}
/// Hash and base 64 encode resulting bytes.
pub fn hash_b64(bytes: &[u8]) -> Result<String, StrandError> {
    let bytes = hash(bytes)?;
    let ret = general_purpose::STANDARD_NO_PAD.encode(&bytes);
    Ok(ret)
}

pub(crate) use sha3::digest::{ExtendableOutput, Update, XofReader};
pub(crate) fn hasher_xof() -> Shake256 {
    Shake256::default()
}

// Rustcrypto ecdsa signatures are only used on the wasm target
cfg_if::cfg_if! {
    if #[cfg(feature = "wasm")] {
        // The rustcrypto signature backend requires 384 bit hashes. Calling
        // verify_digest on a RustCrypto ecdsa VerifyingKey<P384> fails to compile,
        // unless the digest passed is Sha384:
        /*
            the trait `DigestVerifier<CoreWrapper<CtVariableCoreWrapper<Sha256VarCore, UInt<UInt<UInt<UInt<UInt<UInt<UTerm, B1>, B0>, B0>, B0>, B0>, B0>, OidSha256>>, _>` is not implemented for `ecdsa::VerifyingKey<NistP384>`
        */
        use sha2::Sha384;
        pub(crate) fn rust_crypto_ecdsa_hasher() -> RustCryptoHasher {
            Sha384::new()
        }
        pub(crate) type RustCryptoHasher = Sha384;
    }
}

pub fn info() -> String {
    format!("{}, FIPS_ENABLED: FALSE", module_path!())
}
