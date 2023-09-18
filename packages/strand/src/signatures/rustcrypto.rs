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

use crate::hashing::sha2::Digest;
use crate::hashing::sha2::RustCryptoHasher;
use crate::rng::StrandRng;
use crate::serialization::{StrandDeserialize, StrandSerialize};
use crate::util::StrandError;

type Curve = NistP384;

/// An rustcrypto ecdsa backed signature.
#[derive(Clone)]
pub struct StrandSignature(Signature<Curve>);

/// An rustcrypto ecdsa signature verification key.
// Clone: Allows Configuration to be Clonable in Braid
#[derive(Clone)]
pub struct StrandSignaturePk(VerifyingKey<Curve>);
impl StrandSignaturePk {
    pub fn from(
        sk: &StrandSignatureSk,
    ) -> Result<StrandSignaturePk, StrandError> {
        Ok(StrandSignaturePk(VerifyingKey::from(&sk.0)))
    }
    pub fn verify(
        &self,
        signature: &StrandSignature,
        msg: &[u8],
    ) -> Result<(), StrandError> {
        let mut digest: RustCryptoHasher =
            crate::hashing::sha2::rust_crypto_ecdsa_hasher();
        digest.update(msg);

        Ok(self.0.verify_digest(digest, &signature.0)?)
    }
}

/// An rustcrypto ecdsa signing key.
// #[derive(Clone)]
pub struct StrandSignatureSk(SigningKey<Curve>);
impl StrandSignatureSk {
    pub fn new(rng: &mut StrandRng) -> Result<StrandSignatureSk, StrandError> {
        Ok(StrandSignatureSk(SigningKey::random(rng)))
    }
    pub fn sign(&self, msg: &[u8]) -> Result<StrandSignature, StrandError> {
        let mut digest: RustCryptoHasher =
            crate::hashing::sha2::rust_crypto_ecdsa_hasher();
        digest.update(msg);

        let (sig, _) = self.0.sign_digest(digest);

        Ok(StrandSignature(sig))
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
        let sd = self
            .0
            .to_pkcs8_der()
            .map_err(|e| Error::new(ErrorKind::Other, e))?;
        sd.as_bytes().serialize(writer)
    }
}

impl BorshDeserialize for StrandSignatureSk {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let bytes = Vec::<u8>::deserialize(buf)?;
        let sk = SigningKey::from_pkcs8_der(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(StrandSignatureSk(sk))
    }
}

impl BorshSerialize for StrandSignaturePk {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let d = self
            .0
            .to_public_key_der()
            .map_err(|e| Error::new(ErrorKind::Other, e))?;
        d.as_bytes().serialize(writer)
    }
}

impl BorshDeserialize for StrandSignaturePk {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let bytes = Vec::<u8>::deserialize(buf)?;
        let pk = VerifyingKey::from_public_key_der(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(StrandSignaturePk(pk))
    }
}

impl BorshSerialize for StrandSignature {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let s = self.0.to_der();
        s.as_bytes().serialize(writer)
    }
}

impl BorshDeserialize for StrandSignature {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let bytes = Vec::<u8>::deserialize(buf)?;
        let signature = Signature::from_der(&bytes)
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
            let sk = StrandSignatureSk::new(&mut rng).unwrap();
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
        let mut rng = StrandRng;

        let (public_key_string, signature_string) = {
            let signing_key = StrandSignatureSk::new(&mut rng).unwrap();
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
