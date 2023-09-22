use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit},
    consts::{U12, U32},
    ChaCha20Poly1305,
};
use generic_array::GenericArray;

use crate::util::StrandError;
use crate::rng::StrandRng;

pub struct EncryptionData {
    pub encrypted_bytes: Vec<u8>,
    pub nonce: GenericArray<u8, U12>,
}

fn gen_key() -> GenericArray<u8, U32> {
    let mut csprng = StrandRng;
    let key = chacha20poly1305::ChaCha20Poly1305::generate_key(&mut csprng);
    key
}
 
fn encrypt(
    key: GenericArray<u8, U32>,
    data: &[u8],
) -> Result<EncryptionData, StrandError> {
    let mut csprng = StrandRng;
    // https://docs.rs/chacha20poly1305/latest/chacha20poly1305/trait.AeadCore.html#method.generate_nonce
    // 4,294,967,296 messages with random nonces can be encrypted under a given key
    let nonce = ChaCha20Poly1305::generate_nonce(&mut csprng);
    let cipher = ChaCha20Poly1305::new(&key);
    let encrypted = cipher.encrypt(&nonce, data)
        .map_err(|e| StrandError::Chacha20Error(e))?;

    Ok(        
        EncryptionData {
            encrypted_bytes: encrypted,
            nonce: nonce,
        }
    )
}

fn decrypt(
    key: GenericArray<u8, U32>,
    ed: &EncryptionData,
) -> Result<Vec<u8>, StrandError> {

    let cipher = ChaCha20Poly1305::new(&key);
    let bytes: &[u8] = &ed.encrypted_bytes;
    let decrypted = cipher.decrypt(&ed.nonce, bytes)
        .map_err(|e| StrandError::Chacha20Error(e))?;

    Ok(decrypted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;

    #[test]
    fn test_aes() {
        let mut csprng = StrandRng;

        let key = gen_key();
        let mut data = [0u8; 256];
        csprng.fill_bytes(&mut data);

        let encrypted = encrypt(key, &data).unwrap();

        let decrypted = decrypt(key, &encrypted).unwrap();

        assert_eq!(data.to_vec(), decrypted);
    }
}