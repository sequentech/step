// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use borsh::{BorshDeserialize, BorshSerialize};
use openssl::symm::decrypt_aead;
use openssl::symm::encrypt_aead;
use openssl::symm::Cipher;
use rand::RngCore;

use crate::rng::StrandRng;
use crate::util::StrandError;

pub type SymmetricKey = [u8; 32];

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct EncryptionData {
    pub encrypted_bytes: Vec<u8>,
    pub iv: [u8; 12],
    pub tag: [u8; 16],
}

pub fn gen_key() -> [u8; 32] {
    let mut csprng = StrandRng;

    let mut key = [0u8; 32];
    csprng.fill_bytes(&mut key);

    key
}

pub fn encrypt(
    key: [u8; 32],
    data: &[u8],
    aad: &[u8],
) -> Result<EncryptionData, StrandError> {
    let mut csprng = StrandRng;

    let cipher = Cipher::aes_256_gcm();
    let mut tag = [0u8; 16];
    // https://docs.rs/chacha20poly1305/latest/chacha20poly1305/trait.AeadCore.html#method.generate_nonce
    // 4,294,967,296 messages with random nonces can be encrypted under a given
    // key (We refer to chacha20 documentation above as the iv and nonce
    // sizes for each are identical)
    let mut iv = [0u8; 12];
    csprng.fill_bytes(&mut iv);

    let encrypted =
        encrypt_aead(cipher, &key, Some(&iv), &aad, &data, &mut tag);

    Ok(EncryptionData {
        encrypted_bytes: encrypted?,
        iv: iv,
        tag: tag,
    })
}

pub fn decrypt(
    key: &[u8; 32],
    ed: &EncryptionData,
    aad: &[u8],
) -> Result<Vec<u8>, StrandError> {
    let cipher = Cipher::aes_256_gcm();

    let ret = decrypt_aead(
        cipher,
        key,
        Some(&ed.iv),
        &aad,
        &ed.encrypted_bytes,
        &ed.tag,
    );

    Ok(ret?)
}

pub fn sk_from_bytes(bytes: &[u8]) -> Result<SymmetricKey, StrandError> {
    crate::util::to_u8_array(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_aes() {
        let mut csprng = StrandRng;

        let key = gen_key();
        let mut data = [0u8; 256];
        let mut aad = [0u8; 256];
        csprng.fill_bytes(&mut data);
        csprng.fill_bytes(&mut aad);

        let encryption_data = encrypt(key, &data, &aad).unwrap();

        let decrypted = decrypt(&key, &encryption_data, &aad).unwrap();

        assert_eq!(data.to_vec(), decrypted);
    }
}

pub fn info() -> String {
    format!("{}, FIPS_ENABLED: TRUE", module_path!())
}
