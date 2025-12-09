// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-License-Identifier: MIT

use base64::{engine::general_purpose, Engine as _};
use borsh::{BorshDeserialize, BorshSerialize};
use openssl::ec::{EcGroup, EcKey};
use openssl::ecdsa::EcdsaSig;
use openssl::nid::Nid;
use openssl::pkey::{Private, Public};

use std::hash::Hash;
use std::hash::Hasher;
use std::io::{Error, ErrorKind};

use crate::hash::RustCryptoHasher;
use crate::util::StrandError;

const CURVE: Nid = Nid::SECP384R1;

/// An openssl ecdsa backed signature.
// #[derive(Clone)]
pub struct StrandSignature(EcdsaSig);

impl StrandSignature {
    pub fn try_clone(&self) -> Result<Self, StrandError> {
        let r = self.0.r().to_owned()?;
        let s = self.0.s().to_owned()?;

        let sig = EcdsaSig::from_private_components(r, s);

        Ok(StrandSignature(sig?))
    }

    pub fn to_der(&self) -> Result<Vec<u8>, StrandError> {
        Ok(self.0.to_der()?)
    }

    pub fn from_der(bytes: &[u8]) -> Result<Self, StrandError> {
        let signature = EcdsaSig::from_der(bytes)?;

        Ok(StrandSignature(signature))
    }
}

/// An openssl ecdsa signature verification key.
// Clone: Allows Configuration to be Clonable in Braid
// We keep the original bytes in the struct to be able to implement Hash and Eq
// Hash and Eq are required by braid to detect duplicate keys
#[derive(Clone)]
pub struct StrandSignaturePk(EcKey<Public>, Vec<u8>);
impl StrandSignaturePk {
    /// Returns the verification key from this signing key.
    pub fn from(
        sk: &StrandSignatureSk,
    ) -> Result<StrandSignaturePk, StrandError> {
        let bytes = sk.0.public_key_to_der()?;

        let pk = EcKey::<Public>::public_key_from_der(&bytes)?;
        Ok(StrandSignaturePk(pk, bytes))
    }
    /// Verifies the signature given the message. Returns Ok(()) if the
    /// verification passes.
    pub fn verify(
        &self,
        signature: &StrandSignature,
        msg: &[u8],
    ) -> Result<(), StrandError> {
        // Compatibility with sig::rustcrypto
        let mut digest: RustCryptoHasher =
            crate::hash::rust_crypto_ecdsa_hasher()?;
        let _ = digest.update(msg)?;
        let hashed = digest.finish()?;

        let result = signature.0.verify(&hashed, &self.0)?;
        if !result {
            Err(StrandError::Generic(
                "OpenSSL signature failed to verify".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    /// Calls the underlying [check_key](https://docs.rs/openssl/latest/openssl/ec/struct.EcKeyRef.html#method.check_key)
    pub fn check_key(&self) -> Result<(), StrandError> {
        Ok(self.0.check_key()?)
    }

    pub fn to_der(&self) -> Result<Vec<u8>, StrandError> {
        Ok(self.0.public_key_to_der()?)
    }

    pub fn from_der(bytes: &[u8]) -> Result<Self, StrandError> {
        let pk = EcKey::<Public>::public_key_from_der(bytes)?;

        Ok(StrandSignaturePk(pk, bytes.to_vec()))
    }
}

/// An openssl ecdsa signing key.
#[derive(Clone)]
pub struct StrandSignatureSk(EcKey<Private>);
impl StrandSignatureSk {
    /// Generates a key using randomness from rng::StrandRng.
    pub fn gen() -> Result<StrandSignatureSk, StrandError> {
        let group = EcGroup::from_curve_name(CURVE)?;
        let key = EcKey::<Private>::generate(&group)?;

        Ok(StrandSignatureSk(key))
    }
    /// Signs the message returning a signature.
    pub fn sign(&self, msg: &[u8]) -> Result<StrandSignature, StrandError> {
        // Compatibility with sig::rustcrypto
        let mut digest: RustCryptoHasher =
            crate::hash::rust_crypto_ecdsa_hasher()?;
        let _ = digest.update(msg)?;
        let hashed = digest.finish()?;

        let sig = EcdsaSig::sign(&hashed, &self.0)?;

        Ok(StrandSignature(sig))
    }

    pub fn to_der(&self) -> Result<Vec<u8>, StrandError> {
        Ok(self.0.private_key_to_der()?)
    }

    pub fn from_der(bytes: &[u8]) -> Result<Self, StrandError> {
        let sk = EcKey::<Private>::private_key_from_der(bytes)?;

        Ok(StrandSignatureSk(sk))
    }
}

impl PartialEq for StrandSignaturePk {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}
impl Eq for StrandSignaturePk {}

impl Hash for StrandSignaturePk {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.1.hash(state);
    }
}
impl std::fmt::Debug for StrandSignaturePk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &hex::encode(&self.1)[0..10])
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
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let bytes = Vec::<u8>::deserialize(buf)?;
        let sk = StrandSignatureSk::from_der(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(sk)
    }
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
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let bytes = Vec::<u8>::deserialize(buf)?;
        let pk = StrandSignaturePk::from_der(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(pk)
    }
}

impl BorshSerialize for StrandSignature {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let bytes =
            self.to_der().map_err(|e| Error::new(ErrorKind::Other, e))?;
        bytes.serialize(writer)
    }
}

impl BorshDeserialize for StrandSignature {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let bytes = Vec::<u8>::deserialize(buf)?;
        let signature = StrandSignature::from_der(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(signature)
    }
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
        let sk = StrandSignatureSk::from_der(&bytes)?;

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
        let signature = StrandSignature::from_der(&bytes)?;

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
            let sk = StrandSignatureSk::gen().unwrap();
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
            let signing_key = StrandSignatureSk::gen().unwrap();
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
    format!("{}, FIPS_ENABLED: TRUE", module_path!())
}
