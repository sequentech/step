// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit},
    consts::{U12, U32},
    ChaCha20Poly1305,
};
use generic_array::GenericArray;

use crate::rng::StrandRng;
use crate::util::StrandError;

pub type SymmetricKey = GenericArray<u8, U32>;

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct EncryptionData {
    pub encrypted_bytes: Vec<u8>,
    pub nonce: [u8; 12],
}
impl EncryptionData {
    pub fn new(
        encrypted_bytes: Vec<u8>,
        nonce: GenericArray<u8, U12>,
    ) -> EncryptionData {
        EncryptionData {
            encrypted_bytes,
            nonce: nonce.into(),
        }
    }
}

pub fn gen_key() -> GenericArray<u8, U32> {
    let mut csprng = StrandRng;
    let key = chacha20poly1305::ChaCha20Poly1305::generate_key(&mut csprng);
    key
}
pub fn encrypt(
    key: GenericArray<u8, U32>,
    data: &[u8],
) -> Result<EncryptionData, StrandError> {
    let mut csprng = StrandRng;
    // https://docs.rs/chacha20poly1305/latest/chacha20poly1305/trait.AeadCore.html#method.generate_nonce
    // 4,294,967,296 messages with random nonces can be encrypted under a given
    // key
    let nonce = ChaCha20Poly1305::generate_nonce(&mut csprng);
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
    key: &GenericArray<u8, U32>,
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
    let key = GenericArray::<u8, U32>::from_slice(&bytes).to_owned();

    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;

    #[test]
    fn test_chacha_poly() {
        let mut csprng = StrandRng;

        let key = gen_key();
        let mut data = [0u8; 256];
        csprng.fill_bytes(&mut data);

        let encrypted = encrypt(key, &data).unwrap();

        let decrypted = decrypt((&key).into(), &encrypted).unwrap();

        assert_eq!(data.to_vec(), decrypted);
    }
}

pub fn info() -> String {
    format!("{}, FIPS_ENABLED: FALSE", module_path!())
}
