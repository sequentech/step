// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-License-Identifier: MIT
//! # Examples
//!
//! ```
//! // This example shows how to sign bytes, and serialize
//! // and deserialize a signature.
//! use strand::rng::StrandRng;
//! use strand::signature::StrandSignatureSk;
//! use strand::signature::StrandSignaturePk;
//! use strand::signature::StrandSignature;
//! use strand::serialization::{StrandDeserialize, StrandSerialize};
//!
//! let msg = b"message";
//! let mut rng = StrandRng;
//! // generate signing (private) and verification (public) keys
//! let sk = StrandSignatureSk::generate().unwrap();
//! let vk = StrandSignaturePk::from_sk(&sk).unwrap();
//! // sign data
//! let sig = sk.sign(msg);
//!
//! // serialize + deserialize
//! let sig_bytes = sig.unwrap().strand_serialize().unwrap();
//! let sig = StrandSignature::strand_deserialize(&sig_bytes).unwrap();
//! // verify
//! let ok = vk.verify(&sig, msg);
//! assert!(ok.is_ok());
//! ```

use base64::{engine::general_purpose, Engine as _};
use borsh::{BorshDeserialize, BorshSerialize};
use ed25519_dalek::pkcs8::DecodePrivateKey;
use ed25519_dalek::pkcs8::DecodePublicKey;
use ed25519_dalek::pkcs8::EncodePrivateKey;
use ed25519_dalek::pkcs8::EncodePublicKey;
use ed25519_dalek::Signature;
use ed25519_dalek::Signer;
use ed25519_dalek::SigningKey;
use ed25519_dalek::Verifier;
use ed25519_dalek::VerifyingKey;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::hash::Hash;
use std::hash::Hasher;
use std::io::Error;

use crate::rng::StrandRng;
use crate::util;
use crate::util::StrandError;

/// An ed25519-dalek backed signature.
#[derive(Clone)]
pub struct StrandSignature(Signature);
impl StrandSignature {
    // Clone is fallible when signature is implemented from OpenSSL, forcing
    // other signature implementations to conform to the same method signature.
    pub fn try_clone(&self) -> Result<Self, StrandError> {
        Ok(self.clone())
    }

    pub fn to_bytes(&self) -> [u8; 64] {
        self.0.to_bytes()
    }

    pub fn from_bytes(bytes: [u8; 64]) -> Result<StrandSignature, StrandError> {
        let signature = Signature::from(bytes);

        Ok(StrandSignature(signature))
    }

    pub fn to_b64_string(&self) -> Result<String, StrandError> {
        let bytes = self.0.to_bytes();
        Ok(general_purpose::STANDARD.encode(bytes))
    }

    pub fn from_b64_string(b64: &str) -> Result<StrandSignature, StrandError> {
        let bytes: Vec<u8> = general_purpose::STANDARD.decode(b64)?;
        let bytes = util::to_u8_array(&bytes)?;
        Self::from_bytes(bytes)
    }
}

/// An ed25519-dalek backed signature verification key.
// Clone: Allows Configuration to be Clonable in Braid
#[derive(Clone)]
pub struct StrandSignaturePk(VerifyingKey);
impl StrandSignaturePk {
    /// Returns the verification key from this signing key.
    pub fn from_sk(
        sk: &StrandSignatureSk,
    ) -> Result<StrandSignaturePk, StrandError> {
        Ok(StrandSignaturePk(VerifyingKey::from(&sk.0)))
    }
    /// Verifies the signature given the message. Returns Ok(()) if the
    /// verification passes.
    pub fn verify(
        &self,
        signature: &StrandSignature,
        msg: &[u8],
    ) -> Result<(), StrandError> {
        Ok(self.0.verify(msg, &signature.0)?)
    }

    /// Returns a spki der representation.
    pub fn to_der(&self) -> Result<Vec<u8>, StrandError> {
        let doc = self
            .0
            .to_public_key_der()
            .map_err(|e| StrandError::Generic(e.to_string()))?;

        Ok(doc.as_bytes().to_vec())
    }

    /// Parses a spki der representation.
    pub fn from_der(bytes: &[u8]) -> Result<StrandSignaturePk, StrandError> {
        let sk = VerifyingKey::from_public_key_der(bytes)
            .map_err(|e| Error::other(e.to_string()))?;

        Ok(StrandSignaturePk(sk))
    }

    /// Returns a base64 encoded spki der representation.
    pub fn to_der_b64_string(&self) -> Result<String, StrandError> {
        let bytes = self.to_der()?;
        Ok(general_purpose::STANDARD.encode(bytes))
    }

    /// Parses a base 64 encoded spki der representation.
    pub fn from_der_b64_string(
        b64_der: &str,
    ) -> Result<StrandSignaturePk, StrandError> {
        let bytes: Vec<u8> = general_purpose::STANDARD.decode(b64_der)?;
        Self::from_der(&bytes)
    }

    /// Parses a raw byte representation, can be used to read raw bytes inside
    /// spki.
    pub fn from_bytes(
        bytes: [u8; 32],
    ) -> Result<StrandSignaturePk, StrandError> {
        let sk = VerifyingKey::from_bytes(&bytes).map_err(Error::other)?;

        Ok(StrandSignaturePk(sk))
    }
}

/// An ed25519-dalek backed signing key.
#[derive(Clone)]
pub struct StrandSignatureSk(SigningKey);
impl StrandSignatureSk {
    /// Generates a key using randomness from rng::StrandRng.
    pub fn generate() -> Result<StrandSignatureSk, StrandError> {
        let mut rng = StrandRng;
        let sk = SigningKey::generate(&mut rng);
        Ok(StrandSignatureSk(sk))
    }
    /// Signs the message returning a signature.
    ///
    /// The bytes will be hashed using sha512.
    /// https://docs.rs/ed25519-dalek/latest/ed25519_dalek/struct.SigningKey.html
    pub fn sign(&self, msg: &[u8]) -> Result<StrandSignature, StrandError> {
        Ok(StrandSignature(self.0.sign(msg)))
    }

    /// Returns a pkcs#8 v1 der representation.
    pub fn to_der(&self) -> Result<Vec<u8>, StrandError> {
        // We want to force pkcs#8 v1.0 which does not include the public key
        // Otherwise this causes problems with rcgen::KeyPair::from_der
        let kpb = ed25519_dalek::pkcs8::KeypairBytes {
            secret_key: self.0.to_bytes(),
            public_key: None,
        };
        let doc = kpb
            .to_pkcs8_der()
            .map_err(|e| StrandError::Generic(e.to_string()))?;

        Ok(doc.as_bytes().to_vec())
    }

    /// Parses a pkcs#8 v1 or v2 der representation.
    pub fn from_der(bytes: &[u8]) -> Result<StrandSignatureSk, StrandError> {
        let sk = SigningKey::from_pkcs8_der(bytes)
            .map_err(|e| Error::other(e.to_string()))?;

        Ok(StrandSignatureSk(sk))
    }

    /// Returns a base64 encoded pkcs#8 v1 der representation.
    pub fn to_der_b64_string(&self) -> Result<String, StrandError> {
        let bytes = self.to_der()?;
        Ok(general_purpose::STANDARD.encode(bytes))
    }

    /// Parses a base64 encoded pkcs#8 v1 der representation.
    pub fn from_der_b64_string(
        b64_der: &str,
    ) -> Result<StrandSignatureSk, StrandError> {
        let bytes: Vec<u8> = general_purpose::STANDARD.decode(b64_der)?;
        Self::from_der(&bytes)
    }
}

impl PartialEq for StrandSignaturePk {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_ref() == other.0.as_ref()
    }
}
impl Hash for StrandSignaturePk {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_ref().hash(state);
    }
}
impl std::fmt::Debug for StrandSignaturePk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &hex::encode(self.0.as_ref())[0..10])
    }
}
impl Eq for StrandSignaturePk {}

impl BorshSerialize for StrandSignaturePk {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let bytes: [u8; 32] = self.0.to_bytes();
        BorshSerialize::serialize(&bytes, writer)
    }
}

impl BorshDeserialize for StrandSignaturePk {
    fn deserialize_reader<R: std::io::Read>(
        reader: &mut R,
    ) -> Result<Self, std::io::Error> {
        let bytes = <[u8; 32]>::deserialize_reader(reader)?;

        StrandSignaturePk::from_bytes(bytes).map_err(Error::other)
    }
}

impl BorshSerialize for StrandSignature {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let bytes: [u8; 64] = self.0.into();
        bytes.serialize(writer)
    }
}

impl BorshDeserialize for StrandSignature {
    fn deserialize_reader<R: std::io::Read>(
        reader: &mut R,
    ) -> Result<Self, std::io::Error> {
        let bytes = <[u8; 64]>::deserialize_reader(reader)?;
        StrandSignature::from_bytes(bytes).map_err(Error::other)
    }
}

impl Serialize for StrandSignaturePk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = self.0.to_bytes();
        serializer.serialize_str(hex::encode(bytes).as_str())
    }
}

impl<'de> Deserialize<'de> for StrandSignaturePk {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringVisitor;

        impl<'de> Visitor<'de> for StringVisitor {
            type Value = StrandSignaturePk;

            fn expecting(
                &self,
                formatter: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                formatter.write_str("a string containing hex-encoded data")
            }

            fn visit_str<E>(self, v: &str) -> Result<StrandSignaturePk, E>
            where
                E: de::Error,
            {
                let bytes_vec = hex::decode(v).map_err(de::Error::custom)?;
                if bytes_vec.len() != 64 {
                    return Err(de::Error::invalid_length(
                        bytes_vec.len(),
                        &self,
                    ));
                }
                let bytes = bytes_vec
                    .try_into()
                    .map_err(|_| de::Error::custom("Invalid length"))?;
                StrandSignaturePk::from_bytes(bytes).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_string(StringVisitor)
    }
}

impl std::fmt::Debug for StrandSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &hex::encode(self.0.to_bytes()))
    }
}

impl Serialize for StrandSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = self.0.to_bytes();
        serializer.serialize_str(hex::encode(bytes).as_str())
    }
}

impl<'de> Deserialize<'de> for StrandSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringVisitor;

        impl<'de> Visitor<'de> for StringVisitor {
            type Value = StrandSignature;

            fn expecting(
                &self,
                formatter: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                formatter.write_str("a string containing hex-encoded data")
            }

            fn visit_str<E>(self, v: &str) -> Result<StrandSignature, E>
            where
                E: de::Error,
            {
                let bytes_vec = hex::decode(v).map_err(de::Error::custom)?;
                if bytes_vec.len() != 64 {
                    return Err(de::Error::invalid_length(
                        bytes_vec.len(),
                        &self,
                    ));
                }
                let bytes = bytes_vec
                    .try_into()
                    .map_err(|_| de::Error::custom("Invalid length"))?;
                StrandSignature::from_bytes(bytes).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_string(StringVisitor)
    }
}

pub fn info() -> String {
    format!("{}, FIPS_ENABLED: FALSE", module_path!())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::serialization::{StrandDeserialize, StrandSerialize};

    /*
        openssl genpkey -algorithm ed25519 -outform DER -out test25519.der
        openssl base64 -in test25519.der -out test25519.b64
        openssl pkey -in test25519.der -outform der -pubout -out pk.der
        openssl base64 -in pk.der -out pk.b64
    */
    const SK_STR: &str =
        "MC4CAQAwBQYDK2VwBCIEII6bMx4lMnY83pVId7YbeOYGHoSZAnP7KjR/WsjaXkc9";
    const PK_STR: &str =
        "MCowBQYDK2VwAyEApnH8A4iAauMx0tZOx9JrpnG37adrUPiXg5klJ7fZRLU=";

    #[test]
    pub fn test_signature() {
        let msg = b"ok";
        let msg2 = b"not_ok";
        let mut rng = StrandRng;

        let (vk_bytes, sig_bytes) = {
            let sk = StrandSignatureSk(SigningKey::generate(&mut rng));
            let sk_b = sk.to_der().unwrap();
            let sk_d = StrandSignatureSk::from_der(&sk_b).unwrap();

            let sig = sk_d.sign(msg);

            let sig_bytes = sig.unwrap().strand_serialize().unwrap();
            let vk_bytes = StrandSignaturePk::from_sk(&sk_d)
                .unwrap()
                .strand_serialize()
                .unwrap();

            (vk_bytes, sig_bytes)
        };

        let vk = StrandSignaturePk::strand_deserialize(&vk_bytes).unwrap();
        let sig = StrandSignature::strand_deserialize(&sig_bytes).unwrap();

        let ok = vk.verify(&sig, msg);
        assert!(ok.is_ok());

        let not_ok = vk.verify(&sig, msg2);
        assert!(not_ok.is_err());
    }

    #[test]
    pub fn test_der_roundtrip() {
        let msg = b"ok";
        let msg2 = b"not_ok";

        let (vk_bytes, sig_bytes) = {
            let sk = StrandSignatureSk::generate().unwrap();
            let sk_der = sk.to_der().unwrap();
            let sk_d = StrandSignatureSk::from_der(&sk_der).unwrap();

            let sig = sk_d.sign(msg).unwrap();

            let sig_bytes = sig.strand_serialize().unwrap();
            let vk_bytes =
                StrandSignaturePk::from_sk(&sk_d).unwrap().to_der().unwrap();

            (vk_bytes, sig_bytes)
        };

        let vk = StrandSignaturePk::from_der(&vk_bytes).unwrap();
        let sig = StrandSignature::strand_deserialize(&sig_bytes).unwrap();

        let ok = vk.verify(&sig, msg);
        assert!(ok.is_ok());

        let not_ok = vk.verify(&sig, msg2);
        assert!(not_ok.is_err());
    }

    #[test]
    fn test_string_serialization() {
        let message = b"ok";
        let other_message = b"not_ok";
        let mut rng = StrandRng;

        let (public_key_string, signature_string) = {
            let signing_key = StrandSignatureSk(SigningKey::generate(&mut rng));
            let signing_key_string: String =
                signing_key.to_der_b64_string().unwrap();
            let signing_key_deserialized: StrandSignatureSk =
                StrandSignatureSk::from_der_b64_string(&signing_key_string)
                    .unwrap();

            let sig = signing_key_deserialized.sign(message);

            let signature_string: String =
                sig.unwrap().to_b64_string().unwrap();
            let public_key_string: String =
                StrandSignaturePk::from_sk(&signing_key_deserialized)
                    .unwrap()
                    .to_der_b64_string()
                    .unwrap();

            (public_key_string, signature_string)
        };

        let public_key: StrandSignaturePk =
            StrandSignaturePk::from_der_b64_string(&public_key_string).unwrap();
        let signature =
            StrandSignature::from_b64_string(&signature_string).unwrap();

        let ok = public_key.verify(&signature, message);
        assert!(ok.is_ok());

        let not_ok = public_key.verify(&signature, other_message);
        assert!(not_ok.is_err());
    }

    #[test]
    fn test_parse_openssl_b64_der() {
        let message = b"ok\n";
        /*
        data.txt contained one line with "ok"
        openssl pkeyutl -sign -out data.txt.signature -in data.txt -inkey test25519.der -rawin
        base64 data.txt.signature data.txt.signature.b64

        the signature can be verified with
        openssl pkeyutl -verify -pubin -inkey pk.der -rawin -in data.txt -sigfile data.txt.signature
        */
        let signature_string = "nMJ6twxCU1fogkNNNsmvdlTsdeiYn5SnDrjF0Jy5zURG/Z0ZdSY3JIj7Z2pQ4ANHMTBXzRDF60AtQ8EW7WQQBQ==".to_string();
        let secret_key: StrandSignatureSk =
            StrandSignatureSk::from_der_b64_string(SK_STR).unwrap();
        let public_key: StrandSignaturePk =
            StrandSignaturePk::from_der_b64_string(PK_STR).unwrap();

        let sig = secret_key.sign(message).unwrap();
        let ok = public_key.verify(&sig, message);

        assert!(ok.is_ok());

        let signature: StrandSignature =
            StrandSignature::from_b64_string(&signature_string).unwrap();
        let ok = public_key.verify(&signature, message);

        assert!(ok.is_ok());
    }
}
