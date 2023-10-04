// SPDX-FileCopyrightText: 2023 David Ruescas <david@sequentech.io>
// SPDX-FileCopyrightText: 2020 Zcash Foundation
//
// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-License-Identifier: MIT

use base64::{engine::general_purpose, Engine as _};
use borsh::{BorshDeserialize, BorshSerialize};
use ed25519_dalek::Signature;
use ed25519_dalek::Signer;
use ed25519_dalek::SigningKey;
use ed25519_dalek::Verifier;
use ed25519_dalek::VerifyingKey;
use std::hash::Hash;
use std::hash::Hasher;
use std::io::{Error, ErrorKind};

use crate::rng::StrandRng;
use crate::serialization::{StrandDeserialize, StrandSerialize};
use crate::util::StrandError;

/// An ed25519-dalek backed signature.
#[derive(Clone)]
pub struct StrandSignature(Signature);
impl StrandSignature {
    // Clone is fallible when signature is implemented from OpenSSL, forcing other signature
    // implementations to conform to the same call
    pub fn try_clone(&self) -> Result<Self, StrandError> {
        Ok(self.clone())
     }
}

/// An ed25519-dalek backed signature verification key.
// Clone: Allows Configuration to be Clonable in Braid
#[derive(Clone)]
pub struct StrandSignaturePk(VerifyingKey);
impl StrandSignaturePk {
    /// Returns the verification key from this signing key.
    pub fn from(
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
}

/// An ed25519-dalek backed signing key.
#[derive(Clone)]
pub struct StrandSignatureSk(SigningKey);
impl StrandSignatureSk {
    /// Generates a key using randomness from rng::StrandRng.
    pub fn gen() -> Result<StrandSignatureSk, StrandError> {
        let mut rng = StrandRng;
        let sk = SigningKey::generate(&mut rng);
        Ok(StrandSignatureSk(sk))
    }
    /// Signs the message returning a signature.
    pub fn sign(&self, msg: &[u8]) -> Result<StrandSignature, StrandError> {
        Ok(StrandSignature(self.0.sign(msg)))
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

impl TryFrom<String> for StrandSignaturePk {
    type Error = StrandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let bytes: Vec<u8> = general_purpose::STANDARD_NO_PAD.decode(value)?;
        StrandSignaturePk::strand_deserialize(&bytes)
    }
}

impl TryFrom<StrandSignaturePk> for String {
    type Error = StrandError;

    fn try_from(value: StrandSignaturePk) -> Result<Self, Self::Error> {
        let bytes = value.strand_serialize()?;
        Ok(general_purpose::STANDARD_NO_PAD.encode(bytes))
    }
}

impl BorshSerialize for StrandSignatureSk {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let bytes: [u8; 32] = self.0.to_bytes();
        bytes.serialize(writer)
    }
}

impl BorshDeserialize for StrandSignatureSk {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let bytes = <[u8; 32]>::deserialize(buf)?;
        let pk = SigningKey::try_from(bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(StrandSignatureSk(pk))
    }
}

impl BorshSerialize for StrandSignaturePk {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let bytes: [u8; 32] = self.0.to_bytes();
        bytes.serialize(writer)
    }
}

impl BorshDeserialize for StrandSignaturePk {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let bytes = <[u8; 32]>::deserialize(buf)?;
        let pk = VerifyingKey::from_bytes(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(StrandSignaturePk(pk))
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
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let bytes = <[u8; 64]>::deserialize(buf)?;
        let signature = Signature::try_from(bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(StrandSignature(signature))
    }
}

impl TryFrom<String> for StrandSignatureSk {
    type Error = StrandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let bytes: Vec<u8> = general_purpose::STANDARD_NO_PAD.decode(value)?;
        StrandSignatureSk::strand_deserialize(&bytes)
    }
}

impl TryFrom<StrandSignatureSk> for String {
    type Error = StrandError;

    fn try_from(value: StrandSignatureSk) -> Result<Self, Self::Error> {
        let bytes = value.strand_serialize()?;
        Ok(general_purpose::STANDARD_NO_PAD.encode(bytes))
    }
}

impl TryFrom<String> for StrandSignature {
    type Error = StrandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let bytes: Vec<u8> = general_purpose::STANDARD_NO_PAD.decode(value)?;
        StrandSignature::strand_deserialize(&bytes)
    }
}

impl TryFrom<StrandSignature> for String {
    type Error = StrandError;

    fn try_from(value: StrandSignature) -> Result<Self, Self::Error> {
        let bytes = value.strand_serialize()?;
        Ok(general_purpose::STANDARD_NO_PAD.encode(bytes))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::serialization::{StrandDeserialize, StrandSerialize};

    #[test]
    pub fn test_signature() {
        let msg = b"ok";
        let msg2 = b"not_ok";
        let mut rng = StrandRng;

        let (vk_bytes, sig_bytes) = {
            let sk = StrandSignatureSk(SigningKey::generate(&mut rng));
            let sk_b = sk.strand_serialize().unwrap();
            let sk_d = StrandSignatureSk::strand_deserialize(&sk_b).unwrap();

            let sig = sk_d.sign(msg);

            let sig_bytes = sig.unwrap().strand_serialize().unwrap();
            let vk_bytes = StrandSignaturePk::from(&sk_d)
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
    fn test_string_serialization() {
        let message = b"ok";
        let other_message = b"not_ok";
        let mut rng = StrandRng;

        let (public_key_string, signature_string) = {
            let signing_key = StrandSignatureSk(SigningKey::generate(&mut rng));
            let signing_key_string: String = signing_key.try_into().unwrap();
            let signing_key_deserialized: StrandSignatureSk =
                signing_key_string.try_into().unwrap();

            let sig = signing_key_deserialized.sign(message);

            let signature_string: String = sig.unwrap().try_into().unwrap();
            let public_key_string: String =
                StrandSignaturePk::from(&signing_key_deserialized)
                    .unwrap()
                    .try_into()
                    .unwrap();

            (public_key_string, signature_string)
        };

        let public_key: StrandSignaturePk =
            public_key_string.try_into().unwrap();
        let signature = signature_string.try_into().unwrap();

        let ok = public_key.verify(&signature, message);
        assert!(ok.is_ok());

        let not_ok = public_key.verify(&signature, other_message);
        assert!(not_ok.is_err());
    }
}

pub fn info() -> String {
    format!("{}, FIPS_ENABLED: FALSE", module_path!())
}