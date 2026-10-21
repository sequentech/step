// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! How an ER file is sealed for a destination, without Java: the AES
//! password is encrypted with ECIES exactly as the ecies-tool jar does it
//! (SpongyCastle 1.58 `ECIES` cipher, `IESParameterSpec(null, null, 256, 256,
//! null, false)`), and the compressed EML is encrypted the way
//! `openssl enc -aes-256-cbc -md md5 -pass pass:<password>` does it.
//!
//! The ECIES ciphertext is `V || C || T`: `V` the ephemeral public point,
//! uncompressed; `C` the plaintext XORed with `K1`; `T` the HMAC-SHA1 of
//! `C || L2` with key `K2`, where `L2` is the length in bits of the (absent)
//! encoding vector as 8 big-endian bytes. `K2 || K1` is KDF2 with SHA-1 over
//! `V || Z`, `Z` the ECDH shared x-coordinate.

use openssl::bn::BigNumContext;
use openssl::derive::Deriver;
use openssl::ec::{EcGroupRef, EcKey, EcPoint, PointConversionForm};
use openssl::error::ErrorStack;
use openssl::hash::MessageDigest;
use openssl::memcmp;
use openssl::pkcs5::bytes_to_key;
use openssl::pkey::{HasPublic, PKey, PKeyRef, Private};
use openssl::sha::Sha1;
use openssl::sign::Signer;
use openssl::symm::{decrypt, encrypt, Cipher};
use thiserror::Error;

/// The MAC key size of the jar's `IESParameterSpec`, in bytes.
pub const ECIES_MAC_KEY_LEN: usize = 32;
/// The size of an HMAC-SHA1 tag.
pub const ECIES_MAC_LEN: usize = 20;
/// The first bytes of a file written by `openssl enc` with a salt.
pub const OPENSSL_SALTED_MAGIC: &[u8; 8] = b"Salted__";
/// The size of the salt that follows [`OPENSSL_SALTED_MAGIC`].
pub const OPENSSL_SALT_LEN: usize = 8;

const SHA1_LEN: usize = 20;
const UNCOMPRESSED_POINT_TAG: u8 = 0x04;
const COMPRESSED_POINT_TAGS: [u8; 2] = [0x02, 0x03];

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SealingError {
    #[error("the key is not an elliptic curve key")]
    NotAnEcKey,
    #[error("the encrypted key is too short")]
    TooShort,
    #[error("the encrypted key does not start with a valid curve point")]
    InvalidPoint,
    #[error("the encrypted key's MAC does not match: it was sealed for another key, or changed")]
    MacMismatch,
    #[error("the file is not a salted `openssl enc` file")]
    NotSalted,
    #[error("the file does not decrypt with the package's password")]
    WrongPassword,
    #[error("cryptographic library error: {0}")]
    Library(String),
}

impl From<ErrorStack> for SealingError {
    fn from(error: ErrorStack) -> Self {
        SealingError::Library(error.to_string())
    }
}

fn ec_key<T: HasPublic>(key: &PKeyRef<T>) -> Result<EcKey<T>, SealingError> {
    key.ec_key().map_err(|_| SealingError::NotAnEcKey)
}

fn field_len(group: &EcGroupRef) -> usize {
    (group.degree() as usize).div_ceil(8)
}

/// KDF2 (ISO 18033-2) with SHA-1 and no derivation vector.
fn kdf2_sha1(shared: &[u8], len: usize) -> Vec<u8> {
    let mut output = Vec::with_capacity(len + SHA1_LEN);
    let mut counter: u32 = 1;
    while output.len() < len {
        let mut hasher = Sha1::new();
        hasher.update(shared);
        hasher.update(&counter.to_be_bytes());
        output.extend_from_slice(&hasher.finish());
        counter = counter.wrapping_add(1);
    }
    output.truncate(len);
    output
}

fn ecdh<T: HasPublic>(own: &PKeyRef<Private>, peer: &PKeyRef<T>) -> Result<Vec<u8>, SealingError> {
    let mut deriver = Deriver::new(own)?;
    deriver.set_peer(peer)?;
    Ok(deriver.derive_to_vec()?)
}

fn mac(key: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, SealingError> {
    let key = PKey::hmac(key)?;
    let mut signer = Signer::new(MessageDigest::sha1(), &key)?;
    signer.update(ciphertext)?;
    // L2: the bit length of the encoding vector, which is absent.
    signer.update(&0u64.to_be_bytes())?;
    Ok(signer.sign_to_vec()?)
}

/// Splits `K2 || K1` out of the KDF over `V || Z`, for `len` bytes of text.
fn keys(v: &[u8], z: &[u8], len: usize) -> (Vec<u8>, Vec<u8>) {
    let shared = [v, z].concat();
    let mut k = kdf2_sha1(&shared, ECIES_MAC_KEY_LEN + len);
    let k1 = k.split_off(ECIES_MAC_KEY_LEN);
    (k1, k)
}

fn xor(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter().zip(key).map(|(byte, key)| byte ^ key).collect()
}

/// Encrypts `plaintext` for `public_key` as the jar's `encrypt` does.
pub fn ecies_encrypt<T: HasPublic>(
    public_key: &PKeyRef<T>,
    plaintext: &[u8],
) -> Result<Vec<u8>, SealingError> {
    let recipient = ec_key(public_key)?;
    let group = recipient.group();
    let ephemeral = PKey::from_ec_key(EcKey::generate(group)?)?;
    let mut context = BigNumContext::new()?;
    let v = ephemeral.ec_key()?.public_key().to_bytes(
        group,
        PointConversionForm::UNCOMPRESSED,
        &mut context,
    )?;
    let z = ecdh(&ephemeral, public_key)?;
    let (k1, k2) = keys(&v, &z, plaintext.len());
    let c = xor(plaintext, &k1);
    let t = mac(&k2, &c)?;
    Ok([v, c, t].concat())
}

/// Decrypts what the jar's `encrypt` made for `private_key`'s public key.
/// Refuses, without decrypting, a ciphertext whose MAC does not match.
pub fn ecies_decrypt(
    private_key: &PKeyRef<Private>,
    ciphertext: &[u8],
) -> Result<Vec<u8>, SealingError> {
    let own = ec_key(private_key)?;
    let group = own.group();
    let field = field_len(group);
    let point_len = match ciphertext.first() {
        Some(&UNCOMPRESSED_POINT_TAG) => 1 + 2 * field,
        Some(tag) if COMPRESSED_POINT_TAGS.contains(tag) => 1 + field,
        Some(_) => return Err(SealingError::InvalidPoint),
        None => return Err(SealingError::TooShort),
    };
    if ciphertext.len() < point_len + ECIES_MAC_LEN {
        return Err(SealingError::TooShort);
    }
    let (v, rest) = ciphertext.split_at(point_len);
    let (c, t) = rest.split_at(rest.len() - ECIES_MAC_LEN);
    let mut context = BigNumContext::new()?;
    let point =
        EcPoint::from_bytes(group, v, &mut context).map_err(|_| SealingError::InvalidPoint)?;
    let peer = EcKey::from_public_key(group, &point).map_err(|_| SealingError::InvalidPoint)?;
    peer.check_key().map_err(|_| SealingError::InvalidPoint)?;
    let peer = PKey::from_ec_key(peer)?;
    let z = ecdh(private_key, &peer)?;
    let (k1, k2) = keys(v, &z, c.len());
    let expected = mac(&k2, c)?;
    if !memcmp::eq(&expected, t) {
        return Err(SealingError::MacMismatch);
    }
    Ok(xor(c, &k1))
}

fn key_and_iv(password: &[u8], salt: &[u8]) -> Result<(Vec<u8>, Vec<u8>), SealingError> {
    let cipher = Cipher::aes_256_cbc();
    let derived = bytes_to_key(cipher, MessageDigest::md5(), password, Some(salt), 1)?;
    let iv = derived.iv.ok_or_else(|| {
        SealingError::Library("AES-256-CBC derived no initialization vector".to_string())
    })?;
    Ok((derived.key, iv))
}

/// What `openssl enc -aes-256-cbc -e -md md5 -pass pass:<password>` writes.
pub fn openssl_enc_encrypt(password: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, SealingError> {
    let mut salt = [0u8; OPENSSL_SALT_LEN];
    openssl::rand::rand_bytes(&mut salt)?;
    let (key, iv) = key_and_iv(password, &salt)?;
    let ciphertext = encrypt(Cipher::aes_256_cbc(), &key, Some(&iv), plaintext)?;
    Ok([OPENSSL_SALTED_MAGIC.as_slice(), &salt, &ciphertext].concat())
}

/// What `openssl enc -aes-256-cbc -d -md md5 -pass pass:<password>` reads.
pub fn openssl_enc_decrypt(password: &[u8], data: &[u8]) -> Result<Vec<u8>, SealingError> {
    let header_len = OPENSSL_SALTED_MAGIC.len() + OPENSSL_SALT_LEN;
    if data.len() < header_len || !data.starts_with(OPENSSL_SALTED_MAGIC) {
        return Err(SealingError::NotSalted);
    }
    let (key, iv) = key_and_iv(password, &data[OPENSSL_SALTED_MAGIC.len()..header_len])?;
    decrypt(Cipher::aes_256_cbc(), &key, Some(&iv), &data[header_len..])
        .map_err(|_| SealingError::WrongPassword)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::ec::EcGroup;
    use openssl::nid::Nid;
    use openssl::pkey::Public;

    fn p256() -> PKey<Private> {
        let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
        PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap()
    }

    fn public(key: &PKey<Private>) -> PKey<Public> {
        PKey::public_key_from_der(&key.public_key_to_der().unwrap()).unwrap()
    }

    const JAR_KEY: &[u8] =
        include_bytes!("../../../tests/fixtures/transmission/destination-1.key.pem");
    const JAR_VECTORS: &str =
        include_str!("../../../tests/fixtures/transmission/ecies-vectors.json");

    #[derive(serde::Deserialize)]
    struct Vector {
        plaintext: String,
        ciphertext: String,
    }

    fn jar_vectors() -> Vec<(String, Vec<u8>)> {
        use base64::Engine;
        let vectors: Vec<Vector> = serde_json::from_str(JAR_VECTORS).unwrap();
        vectors
            .into_iter()
            .map(|vector| {
                let ciphertext = base64::engine::general_purpose::STANDARD
                    .decode(vector.ciphertext)
                    .unwrap();
                (vector.plaintext, ciphertext)
            })
            .collect()
    }

    #[test]
    fn opens_what_the_jar_sealed() {
        let key = PKey::private_key_from_pem(JAR_KEY).unwrap();
        let vectors = jar_vectors();
        assert!(!vectors.is_empty());
        for (plaintext, ciphertext) in vectors {
            assert_eq!(
                ecies_decrypt(&key, &ciphertext).unwrap(),
                plaintext.as_bytes()
            );
        }
    }

    #[test]
    fn what_the_jar_sealed_does_not_open_with_another_key() {
        for (_, ciphertext) in jar_vectors() {
            assert_eq!(
                ecies_decrypt(&p256(), &ciphertext),
                Err(SealingError::MacMismatch)
            );
        }
    }

    #[test]
    fn kdf2_counts_from_one() {
        let mut hasher = Sha1::new();
        hasher.update(b"z");
        hasher.update(&1u32.to_be_bytes());
        let first = hasher.finish();
        let mut hasher = Sha1::new();
        hasher.update(b"z");
        hasher.update(&2u32.to_be_bytes());
        let second = hasher.finish();
        assert_eq!(kdf2_sha1(b"z", 30), [&first[..], &second[..10]].concat());
        assert!(kdf2_sha1(b"z", 0).is_empty());
    }

    #[test]
    fn a_sealed_text_opens_with_its_key_only() {
        let key = p256();
        let sealed = ecies_encrypt(&public(&key), b"secret password").unwrap();
        assert_eq!(sealed.len(), 65 + 15 + ECIES_MAC_LEN);
        assert_eq!(ecies_decrypt(&key, &sealed).unwrap(), b"secret password");
        assert_eq!(
            ecies_decrypt(&p256(), &sealed),
            Err(SealingError::MacMismatch)
        );
    }

    #[test]
    fn an_empty_text_is_sealed_too() {
        let key = p256();
        let sealed = ecies_encrypt(&public(&key), b"").unwrap();
        assert_eq!(ecies_decrypt(&key, &sealed).unwrap(), b"");
    }

    #[test]
    fn a_changed_byte_is_refused() {
        let key = p256();
        let sealed = ecies_encrypt(&public(&key), b"secret password").unwrap();
        for index in [0, 1, 64, 70, sealed.len() - 1] {
            let mut changed = sealed.clone();
            changed[index] ^= 0x01;
            assert!(ecies_decrypt(&key, &changed).is_err(), "byte {index}");
        }
    }

    #[test]
    fn short_or_malformed_ciphertexts_are_refused() {
        let key = p256();
        assert_eq!(ecies_decrypt(&key, &[]), Err(SealingError::TooShort));
        assert_eq!(
            ecies_decrypt(&key, &[UNCOMPRESSED_POINT_TAG; 70]),
            Err(SealingError::TooShort)
        );
        assert_eq!(
            ecies_decrypt(&key, &[0x07; 100]),
            Err(SealingError::InvalidPoint)
        );
        let mut off_curve = vec![UNCOMPRESSED_POINT_TAG];
        off_curve.extend([0x01; 64 + ECIES_MAC_LEN]);
        assert_eq!(
            ecies_decrypt(&key, &off_curve),
            Err(SealingError::InvalidPoint)
        );
    }

    #[test]
    fn a_compressed_ephemeral_point_is_read_but_bound_by_the_mac() {
        let key = p256();
        let sealed = ecies_encrypt(&public(&key), b"compressed").unwrap();
        let ec = key.ec_key().unwrap();
        let group = ec.group();
        let mut context = BigNumContext::new().unwrap();
        let point = EcPoint::from_bytes(group, &sealed[..65], &mut context).unwrap();
        let compressed = point
            .to_bytes(group, PointConversionForm::COMPRESSED, &mut context)
            .unwrap();
        // The MAC binds the encoding of V, so a re-encoded V is refused.
        let reencoded = [compressed.as_slice(), &sealed[65..]].concat();
        assert_eq!(
            ecies_decrypt(&key, &reencoded),
            Err(SealingError::MacMismatch)
        );
    }

    #[test]
    fn a_key_that_is_not_ec_is_refused() {
        let rsa = PKey::from_rsa(openssl::rsa::Rsa::generate(2048).unwrap()).unwrap();
        assert_eq!(
            ecies_encrypt(&rsa, b"x").unwrap_err(),
            SealingError::NotAnEcKey
        );
        assert_eq!(
            ecies_decrypt(&rsa, &[UNCOMPRESSED_POINT_TAG; 100]).unwrap_err(),
            SealingError::NotAnEcKey
        );
    }

    #[test]
    fn openssl_enc_round_trips_and_refuses_another_password() {
        let sealed = openssl_enc_encrypt(b"pass", b"compressed returns").unwrap();
        assert!(sealed.starts_with(OPENSSL_SALTED_MAGIC));
        assert_eq!(
            openssl_enc_decrypt(b"pass", &sealed).unwrap(),
            b"compressed returns"
        );
        assert!(openssl_enc_decrypt(b"other", &sealed).is_err());
        assert_eq!(
            openssl_enc_decrypt(b"pass", b"Salted_"),
            Err(SealingError::NotSalted)
        );
        assert_eq!(
            openssl_enc_decrypt(b"pass", b"NotSalted_______data"),
            Err(SealingError::NotSalted)
        );
    }

    #[test]
    fn openssl_enc_reads_what_the_openssl_command_wrote() {
        // `printf 'hello' | openssl enc -aes-256-cbc -md md5 -pass pass:pw
        // -S 0102030405060708` (OpenSSL 3 leaves the header out with -S).
        let salt = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let ciphertext = hex::decode("4536e003fedaedf3309133573bd423ce").unwrap();
        let data = [OPENSSL_SALTED_MAGIC.as_slice(), &salt, &ciphertext].concat();
        assert_eq!(openssl_enc_decrypt(b"pw", &data).unwrap(), b"hello");
    }
}
