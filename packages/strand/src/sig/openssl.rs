// SPDX-FileCopyrightText: 2023 David Ruescas <david@sequentech.io>
// SPDX-FileCopyrightText: 2020 Zcash Foundation
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

use crate::serialization::{StrandDeserialize, StrandSerialize};
use crate::util::Digest;
use crate::util::RustCryptoHasher;
use crate::util::StrandError;

const CURVE: Nid = Nid::SECP384R1;

/// An openssl ecdsa backed signature.
// #[derive(Clone)]
pub struct StrandSignature(EcdsaSig);

/// An openssl ecdsa signature verification key.
// Clone: Allows Configuration to be Clonable in Braid
// #[derive(Clone)]
pub struct StrandSignaturePk(EcKey<Public>, Vec<u8>);
impl StrandSignaturePk {
    pub fn from(
        sk: &StrandSignatureSk,
    ) -> Result<StrandSignaturePk, StrandError> {
        let bytes = sk.0.public_key_to_der()?;

        let pk = EcKey::<Public>::public_key_from_der(&bytes)?;
        Ok(StrandSignaturePk(pk, bytes))
    }
    pub fn verify(
        &self,
        signature: &StrandSignature,
        msg: &[u8],
    ) -> Result<(), StrandError> {
        // Compatibility with sig::rustcrypto
        let mut digest: RustCryptoHasher =
            crate::util::rust_crypto_ecdsa_hasher();
        digest.update(msg);
        let hashed = digest.finalize();

        let result = signature.0.verify(&hashed, &self.0)?;
        if !result {
            Err(StrandError::Generic(
                "OpenSSL signature failed to verify".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    pub fn check_key(&self) -> Result<(), StrandError> {
        Ok(self.0.check_key()?)
    }
}

/// An openssl ecdsa signing key.
// #[derive(Clone)]
pub struct StrandSignatureSk(EcKey<Private>);
impl StrandSignatureSk {
    pub fn new() -> Result<StrandSignatureSk, StrandError> {
        let group = EcGroup::from_curve_name(CURVE)?;
        let key = EcKey::<Private>::generate(&group)?;

        Ok(StrandSignatureSk(key))
    }
    pub fn sign(&self, msg: &[u8]) -> Result<StrandSignature, StrandError> {
        // Compatibility with sig::rustcrypto
        let mut digest: RustCryptoHasher =
            crate::util::rust_crypto_ecdsa_hasher();
        digest.update(msg);
        let hashed = digest.finalize();

        let sig = EcdsaSig::sign(&hashed, &self.0)?;

        Ok(StrandSignature(sig))
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
        let bytes = self.0.private_key_to_der()?;
        bytes.serialize(writer)
    }
}

impl BorshDeserialize for StrandSignatureSk {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let bytes = Vec::<u8>::deserialize(buf)?;
        let sk = EcKey::<Private>::private_key_from_der(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(StrandSignatureSk(sk))
    }
}

impl BorshSerialize for StrandSignaturePk {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let bytes = self.0.public_key_to_der()?;
        bytes.serialize(writer)
    }
}

impl BorshDeserialize for StrandSignaturePk {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let bytes = Vec::<u8>::deserialize(buf)?;
        let pk = EcKey::<Public>::public_key_from_der(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(StrandSignaturePk(pk, bytes))
    }
}

impl BorshSerialize for StrandSignature {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let bytes = self.0.to_der()?;
        bytes.serialize(writer)
    }
}

impl BorshDeserialize for StrandSignature {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let bytes = Vec::<u8>::deserialize(buf)?;
        let signature = EcdsaSig::from_der(&bytes)
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

        let (vk_bytes, sig_bytes) = {
            let sk = StrandSignatureSk::new().unwrap();
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
            let signing_key = StrandSignatureSk::new().unwrap();
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
