// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! Digital signature utilities and [context][`crate::context::Context`] dependency.
//!
//! # Examples
//! ```
//! use cryptography::context::Context;
//! use cryptography::context::RistrettoCtx as RCtx;
//! use cryptography::utils::signatures::{SignatureScheme, Signer, Verifier};
//!
//! let sk = RCtx::gen_signing_key();
//! let vk = sk.verifying_key();
//!
//! let message: &[u8] = b"message";
//! let signature = sk.sign(message);
//!
//! let vk = sk.verifying_key();
//! let ok = vk.verify(message, &signature);
//! assert!(ok.is_ok());
//! ```

use std::marker::PhantomData;

pub use ed25519::signature::{Error, Signer, Verifier};
use ed25519_dalek::{Signature, SigningKey, VerifyingKey, ed25519};

use crate::utils::Error as CryptoError;
use crate::utils::{
    rng::CRng,
    serialization::{Deserializable, Serializable, take},
};

/**
 * A digital signature scheme.
 *
 * This trait defines the types and methods required for a digital signature
 * scheme, such as [`Ed25519`].
 */
pub trait SignatureScheme<R: CRng> {
    /// The signer type, a private key used for signing.
    type Signer: Signer<Self::Signature> + Serializable + Deserializable + Clone;
    /// The verifier type, a public key used to verify signatures.
    type Verifier: Verifier<Self::Signature>
        + Serializable
        + Deserializable
        + Clone
        + PartialEq
        + Eq
        + std::hash::Hash
        + std::fmt::Debug;
    /// The signature type, a digital signature on some data.
    type Signature: Serializable + Deserializable + Clone;

    /// Generates a new private signing key.
    ///
    /// The corresponding public verification key can be obtained with `signing_key.verifying_key()`.
    fn gen_signing_key(rng: &mut R) -> Self::Signer;

    /// Gets the verifying key from a signing key.
    fn verifying_key(signer: &Self::Signer) -> Self::Verifier;

    /// Serializes a verifying key to a base64-encoded string.
    ///
    /// This is useful when reading/writing verification keys to/from configuration files
    /// in a generic context.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    fn verifier_to_base64_string(verifier: &Self::Verifier) -> Result<String, CryptoError>;

    /// Deserializes a verifying key from a base64-encoded string.
    ///
    /// This is the inverse of `verifier_to_base64_string`.
    ///
    /// # Errors
    ///
    /// Returns an error if parsing fails.
    fn verifier_from_base64_string(s: &str) -> Result<Self::Verifier, CryptoError>;

    /// Serializes a signing key to a base64-encoded string.
    ///
    /// This is useful when reading/writing signing keys to/from configuration files
    /// in a generic context.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    fn signer_to_base64_string(signer: &Self::Signer) -> Result<String, CryptoError>;

    /// Deserializes a signing key from a base64-encoded string.
    ///
    /// This is the inverse of `signer_to_base64_string`.
    ///
    /// # Errors
    ///
    /// Returns an error if parsing fails.
    fn signer_from_base64_string(s: &str) -> Result<Self::Signer, CryptoError>;
}

/**
 * Ed25519 digital signature scheme.
 *
 * This implementation uses the [`ed25519-dalek`](https://docs.rs/ed25519-dalek/latest/ed25519_dalek/) crate.
 *
 * # Examples
 * ```
 * use cryptography::context::Context;
 * use cryptography::context::RistrettoCtx as RCtx;
 * use cryptography::utils::signatures::{SignatureScheme, Signer, Verifier};
 *
 * let sk = RCtx::gen_signing_key();
 * let vk = sk.verifying_key();
 *
 * let message: &[u8] = b"message";
 * let signature = sk.sign(message);
 *
 * let vk = sk.verifying_key();
 * let ok = vk.verify(message, &signature);
 * assert!(ok.is_ok());
 * ```
 */
pub struct Ed25519<R: CRng>(PhantomData<R>);
impl<R: CRng> SignatureScheme<R> for Ed25519<R> {
    type Signer = ed25519_dalek::SigningKey;
    type Verifier = ed25519_dalek::VerifyingKey;
    type Signature = ed25519_dalek::Signature;

    fn gen_signing_key(rng: &mut R) -> ed25519_dalek::SigningKey {
        Self::Signer::generate(rng)
    }

    fn verifying_key(signer: &Self::Signer) -> Self::Verifier {
        signer.verifying_key()
    }

    fn verifier_to_base64_string(verifier: &Self::Verifier) -> Result<String, CryptoError> {
        use base64::{Engine as _, engine::general_purpose};

        // Ed25519 public keys are 32 bytes
        let bytes = verifier.to_bytes();
        Ok(general_purpose::STANDARD.encode(bytes))
    }

    fn verifier_from_base64_string(s: &str) -> Result<Self::Verifier, CryptoError> {
        use base64::{Engine as _, engine::general_purpose};

        let bytes = general_purpose::STANDARD.decode(s).map_err(|e| {
            CryptoError::DeserializationError(format!("Failed to decode base64: {e:?}"))
        })?;

        let bytes: [u8; 32] = bytes.try_into().map_err(|_| {
            CryptoError::DeserializationError("Invalid key length: expected 32 bytes".to_string())
        })?;

        VerifyingKey::from_bytes(&bytes).map_err(|e| {
            CryptoError::DeserializationError(format!("Failed to parse verifying key: {e:?}"))
        })
    }

    fn signer_to_base64_string(signer: &Self::Signer) -> Result<String, CryptoError> {
        use base64::{Engine as _, engine::general_purpose};

        // Ed25519 secret keys are 32 bytes
        let bytes = signer.to_bytes();
        Ok(general_purpose::STANDARD.encode(bytes))
    }

    fn signer_from_base64_string(s: &str) -> Result<Self::Signer, CryptoError> {
        use base64::{Engine as _, engine::general_purpose};

        let bytes = general_purpose::STANDARD.decode(s).map_err(|e| {
            CryptoError::DeserializationError(format!("Failed to decode base64: {e:?}"))
        })?;

        let bytes: [u8; 32] = bytes.try_into().map_err(|_| {
            CryptoError::DeserializationError("Invalid key length: expected 32 bytes".to_string())
        })?;

        Ok(SigningKey::from_bytes(&bytes))
    }
}

impl Serializable for SigningKey {
    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.as_bytes());
    }
}
impl Deserializable for SigningKey {
    fn read(input: &mut &[u8]) -> Result<Self, CryptoError> {
        let bytes = take(input, ed25519_dalek::SECRET_KEY_LENGTH)?;
        let array: [u8; ed25519_dalek::SECRET_KEY_LENGTH] = bytes
            .try_into()
            .expect("take returns exactly the requested bytes");
        Ok(SigningKey::from_bytes(&array))
    }
}

impl Serializable for VerifyingKey {
    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.as_bytes());
    }
}
impl Deserializable for VerifyingKey {
    fn read(input: &mut &[u8]) -> Result<Self, CryptoError> {
        let bytes = take(input, ed25519_dalek::PUBLIC_KEY_LENGTH)?;
        let array: [u8; ed25519_dalek::PUBLIC_KEY_LENGTH] = bytes
            .try_into()
            .expect("take returns exactly the requested bytes");
        Ok(VerifyingKey::from_bytes(&array)?)
    }
}

impl Serializable for Signature {
    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_bytes());
    }
}
impl Deserializable for Signature {
    fn read(input: &mut &[u8]) -> Result<Self, CryptoError> {
        let bytes = take(input, ed25519_dalek::SIGNATURE_LENGTH)?;
        let array: [u8; ed25519_dalek::SIGNATURE_LENGTH] = bytes
            .try_into()
            .expect("take returns exactly the requested bytes");
        Ok(Signature::from_bytes(&array))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn test_signatures_context() {
        use crate::context::Context;
        use crate::context::RistrettoCtx;

        let sk = RistrettoCtx::gen_signing_key();
        let vk = sk.verifying_key();

        let mut csprng = RistrettoCtx::get_rng();
        let message: &[u8] = &csprng.next_u64().to_be_bytes();
        let signature = sk.sign(message);
        let ok = vk.verify(message, &signature);
        assert!(ok.is_ok());

        let signature = sk.sign(&[]);
        let ok = vk.verify(&[], &signature);

        assert!(ok.is_ok());
    }

    #[test]
    fn test_signatures_serialization() {
        use crate::context::Context;
        use crate::context::RistrettoCtx;

        let sk = RistrettoCtx::gen_signing_key();
        let vk = sk.verifying_key();

        let mut csprng = RistrettoCtx::get_rng();
        let message: &[u8] = &csprng.next_u64().to_be_bytes();
        let signature = sk.sign(message);

        let sk_bytes = sk.ser();
        let vk_bytes = vk.ser();
        let sig_bytes = signature.ser();

        let sk = SigningKey::deser(&sk_bytes).unwrap();
        let vk = VerifyingKey::deser(&vk_bytes).unwrap();
        let signature = Signature::deser(&sig_bytes).unwrap();

        let ok = vk.verify(message, &signature);
        assert!(ok.is_ok());

        let sk_bytes = sk.ser();
        let vk_bytes = vk.ser();
        let sig_bytes = signature.ser();

        let sk = SigningKey::deser(&sk_bytes).unwrap();
        let vk = VerifyingKey::deser(&vk_bytes).unwrap();
        let signature = Signature::deser(&sig_bytes).unwrap();

        let ok = vk.verify(message, &signature);
        assert!(ok.is_ok());

        let signature = sk.sign(&[]);
        let ok = vk.verify(&[], &signature);

        assert!(ok.is_ok());
    }
}
