// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What the service asks of the device that holds the keys. Providers live
//! only inside this service: every other component reaches keys through the
//! service's API, never through a provider of its own.
//!
//! Output formats are fixed so callers keep their existing verifiers:
//! - `Ed25519`: pure Ed25519 (RFC 8032) over the exact message, 64 bytes.
//! - `EcdsaP256`: ECDSA P-256 over SHA-256 of the exact message, `r || s`
//!   (64 bytes); [`crate::formats::ecdsa_raw_to_der`] gives the DER form.
//! - `HmacSha256`: 32 bytes.

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum Algorithm {
    Ed25519,
    EcdsaP256,
    EcdhP256,
    HmacSha256,
    Aes256Gcm,
}

impl Algorithm {
    pub fn is_asymmetric(self) -> bool {
        matches!(
            self,
            Algorithm::Ed25519 | Algorithm::EcdsaP256 | Algorithm::EcdhP256
        )
    }
}

/// The key attributes that show a key can't leave the device.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyAttributes {
    pub sensitive: bool,
    pub extractable: bool,
    pub always_sensitive: bool,
    pub never_extractable: bool,
    /// Generated on the device (`CKA_LOCAL`), not imported.
    pub local: bool,
}

/// Which device answers. The serial number is never reported in full: it
/// identifies physical hardware, so only its SHA-256 prefix is shown.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub backend: String,
    pub library: String,
    pub library_version: String,
    pub token_model: String,
    pub token_manufacturer: String,
    pub firmware_version: String,
    pub serial_sha256_prefix: String,
    /// True only for a physical device. SoftHSM is never reported as hardware.
    pub hardware: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("no key labelled '{0}'")]
    NoSuchKey(String),
    #[error("a key labelled '{0}' already exists")]
    Exists(String),
    #[error("{0} is not supported by this backend")]
    Unsupported(String),
    #[error("device error: {0}")]
    Device(String),
}

pub type ProviderResult<T> = std::result::Result<T, ProviderError>;

pub trait Provider: Send + Sync {
    fn info(&self) -> ProviderInfo;
    /// Generates a key on the device; returns the public key (raw Ed25519
    /// bytes, or the uncompressed SEC1 P-256 point) or nothing for a secret key.
    fn generate(&self, label: &str, algorithm: Algorithm) -> ProviderResult<Vec<u8>>;
    fn public_key(&self, label: &str, algorithm: Algorithm) -> ProviderResult<Vec<u8>>;
    fn sign(&self, label: &str, algorithm: Algorithm, message: &[u8]) -> ProviderResult<Vec<u8>>;
    /// ECDH with the key `label` and a peer's uncompressed P-256 point; the
    /// shared secret leaves the device for a software KDF, as `sealing.rs`
    /// already does.
    fn ecdh(&self, label: &str, peer_point: &[u8]) -> ProviderResult<Zeroizing<Vec<u8>>>;
    /// AES-256-GCM; returns `iv(12) || ciphertext || tag(16)`.
    fn encrypt(&self, label: &str, plaintext: &[u8], aad: &[u8]) -> ProviderResult<Vec<u8>>;
    fn decrypt(&self, label: &str, sealed: &[u8], aad: &[u8])
        -> ProviderResult<Zeroizing<Vec<u8>>>;
    fn random(&self, length: usize) -> ProviderResult<Vec<u8>>;
    fn destroy(&self, label: &str) -> ProviderResult<()>;
    fn attributes(&self, label: &str) -> ProviderResult<KeyAttributes>;
}
