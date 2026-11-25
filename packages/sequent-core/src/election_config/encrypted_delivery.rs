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

#[cfg(all(feature = "election_config_signing", not(target_arch = "wasm32")))]
pub use credentials_envelope::{
    decrypt_signed_delivery_parts, encrypt_signed_delivery_with_credentials,
    DeliveryParts, MAX_CREDENTIALS_SIDECAR_BYTES,
    MAX_ENCRYPTED_SIGNED_DELIVERY_WITH_CREDENTIALS_BYTES,
};

/// V2, compiled only for native targets with `election_config_signing`
/// (never `wasm32`, so never browser JS): the V1 envelope with version byte
/// 2, whose plaintext is a 4-byte big-endian length, a messaging credentials
/// sidecar of that length, then the signed ZIP. The same header is the AAD
/// and the same password KDF applies. V1 and its limits are unchanged:
/// [`decrypt_signed_delivery`] still refuses any version but 1, so no V1
/// reader returns a ZIP without its sidecar.
#[cfg(all(feature = "election_config_signing", not(target_arch = "wasm32")))]
mod credentials_envelope {
    use super::*;
    use zeroize::Zeroizing;

    const VERSION_WITH_CREDENTIALS: u8 = 2;
    const SIDECAR_LEN_BYTES: usize = 4;
    pub const MAX_CREDENTIALS_SIDECAR_BYTES: usize = 16 * 1024;
    pub const MAX_ENCRYPTED_SIGNED_DELIVERY_WITH_CREDENTIALS_BYTES: usize =
        MAX_ENCRYPTED_SIGNED_DELIVERY_BYTES
            + SIDECAR_LEN_BYTES
            + MAX_CREDENTIALS_SIDECAR_BYTES;

    fn check_sidecar_len(len: usize) -> Result<(), EncryptedDeliveryError> {
        if len == 0 || len > MAX_CREDENTIALS_SIDECAR_BYTES {
            return Err(EncryptedDeliveryError::InvalidEnvelope);
        }
        Ok(())
    }

    /// Seal a V2 plaintext given as parts. The buffer is allocated once at
    /// its final size, so no reallocation leaves a plaintext copy, and it is
    /// cleared on any early return.
    fn seal(
        parts: &[&[u8]],
        password: &str,
    ) -> Result<Vec<u8>, EncryptedDeliveryError> {
        let mut header = [0; HEADER_LEN];
        header[..MAGIC.len()].copy_from_slice(MAGIC);
        header[MAGIC.len()] = VERSION_WITH_CREDENTIALS;
        SystemRandom::new()
            .fill(&mut header[SALT_START..])
            .map_err(|_| EncryptedDeliveryError::RandomUnavailable)?;
        let body: usize = parts.iter().map(|part| part.len()).sum();
        let mut wire =
            Zeroizing::new(Vec::with_capacity(HEADER_LEN + body + TAG_LEN));
        wire.extend_from_slice(&header);
        for part in parts {
            wire.extend_from_slice(part);
        }
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
        // Ciphertext only from here: hand the allocation out uncopied.
        Ok(std::mem::take(&mut *wire))
    }

    pub fn encrypt_signed_delivery_with_credentials(
        signed_zip: &[u8],
        credentials: &[u8],
        password: &str,
    ) -> Result<Vec<u8>, EncryptedDeliveryError> {
        check_payload_len(signed_zip.len())?;
        check_sidecar_len(credentials.len())?;
        check_password(password)?;
        let len = (credentials.len() as u32).to_be_bytes();
        seal(&[&len, credentials, signed_zip], password)
    }

    /// A V1 or V2 envelope's contents. `credentials` (V2 only) are the
    /// sidecar's exact bytes; never printed, cleared on drop.
    pub struct DeliveryParts {
        pub signed_zip: Vec<u8>,
        pub credentials: Option<Zeroizing<Vec<u8>>>,
    }

    impl fmt::Debug for DeliveryParts {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("DeliveryParts")
                .field("signed_zip_len", &self.signed_zip.len())
                .field(
                    "credentials",
                    &self.credentials.as_ref().map(|_| "<redacted>"),
                )
                .finish()
        }
    }

    fn split_credentials(
        body: &[u8],
    ) -> Result<(&[u8], &[u8]), EncryptedDeliveryError> {
        let prefix: [u8; SIDECAR_LEN_BYTES] = body
            .get(..SIDECAR_LEN_BYTES)
            .and_then(|b| b.try_into().ok())
            .ok_or(EncryptedDeliveryError::InvalidEnvelope)?;
        let len = u32::from_be_bytes(prefix) as usize;
        check_sidecar_len(len)?;
        let zip_start = SIDECAR_LEN_BYTES + len;
        if body.len() <= zip_start {
            return Err(EncryptedDeliveryError::InvalidEnvelope);
        }
        Ok((&body[SIDECAR_LEN_BYTES..zip_start], &body[zip_start..]))
    }

    /// V1 goes through the unchanged [`decrypt_signed_delivery`] (and its
    /// own size limit); V2 is checked the same way before any password work.
    pub fn decrypt_signed_delivery_parts(
        bytes: &[u8],
        password: &str,
    ) -> Result<DeliveryParts, EncryptedDeliveryError> {
        if bytes.len() > MAX_ENCRYPTED_SIGNED_DELIVERY_WITH_CREDENTIALS_BYTES {
            return Err(EncryptedDeliveryError::TooLarge);
        }
        match bytes.get(MAGIC.len()) {
            Some(&VERSION) => {
                return Ok(DeliveryParts {
                    signed_zip: decrypt_signed_delivery(bytes, password)?,
                    credentials: None,
                });
            }
            Some(&VERSION_WITH_CREDENTIALS)
                if is_encrypted_signed_delivery(bytes)
                    && bytes.len() > HEADER_LEN + TAG_LEN => {}
            _ => return Err(EncryptedDeliveryError::InvalidEnvelope),
        }
        check_password(password)?;
        let (header, ciphertext) = bytes.split_at(HEADER_LEN);
        let mut body = Zeroizing::new(ciphertext.to_vec());
        let len = key(password, header)
            .and_then(|cipher| {
                cipher.open_in_place(nonce(header), Aad::from(header), &mut body[..])
            })
            .map_err(|_| EncryptedDeliveryError::AuthenticationFailed)?
            .len();
        body.truncate(len);
        // Authenticated already; a malformed frame is still refused whole.
        let (credentials, signed_zip) = split_credentials(&body)?;
        check_payload_len(signed_zip.len())?;
        Ok(DeliveryParts {
            signed_zip: signed_zip.to_vec(),
            credentials: Some(Zeroizing::new(credentials.to_vec())),
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        const PASSWORD: &str = "correct horse ⚿";
        const PAYLOAD: [u8; 6] = [0, 1, 2, 255, 80, 75];
        const SIDECAR: &[u8] = b"{\"synthetic\":\"sidecar-canary\"}";

        #[test]
        fn v1_limits_and_reader_are_unchanged() {
            assert_eq!(
                MAX_ENCRYPTED_SIGNED_DELIVERY_BYTES,
                MAX_SIGNED_DELIVERY_BYTES + HEADER_LEN + TAG_LEN
            );
            let wire =
                encrypt_signed_delivery_with_credentials(&PAYLOAD, SIDECAR, PASSWORD)
                    .unwrap();
            assert_eq!(wire[MAGIC.len()], VERSION_WITH_CREDENTIALS);
            assert_eq!(
                decrypt_signed_delivery(&wire, PASSWORD).unwrap_err(),
                EncryptedDeliveryError::InvalidEnvelope
            );
            let v1 = encrypt_signed_delivery(&PAYLOAD, PASSWORD).unwrap();
            let parts = decrypt_signed_delivery_parts(&v1, PASSWORD).unwrap();
            assert_eq!(parts.signed_zip, PAYLOAD);
            assert!(parts.credentials.is_none());
        }

        #[test]
        fn v2_carries_the_sidecar_beside_the_zip_under_one_password() {
            let wire =
                encrypt_signed_delivery_with_credentials(&PAYLOAD, SIDECAR, PASSWORD)
                    .unwrap();
            assert!(!wire.windows(SIDECAR.len()).any(|w| w == SIDECAR));
            let parts = decrypt_signed_delivery_parts(&wire, PASSWORD).unwrap();
            assert_eq!(parts.signed_zip, PAYLOAD);
            assert_eq!(parts.credentials.as_deref().map(Vec::as_slice), Some(SIDECAR));
            assert!(!format!("{parts:?}").contains("canary"));
            assert_eq!(
                decrypt_signed_delivery_parts(&wire, "wrong").unwrap_err(),
                EncryptedDeliveryError::AuthenticationFailed
            );
        }

        #[test]
        fn v2_framing_is_authenticated_and_bounded() {
            let wire =
                encrypt_signed_delivery_with_credentials(&PAYLOAD, SIDECAR, PASSWORD)
                    .unwrap();
            // The version byte is in the AAD: a downgrade fails to open.
            let mut downgraded = wire.clone();
            downgraded[MAGIC.len()] = VERSION;
            assert_eq!(
                decrypt_signed_delivery(&downgraded, PASSWORD).unwrap_err(),
                EncryptedDeliveryError::AuthenticationFailed
            );
            for index in [SALT_START, HEADER_LEN, HEADER_LEN + 3, wire.len() - 1] {
                let mut corrupt = wire.clone();
                corrupt[index] ^= 0x01;
                assert_eq!(
                    decrypt_signed_delivery_parts(&corrupt, PASSWORD).unwrap_err(),
                    EncryptedDeliveryError::AuthenticationFailed
                );
            }
            let too_long = vec![b'x'; MAX_CREDENTIALS_SIDECAR_BYTES + 1];
            for (zip, sidecar) in [
                (&PAYLOAD[..], &[][..]),
                (&PAYLOAD[..], too_long.as_slice()),
                (&[][..], SIDECAR),
            ] {
                assert_eq!(
                    encrypt_signed_delivery_with_credentials(zip, sidecar, PASSWORD)
                        .unwrap_err(),
                    EncryptedDeliveryError::InvalidEnvelope
                );
            }
            // Authentic but malformed frames.
            for body in [
                vec![0, 0, 0],
                [&0u32.to_be_bytes()[..], &PAYLOAD[..]].concat(),
                [&(SIDECAR.len() as u32).to_be_bytes()[..], SIDECAR].concat(),
                [&u32::MAX.to_be_bytes()[..], SIDECAR, &PAYLOAD[..]].concat(),
            ] {
                let sealed = seal(&[body.as_slice()], PASSWORD).unwrap();
                assert_eq!(
                    decrypt_signed_delivery_parts(&sealed, PASSWORD).unwrap_err(),
                    EncryptedDeliveryError::InvalidEnvelope
                );
            }
            let mut future = wire;
            future[MAGIC.len()] = 3;
            assert_eq!(
                decrypt_signed_delivery_parts(&future, PASSWORD).unwrap_err(),
                EncryptedDeliveryError::InvalidEnvelope
            );
            let oversized =
                vec![0_u8; MAX_ENCRYPTED_SIGNED_DELIVERY_WITH_CREDENTIALS_BYTES + 1];
            assert_eq!(
                decrypt_signed_delivery_parts(&oversized, PASSWORD).unwrap_err(),
                EncryptedDeliveryError::TooLarge
            );
        }
    }
}
