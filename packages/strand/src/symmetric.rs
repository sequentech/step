use openssl::symm::encrypt_aead;
use openssl::symm::decrypt_aead;
use openssl::symm::Cipher;

use crate::util::StrandError;
use crate::rng::StrandRng;
use rand::RngCore;

fn gen_key() -> [u8; 32] {
    let mut csprng = StrandRng;

    let mut key = [0u8; 32];
    csprng.fill_bytes(&mut key);

    key
}

fn encrypt(key: &[u8; 32], data: &[u8], aad: &[u8]) -> Result<(Vec<u8>, [u8; 16], [u8; 12]), StrandError> {
    let mut csprng = StrandRng;

    let cipher = Cipher::aes_256_gcm();
    let mut tag = [0u8; 16];
    let mut iv = [0u8; 12];
    csprng.fill_bytes(&mut iv);

    let encrypted = encrypt_aead(
        cipher,
        key,
        Some(&iv),
        &aad,
        &data,
        &mut tag
    );

    Ok((encrypted?, tag, iv))
}

fn decrypt(key: &[u8; 32], encrypted: &[u8], aad: &[u8], tag: &[u8; 16], iv: &[u8; 12]) -> Result<Vec<u8>, StrandError> {
    let cipher = Cipher::aes_256_gcm();

    let ret = decrypt_aead(
        cipher,
        key,
        Some(iv),
        &aad,
        &encrypted,
        tag
    );

    Ok(ret?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_aes() {
        let key = gen_key();
        let data = [0u8; 256];
        let aad = [0u8; 256];
        let (encrypted, tag, iv) = encrypt(&key, &data, &aad).unwrap();
        
        let decrypted = decrypt(&key, &encrypted, &aad, &tag, &iv).unwrap();

        assert_eq!(data.to_vec(), decrypted);
    }
}