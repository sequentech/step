// SPDX-FileCopyrightText: 2023 David Ruescas <david@sequentech.io>
// SPDX-FileCopyrightText: 2020 Zcash Foundation
//
// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-License-Identifier: MIT

use base64::{engine::general_purpose, Engine as _};
use borsh::{BorshDeserialize, BorshSerialize};

use ecdsa::signature::{DigestSigner, DigestVerifier};
use ecdsa::Signature;
use ecdsa::SigningKey;
use ecdsa::VerifyingKey;
use p384::pkcs8::{
    DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey,
};
use p384::NistP384;
use std::io::{Error, ErrorKind};

// The rustcrypto signature backend does not support FIPS,
// we use the rustcrypto hashing backend.
use crate::hashing::rustcrypto::Digest;
use crate::hashing::rustcrypto::RustCryptoHasher;
use crate::rng::StrandRng;
use crate::util::StrandError;

type Curve = NistP384;

/// An rustcrypto ecdsa signature.
#[derive(Clone)]
pub struct StrandSignature(Signature<Curve>);
impl StrandSignature {
    // Clone is fallible when signature is implemented from OpenSSL, forcing
    // other signature implementations to conform to the same call
    pub fn try_clone(&self) -> Result<Self, StrandError> {
        Ok(self.clone())
    }

    pub fn to_der(&self) -> Result<Vec<u8>, StrandError> {
        let bytes = self.0.to_der().as_bytes().to_vec();
        Ok(bytes)
    }

    pub fn from_der(bytes: &[u8]) -> Result<Self, StrandError> {
        let signature = Signature::from_der(&bytes)?;

        Ok(StrandSignature(signature))
    }
}

/// An rustcrypto ecdsa signature verification key.
// Clone: Allows Configuration to be Clonable in Braid
#[derive(Clone)]
pub struct StrandSignaturePk(VerifyingKey<Curve>);
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
        let mut digest: RustCryptoHasher =
            crate::hash::rust_crypto_ecdsa_hasher();
        digest.update(msg);

        Ok(self.0.verify_digest(digest, &signature.0)?)
    }

    pub fn to_der(&self) -> Result<Vec<u8>, StrandError> {
        let d = self
            .0
            .to_public_key_der()
            .map_err(|e| StrandError::Generic(e.to_string()))?;

        Ok(d.as_bytes().to_vec())
    }

    pub fn from_der(bytes: &[u8]) -> Result<Self, StrandError> {
        let pk = VerifyingKey::from_public_key_der(&bytes)
            .map_err(|e| StrandError::Generic(e.to_string()))?;

        Ok(StrandSignaturePk(pk))
    }
}

/// An rustcrypto ecdsa signing key.
// #[derive(Clone)]
pub struct StrandSignatureSk(SigningKey<Curve>);
impl StrandSignatureSk {
    /// Generates a key using randomness from rng::StrandRng.
    pub fn r#gen() -> Result<StrandSignatureSk, StrandError> {
        let mut rng = StrandRng;
        Ok(StrandSignatureSk(SigningKey::random(&mut rng)))
    }
    /// Signs the message returning a signature.
    pub fn sign(&self, msg: &[u8]) -> Result<StrandSignature, StrandError> {
        let mut digest: RustCryptoHasher =
            crate::hashing::rustcrypto::rust_crypto_ecdsa_hasher();
        digest.update(msg);

        let (sig, _) = self.0.sign_digest(digest);

        Ok(StrandSignature(sig))
    }

    pub fn to_der(&self) -> Result<Vec<u8>, StrandError> {
        let d = self
            .0
            .to_pkcs8_der()
            .map_err(|e| StrandError::Generic(e.to_string()))?;

        Ok(d.as_bytes().to_vec())
    }

    pub fn from_der(bytes: &[u8]) -> Result<Self, StrandError> {
        let sk = SigningKey::from_pkcs8_der(&bytes)
            .map_err(|e| StrandError::Generic(e.to_string()))?;

        Ok(StrandSignatureSk(sk))
    }
}

impl PartialEq for StrandSignaturePk {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for StrandSignaturePk {}

impl std::fmt::Debug for StrandSignaturePk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", &self.0)
    }
}

impl BorshSerialize for StrandSignatureSk {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let bytes =
            self.to_der().map_err(|e| Error::new(ErrorKind::Other, e))?;
        bytes.serialize(writer)
    }
}

impl BorshDeserialize for StrandSignatureSk {
    fn deserialize_reader<R: std::io::Read>(
        reader: &mut R,
    ) -> Result<Self, std::io::Error> {
        let bytes = Vec::<u8>::deserialize_reader(reader)?;
        let sk = StrandSignatureSk::from_der(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(sk)
    }

    /*fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let bytes = Vec::<u8>::deserialize(buf)?;
        let sk = StrandSignatureSk::from_der(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(sk)
    }*/
}

impl BorshSerialize for StrandSignaturePk {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let bytes =
            self.to_der().map_err(|e| Error::new(ErrorKind::Other, e))?;
        bytes.serialize(writer)
    }
}

impl BorshDeserialize for StrandSignaturePk {
    fn deserialize_reader<R: std::io::Read>(
        reader: &mut R,
    ) -> Result<Self, std::io::Error> {
        let bytes = Vec::<u8>::deserialize_reader(reader)?;
        let pk = StrandSignaturePk::from_der(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(pk)
    }

    /*fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let bytes = Vec::<u8>::deserialize(buf)?;
        let pk = StrandSignaturePk::from_der(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(pk)
    }*/
}

impl BorshSerialize for StrandSignature {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let s = self.to_der().map_err(|e| Error::new(ErrorKind::Other, e))?;
        s.serialize(writer)
    }
}

impl BorshDeserialize for StrandSignature {
    fn deserialize_reader<R: std::io::Read>(
        reader: &mut R,
    ) -> Result<Self, std::io::Error> {
        let bytes = Vec::<u8>::deserialize_reader(reader)?;
        let signature = StrandSignature::from_der(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(signature)
    }

    /* fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let bytes = Vec::<u8>::deserialize(buf)?;
        let signature = StrandSignature::from_der(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(signature)
    }*/
}

impl TryFrom<String> for StrandSignaturePk {
    type Error = StrandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let bytes: Vec<u8> = general_purpose::STANDARD.decode(value)?;
        let pk = StrandSignaturePk::from_der(&bytes)?;

        Ok(pk)
    }
}

impl TryFrom<StrandSignaturePk> for String {
    type Error = StrandError;

    fn try_from(value: StrandSignaturePk) -> Result<Self, Self::Error> {
        let bytes = value.to_der()?;
        Ok(general_purpose::STANDARD.encode(bytes))
    }
}

impl TryFrom<String> for StrandSignatureSk {
    type Error = StrandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let bytes: Vec<u8> = general_purpose::STANDARD.decode(value)?;
        let sk = StrandSignatureSk::from_der(&bytes)
            .map_err(|e| StrandError::Generic(e.to_string()))?;

        Ok(sk)
    }
}

impl TryFrom<StrandSignatureSk> for String {
    type Error = StrandError;

    fn try_from(value: StrandSignatureSk) -> Result<Self, Self::Error> {
        let bytes = value.to_der()?;
        Ok(general_purpose::STANDARD.encode(bytes))
    }
}

impl TryFrom<String> for StrandSignature {
    type Error = StrandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let bytes: Vec<u8> = general_purpose::STANDARD.decode(value)?;
        let signature = StrandSignature::from_der(&bytes)
            .map_err(|e| StrandError::Generic(e.to_string()))?;

        Ok(signature)
    }
}

impl TryFrom<StrandSignature> for String {
    type Error = StrandError;

    fn try_from(value: StrandSignature) -> Result<Self, Self::Error> {
        let bytes = value.to_der()?;
        Ok(general_purpose::STANDARD.encode(bytes))
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

        let (vk_bytes, sig_bytes) = {
            let sk = StrandSignatureSk::r#gen().unwrap();
            let sk_b = sk.strand_serialize().unwrap();
            let sk_d = StrandSignatureSk::strand_deserialize(&sk_b).unwrap();

            let sig = sk_d.sign(msg).unwrap();

            let sig_bytes = sig.strand_serialize().unwrap();
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

        let (public_key_string, signature_string) = {
            let signing_key = StrandSignatureSk::r#gen().unwrap();
            let signing_key_string: String = signing_key.try_into().unwrap();
            let signing_key_deserialized: StrandSignatureSk =
                signing_key_string.try_into().unwrap();

            let sig = signing_key_deserialized.sign(message).unwrap();

            let signature_string: String = sig.try_into().unwrap();
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
