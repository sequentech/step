// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Password transport for an already signed configuration delivery. This
//! envelope authenticates the ciphertext, not the delivery's authority:
//! the unwrapped ZIP must still pass normal package verification and approval.
//!
//! V1, in byte order: 12 bytes magic, one version byte, 16-byte random salt,
//! 12-byte random nonce, AES-256-GCM ciphertext and a 16-byte tag. The full
//! header is associated data. Derive the key from the exact UTF-8 password
//! with PBKDF2-HMAC-SHA256 (600,000 rounds) and the header's salt. No variable
//! work factors, compression, public metadata or legacy OpenSSL fallback.

use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::pbkdf2;
use ring::rand::{SecureRandom, SystemRandom};
use std::fmt;
use std::num::NonZeroU32;

const MAGIC: &[u8; 12] = b"SEQUENT-EZIP";
const VERSION: u8 = 1;
const SALT_START: usize = MAGIC.len() + 1;
const NONCE_START: usize = SALT_START + 16;
const HEADER_LEN: usize = NONCE_START + 12;
const TAG_LEN: usize = 16;
const ROUNDS: NonZeroU32 = NonZeroU32::new(600_000).unwrap();
const MAX_PASSWORD_BYTES: usize = 1024;

/// Lower than the signed archive's separate expansion limit: the browser
/// materializes the encrypted package and its plaintext in memory.
pub const MAX_SIGNED_DELIVERY_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_ENCRYPTED_SIGNED_DELIVERY_BYTES: usize =
    MAX_SIGNED_DELIVERY_BYTES + HEADER_LEN + TAG_LEN;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptedDeliveryError {
    InvalidPassword,
    InvalidEnvelope,
    TooLarge,
    RandomUnavailable,
    SealFailed,
    AuthenticationFailed,
}

impl fmt::Display for EncryptedDeliveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidPassword => "Enter a non-blank package password of at most 1024 UTF-8 bytes.",
            Self::InvalidEnvelope => "This is not a supported encrypted signed configuration package.",
            Self::TooLarge => "The encrypted configuration package exceeds the 256 MiB payload limit.",
            Self::RandomUnavailable => "Secure randomness for package encryption is unavailable.",
            Self::SealFailed => "The signed package could not be encrypted.",
            Self::AuthenticationFailed => "The package password is incorrect or the encrypted package has been modified.",
        })
    }
}

impl std::error::Error for EncryptedDeliveryError {}

/// Classify by magic alone even when the version or payload is invalid. This
/// lets receivers refuse unsupported/truncated envelopes rather than falling
/// back to the normal import path. Callers need only read the first 12 bytes.
pub fn is_encrypted_signed_delivery(bytes: &[u8]) -> bool {
    bytes.starts_with(MAGIC)
}

fn check_password(password: &str) -> Result<(), EncryptedDeliveryError> {
    if password.len() > MAX_PASSWORD_BYTES || password.trim().is_empty() {
        return Err(EncryptedDeliveryError::InvalidPassword);
    }
    Ok(())
}

fn check_payload_len(len: usize) -> Result<(), EncryptedDeliveryError> {
    if len > MAX_SIGNED_DELIVERY_BYTES {
        return Err(EncryptedDeliveryError::TooLarge);
    }
    if len == 0 {
        return Err(EncryptedDeliveryError::InvalidEnvelope);
    }
    Ok(())
}

fn nonce(header: &[u8]) -> Nonce {
    // This slice is always exactly 12 bytes after preflight.
    Nonce::assume_unique_for_key(
        header[NONCE_START..HEADER_LEN].try_into().unwrap(),
    )
}

fn key(
    password: &str,
    header: &[u8],
) -> Result<LessSafeKey, ring::error::Unspecified> {
    let mut key_bytes = [0; 32];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        ROUNDS,
        &header[SALT_START..NONCE_START],
        password.as_bytes(),
        &mut key_bytes,
    );
    let key = UnboundKey::new(&AES_256_GCM, &key_bytes);
    key_bytes.fill(0);
    key.map(LessSafeKey::new)
}

pub fn encrypt_signed_delivery(
    signed_zip: &[u8],
    password: &str,
) -> Result<Vec<u8>, EncryptedDeliveryError> {
    check_payload_len(signed_zip.len())?;
    check_password(password)?;
    let mut header = [0; HEADER_LEN];
    header[..MAGIC.len()].copy_from_slice(MAGIC);
    header[MAGIC.len()] = VERSION;
    SystemRandom::new()
        .fill(&mut header[SALT_START..])
        .map_err(|_| EncryptedDeliveryError::RandomUnavailable)?;

    let mut wire = Vec::with_capacity(HEADER_LEN + signed_zip.len() + TAG_LEN);
    wire.extend_from_slice(&header);
    wire.extend_from_slice(signed_zip);
    let tag = key(password, &header)
        .and_then(|cipher| {
            cipher.seal_in_place_separate_tag(
                nonce(&header),
                Aad::from(&header),
                &mut wire[HEADER_LEN..],
            )
        })
        .map_err(|_| EncryptedDeliveryError::SealFailed)?;
    wire.extend_from_slice(tag.as_ref());
    Ok(wire)
}

fn preflight(bytes: &[u8]) -> Result<(&[u8], &[u8]), EncryptedDeliveryError> {
    if bytes.len() > MAX_ENCRYPTED_SIGNED_DELIVERY_BYTES {
        return Err(EncryptedDeliveryError::TooLarge);
    }
    if !is_encrypted_signed_delivery(bytes)
        || bytes.get(MAGIC.len()) != Some(&VERSION)
        || bytes.len() <= HEADER_LEN + TAG_LEN
    {
        return Err(EncryptedDeliveryError::InvalidEnvelope);
    }
    Ok(bytes.split_at(HEADER_LEN))
}

pub fn decrypt_signed_delivery(
    bytes: &[u8],
    password: &str,
) -> Result<Vec<u8>, EncryptedDeliveryError> {
    let (header, ciphertext) = preflight(bytes)?;
    check_password(password)?;
    let mut body = ciphertext.to_vec();
    let plaintext_len = key(password, header)
        .and_then(|cipher| {
            cipher.open_in_place(nonce(header), Aad::from(header), &mut body)
        })
        .map_err(|_| EncryptedDeliveryError::AuthenticationFailed)?
        .len();
    body.truncate(plaintext_len);
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSWORD: &str = "correct horse ⚿";
    const PAYLOAD: [u8; 6] = [0, 1, 2, 255, 80, 75];

    #[test]
    fn independent_v1_aes256_gcm_vector() {
        // Independently computed using Node's PBKDF2-SHA256 and AES-256-GCM:
        // salt 16 x 0x73, nonce 12 x 0x6e, header used as AAD.
        let hex = concat!(
            "53455155454e542d455a49500173737373737373737373737373737373",
            "6e6e6e6e6e6e6e6e6e6e6e6ea18d721459def35bde55338c720460fcdf6394d28686"
        );
        let bytes = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(decrypt_signed_delivery(&bytes, PASSWORD).unwrap(), PAYLOAD);
    }

    #[test]
    fn binary_roundtrip_randomized_and_password_not_trimmed() {
        let password = " test password ";
        let wire = encrypt_signed_delivery(&PAYLOAD, password).unwrap();
        assert!(is_encrypted_signed_delivery(&wire[..MAGIC.len()]));
        assert_eq!(decrypt_signed_delivery(&wire, password).unwrap(), PAYLOAD);
        assert_ne!(encrypt_signed_delivery(&PAYLOAD, password).unwrap(), wire);
        assert_eq!(
            decrypt_signed_delivery(&wire, password.trim()).unwrap_err(),
            EncryptedDeliveryError::AuthenticationFailed
        );
    }

    #[test]
    fn tamper_and_truncation_never_return_plaintext() {
        let wire = encrypt_signed_delivery(&PAYLOAD, PASSWORD).unwrap();
        for index in [SALT_START, NONCE_START, HEADER_LEN, wire.len() - 1] {
            let mut corrupt = wire.clone();
            corrupt[index] ^= 0x08;
            assert_eq!(
                decrypt_signed_delivery(&corrupt, PASSWORD).unwrap_err(),
                EncryptedDeliveryError::AuthenticationFailed
            );
        }
        for size in [MAGIC.len(), HEADER_LEN, HEADER_LEN + TAG_LEN] {
            assert_eq!(
                decrypt_signed_delivery(&wire[..size], PASSWORD).unwrap_err(),
                EncryptedDeliveryError::InvalidEnvelope
            );
        }
        let mut unknown = wire.clone();
        unknown[MAGIC.len()] += 1;
        assert!(is_encrypted_signed_delivery(&unknown));
        assert_eq!(
            decrypt_signed_delivery(&unknown, PASSWORD).unwrap_err(),
            EncryptedDeliveryError::InvalidEnvelope
        );
        assert!(!is_encrypted_signed_delivery(b"Salted__"));
        assert!(!is_encrypted_signed_delivery(b"PK\x03\x04"));
    }

    #[test]
    fn enforce_input_bounds_and_missing_password() {
        assert!(check_payload_len(MAX_SIGNED_DELIVERY_BYTES).is_ok());
        assert_eq!(
            check_payload_len(MAX_SIGNED_DELIVERY_BYTES + 1).unwrap_err(),
            EncryptedDeliveryError::TooLarge
        );
        assert_eq!(
            encrypt_signed_delivery(&[], PASSWORD).unwrap_err(),
            EncryptedDeliveryError::InvalidEnvelope
        );
        for password in ["", " \n ", &"a".repeat(MAX_PASSWORD_BYTES + 1)] {
            assert_eq!(
                encrypt_signed_delivery(&PAYLOAD, password).unwrap_err(),
                EncryptedDeliveryError::InvalidPassword
            );
        }
        let wire = encrypt_signed_delivery(&PAYLOAD, PASSWORD).unwrap();
        assert_eq!(
            decrypt_signed_delivery(&wire, "").unwrap_err(),
            EncryptedDeliveryError::InvalidPassword
        );
    }
}
