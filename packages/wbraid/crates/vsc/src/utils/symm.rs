// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// Adapted from Sequent Tech's strand library
// See LICENSE.md for details

//! Symmetric encryption utilities using ChaCha20-Poly1305
//!
//! # Examples
//!
//! ```
//! use cryptography::utils::symm::{gen_key, encrypt, decrypt};
//!
//! // generate random key
//! let key = gen_key().unwrap();
//! // some data to encrypt
//! let data = b"Hello, world!";
//! // encrypt
//! let encrypted = encrypt(key, data).unwrap();
//! // decrypt
//! let decrypted = decrypt(&key, &encrypted).unwrap();
//!
//! assert_eq!(data.to_vec(), decrypted);
//! ```

use canonical_derive::Canonical;
use chacha20poly1305::{aead::Aead, aead::Generate, aead::KeyInit, ChaCha20Poly1305, Nonce};
use chacha20poly1305::aead::Key;

use crate::utils::error::Error;

/// Symmetric encryption key for ChaCha20-Poly1305
/// 
/// Re-export the Array type from chacha20poly1305's dependency
pub type SymmetricKey = Key<ChaCha20Poly1305>;

/// Encrypted data with associated nonce for ChaCha20-Poly1305 AEAD
#[derive(Canonical, Clone)]
pub struct EncryptionData {
    /// The encrypted ciphertext
    pub encrypted_bytes: Vec<u8>,
    /// The nonce used for encryption (96 bits)
    pub nonce: [u8; 12],
}

impl EncryptionData {
    /// Create a new `EncryptionData` from encrypted bytes and nonce
    #[must_use]
    pub fn new(encrypted_bytes: Vec<u8>, nonce: Nonce) -> EncryptionData {
        EncryptionData {
            encrypted_bytes,
            nonce: nonce.into(),
        }
    }
}

/// Generate a random symmetric encryption key
/// 
/// From crate doc:"Generate random key using the operating system’s secure RNG."
/// 
/// # Errors
///     
/// Returns `Error::EncryptionError` if key generation fails
pub fn gen_key() -> Result<SymmetricKey, Error> {
    #[cfg_attr(
        feature = "custom-warnings",
        crate::warning(
            "We should pass in our single rng entry point, instead of delegating to Key internal generator"
        )
    )]
    Ok(Key::<ChaCha20Poly1305>::generate())
}

/// Encrypt data using ChaCha20-Poly1305
///
/// # Errors
///
/// Returns `Error::EncryptionError` if encryption fails
pub fn encrypt(key: SymmetricKey, data: &[u8]) -> Result<EncryptionData, Error> {
    // https://docs.rs/chacha20poly1305/latest/chacha20poly1305/trait.AeadCore.html#method.generate_nonce
    // 4,294,967,296 messages with random nonces can be encrypted under a given key
    #[cfg_attr(
        feature = "custom-warnings",
        crate::warning(
            "We should pass in our single rng entry point, instead of delegating to Nonce internal generator"
        )
    )]
    let nonce = Nonce::generate();
    let cipher = ChaCha20Poly1305::new(&key);
    let encrypted = cipher
        .encrypt(&nonce, data)
        .map_err(|e| Error::EncryptionError(e.to_string()))?;

    Ok(EncryptionData::new(encrypted, nonce))
}

/// Decrypt data using ChaCha20-Poly1305
///
/// # Errors
///
/// Returns `Error::DecryptionError` if decryption fails
pub fn decrypt(key: &SymmetricKey, ed: &EncryptionData) -> Result<Vec<u8>, Error> {
    let cipher = ChaCha20Poly1305::new(key);
    let nonce = Nonce::from(ed.nonce);
    let decrypted = cipher
        .decrypt(&nonce, ed.encrypted_bytes.as_ref())
        .map_err(|e| Error::DecryptionError(e.to_string()))?;

    Ok(decrypted)
}

/// Create a symmetric key from raw bytes
///
/// # Errors
///
/// Returns `Error::DeserializationError` if the byte slice is not exactly 32 bytes
pub fn sk_from_bytes(bytes: &[u8]) -> Result<SymmetricKey, Error> {
    let array: [u8; 32] = bytes.try_into()
        .map_err(|_| Error::DeserializationError("Invalid symmetric key length: expected 32 bytes".to_string()))?;
    Ok(array.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn test_chacha_poly() {
        let key = gen_key().unwrap();
        let mut data = [0u8; 256];
        rand::rng().fill_bytes(&mut data);

        let encrypted = encrypt(key, &data).unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();

        assert_eq!(data.to_vec(), decrypted);
    }
}
