// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! # Examples
//!
//! ```
//! // This example shows how to symmetrically encrypt bytes.
//! use strand::rng::StrandRng;
//! use rand::RngCore;
//! use strand::symm::encrypt;
//! use strand::symm::decrypt;
//! use strand::symm::gen_key;
//!
//! // generate random key
//! let key = gen_key();
//! // generate random data
//! let mut csprng = StrandRng;
//! let mut data = [0u8; 256];
//! csprng.fill_bytes(&mut data);
//! // encrypt
//! let encrypted = encrypt(key, &data).unwrap();
//! // decrypt
//! let decrypted = decrypt((&key).into(), &encrypted).unwrap();
//!
//! assert_eq!(data.to_vec(), decrypted);
//! ```

use borsh::{BorshDeserialize, BorshSerialize};
use chacha20poly1305::{aead::Aead, aead::Generate, aead::KeyInit, ChaCha20Poly1305, Nonce};
use chacha20poly1305::aead::Key;


use crate::util::StrandError;
use crate::rng::StrandRng;

pub type SymmetricKey = Key<ChaCha20Poly1305>;

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct EncryptionData {
    pub encrypted_bytes: Vec<u8>,
    pub nonce: [u8; 12],
}
impl EncryptionData {
    pub fn new(
        encrypted_bytes: Vec<u8>,
        nonce: Nonce,
    ) -> EncryptionData {
        EncryptionData {
            encrypted_bytes,
            nonce: nonce.into(),
        }
    }
}

pub fn gen_key() -> SymmetricKey {
    let mut csprng = StrandRng;
    let key = Key::<ChaCha20Poly1305>::generate_from_rng(&mut csprng);
    key
}
pub fn encrypt(
    key: SymmetricKey,
    data: &[u8],
) -> Result<EncryptionData, StrandError> {
    // https://docs.rs/chacha20poly1305/latest/chacha20poly1305/trait.AeadCore.html#method.generate_nonce
    // 4,294,967,296 messages with random nonces can be encrypted under a given
    // key
    let mut csprng = StrandRng;
    let nonce = Nonce::generate_from_rng(&mut csprng);
    let cipher = ChaCha20Poly1305::new(&key);
    let encrypted = cipher
        .encrypt(&nonce, data)
        .map_err(|e| StrandError::Chacha20Error(e))?;

    Ok(EncryptionData {
        encrypted_bytes: encrypted,
        nonce: nonce.into(),
    })
}

pub fn decrypt(
    key: &SymmetricKey,
    ed: &EncryptionData,
) -> Result<Vec<u8>, StrandError> {
    let cipher = ChaCha20Poly1305::new(&key);
    let bytes: &[u8] = &ed.encrypted_bytes;
    let decrypted = cipher
        .decrypt(&ed.nonce.into(), bytes)
        .map_err(|e| StrandError::Chacha20Error(e))?;

    Ok(decrypted)
}

pub fn sk_from_bytes(bytes: &[u8]) -> Result<SymmetricKey, StrandError> {
    let array: [u8; 32] = bytes.try_into()
        .map_err(|_| StrandError::Generic("Invalid symmetric key length: expected 32 bytes".to_string()))?;
    
    Ok(array.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::StrandRng;
    use rand::TryRng;

    #[test]
    fn test_chacha_poly() {
        let mut csprng = StrandRng;

        let key = gen_key();
        let mut data = [0u8; 256];
        csprng.try_fill_bytes(&mut data);

        let encrypted = encrypt(key, &data).unwrap();

        let decrypted = decrypt((&key).into(), &encrypted).unwrap();

        assert_eq!(data.to_vec(), decrypted);
    }
}

pub fn info() -> String {
    format!("{}, FIPS_ENABLED: FALSE", module_path!())
}
