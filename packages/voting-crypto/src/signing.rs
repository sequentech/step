// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Application signing keys backed exclusively by wbraid's signature scheme.

use anyhow::{Context as _, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use borsh::{BorshDeserialize, BorshSerialize};
use cryptography::context::{Context, RistrettoCtx};
use cryptography::utils::serialization::{Deserializable, Serializable};
use cryptography::utils::signatures::{SignatureScheme, Signer as _, Verifier as _};
use ed25519::pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

type Scheme = <RistrettoCtx as Context>::SignatureScheme;
type Rng = <RistrettoCtx as Context>::Rng;
type NativeSigningKey = <Scheme as SignatureScheme<Rng>>::Signer;
type NativeVerificationKey = <Scheme as SignatureScheme<Rng>>::Verifier;
type NativeSignature = <Scheme as SignatureScheme<Rng>>::Signature;

/// An Ed25519 signature in the platform's application records.
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Signature([u8; 64]);

/// A public Ed25519 verification key.
#[derive(Clone, Debug, PartialEq, Eq, std::hash::Hash, BorshSerialize)]
pub struct VerificationKey([u8; 32]);

/// A private Ed25519 signing seed; debug output never reveals it.
#[derive(Clone, BorshSerialize, BorshDeserialize)]
pub struct SigningKey([u8; 32]);

impl std::fmt::Debug for SigningKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SigningKey([redacted])")
    }
}

macro_rules! base64_encoding {
    ($name:ident, $size:expr) => {
        impl $name {
            /// Encode the raw canonical bytes as standard base64.
            pub fn to_b64_string(&self) -> Result<String> {
                Ok(STANDARD.encode(self.0))
            }
            /// Parse exactly the required number of raw canonical bytes.
            pub fn from_b64_string(encoded: &str) -> Result<Self> {
                let bytes = STANDARD.decode(encoded).context("invalid base64")?;
                if bytes.len() != $size {
                    return Self::from_standard_encoding(&bytes);
                }
                let bytes: [u8; $size] = bytes.try_into().map_err(|bytes: Vec<u8>| {
                    anyhow::anyhow!("expected {} bytes, got {}", $size, bytes.len())
                })?;
                Self::from_bytes(bytes)
            }
            /// Return the raw canonical bytes.
            pub fn to_bytes(&self) -> [u8; $size] {
                self.0
            }
        }
        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&hex::encode(self.0))
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let encoded = <String as Deserialize>::deserialize(deserializer)?;
                let bytes = hex::decode(&encoded).map_err(serde::de::Error::custom)?;
                let bytes = bytes.try_into().map_err(|bytes: Vec<u8>| {
                    serde::de::Error::custom(format!(
                        "expected {} bytes, got {}",
                        $size,
                        bytes.len()
                    ))
                })?;
                Self::from_bytes(bytes).map_err(serde::de::Error::custom)
            }
        }
    };
}
base64_encoding!(Signature, 64);
base64_encoding!(VerificationKey, 32);
base64_encoding!(SigningKey, 32);

impl Signature {
    /// Parse the signature with wbraid's canonical decoder.
    pub fn from_bytes(bytes: [u8; 64]) -> Result<Self> {
        NativeSignature::deser(&bytes)?;
        Ok(Self(bytes))
    }
    /// Clone the signature.
    pub fn try_clone(&self) -> Result<Self> {
        Ok(self.clone())
    }
}

impl VerificationKey {
    /// Derive a verification key through wbraid's signature scheme.
    pub fn from_sk(key: &SigningKey) -> Result<Self> {
        let native = NativeSigningKey::deser(&key.0)?;
        let bytes = Scheme::verifying_key(&native)
            .ser()
            .try_into()
            .map_err(|_| anyhow::anyhow!("unexpected verification key size"))?;
        Self::from_bytes(bytes)
    }
    /// Parse and validate a public key.
    pub fn from_bytes(bytes: [u8; 32]) -> Result<Self> {
        NativeVerificationKey::deser(&bytes)?;
        Ok(Self(bytes))
    }
    /// Verify an application signature through wbraid.
    pub fn verify(&self, signature: &Signature, message: &[u8]) -> Result<()> {
        let native = NativeVerificationKey::deser(&self.0)?;
        let signature = NativeSignature::deser(&signature.0)?;
        native.verify(message, &signature)?;
        Ok(())
    }
}

impl SigningKey {
    /// Generate a signing seed using wbraid's cryptographic RNG.
    pub fn generate() -> Result<Self> {
        let bytes = RistrettoCtx::gen_signing_key()
            .ser()
            .try_into()
            .map_err(|_| anyhow::anyhow!("unexpected signing key size"))?;
        Self::from_bytes(bytes)
    }
    /// Parse a signing seed.
    pub fn from_bytes(bytes: [u8; 32]) -> Result<Self> {
        NativeSigningKey::deser(&bytes)?;
        Ok(Self(bytes))
    }
    /// Sign an application record through wbraid.
    pub fn sign(&self, message: &[u8]) -> Result<Signature> {
        let native = NativeSigningKey::deser(&self.0)?;
        let signature: NativeSignature = native.sign(message);
        let bytes = signature
            .ser()
            .try_into()
            .map_err(|_| anyhow::anyhow!("unexpected signature size"))?;
        Signature::from_bytes(bytes)
    }
}

impl SigningKey {
    /// Parse standard PKCS#8 with wbraid's native Ed25519 implementation.
    pub fn from_standard_encoding(bytes: &[u8]) -> Result<Self> {
        let encoded = ed25519::pkcs8::KeypairBytes::from_pkcs8_der(bytes)?;
        let key = Self::from_bytes(encoded.secret_key)?;
        if let Some(public) = encoded.public_key {
            anyhow::ensure!(
                VerificationKey::from_sk(&key)?.to_bytes() == public.to_bytes(),
                "PKCS#8 public key does not match its signing seed"
            );
        }
        Ok(key)
    }
    pub fn to_der(&self) -> Result<Vec<u8>> {
        Ok(ed25519::pkcs8::KeypairBytes {
            secret_key: self.0,
            public_key: None,
        }
        .to_pkcs8_der()?
        .as_bytes()
        .to_vec())
    }
}
impl VerificationKey {
    /// Parse standard SubjectPublicKeyInfo with wbraid's native implementation.
    pub fn from_standard_encoding(bytes: &[u8]) -> Result<Self> {
        Self::from_bytes(ed25519::pkcs8::PublicKeyBytes::from_public_key_der(bytes)?.to_bytes())
    }
    pub fn to_der(&self) -> Result<Vec<u8>> {
        Ok(ed25519::pkcs8::PublicKeyBytes(self.0)
            .to_public_key_der()?
            .as_bytes()
            .to_vec())
    }
}
impl Signature {
    fn from_standard_encoding(_bytes: &[u8]) -> Result<Self> {
        anyhow::bail!("Ed25519 signatures must contain exactly 64 bytes")
    }
}

impl BorshDeserialize for VerificationKey {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let bytes = <[u8; 32]>::deserialize_reader(reader)?;
        Self::from_bytes(bytes)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
    }
}
