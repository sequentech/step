// SPDX-FileCopyrightText: 2023 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-License-Identifier: MIT

use base64::{engine::general_purpose, Engine as _};
use ed25519_dalek::Signature;
use ed25519_dalek::Signer;
use ed25519_dalek::SigningKey;
use ed25519_dalek::Verifier;
use ed25519_dalek::VerifyingKey;
use ed25519_dalek::pkcs8::EncodePublicKey;
use ed25519_dalek::pkcs8::DecodePublicKey;
use ed25519_dalek::pkcs8::EncodePrivateKey;
use ed25519_dalek::pkcs8::DecodePrivateKey;
use std::hash::Hash;
use std::hash::Hasher;
use std::io::{Error, ErrorKind};

use crate::rng::StrandRng;
use crate::serialization::{StrandDeserialize, StrandSerialize};
use crate::util;
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

     pub fn to_bytes(&self) -> [u8; 64] {
        self.0.to_bytes()
     }

     pub fn from_bytes(bytes: [u8; 64]) -> Result<StrandSignature, StrandError> {
        let signature = Signature::try_from(bytes)
            .map_err(|e| StrandError::Generic(e.to_string()))?;

        Ok(StrandSignature(signature))
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

    pub fn to_der(&self) -> Result<Vec<u8>, StrandError> {
        let doc = self.0.to_public_key_der()
            .map_err(|e| StrandError::Generic(e.to_string()))?;
        
        Ok(doc.as_bytes().to_vec())
    }

    pub fn from_der(bytes: &[u8]) -> Result<StrandSignaturePk, StrandError> {
        let sk = VerifyingKey::from_public_key_der(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(StrandSignaturePk(sk))
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

    pub fn to_der(&self) -> Result<Vec<u8>, StrandError> {
        let doc = self.0.to_pkcs8_der()
            .map_err(|e| StrandError::Generic(e.to_string()))?;
        
        Ok(doc.as_bytes().to_vec())
    }

    pub fn from_der(bytes: &[u8]) -> Result<StrandSignatureSk, StrandError> {
        let sk = SigningKey::from_pkcs8_der(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(StrandSignatureSk(sk))
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


impl StrandSerialize for StrandSignatureSk {
    fn strand_serialize(&self) -> Result<Vec<u8>, StrandError> {
        let doc = self.0.to_pkcs8_der()
            .map_err(|e| StrandError::Generic(e.to_string()))?;
        
        Ok(doc.as_bytes().to_vec())
    }
}

impl StrandDeserialize for StrandSignatureSk {
    fn strand_deserialize(bytes: &[u8]) -> Result<Self, StrandError> {
        let sk = SigningKey::from_pkcs8_der(&bytes)
        .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(StrandSignatureSk(sk))
    }
}


impl StrandSerialize for StrandSignaturePk {
    fn strand_serialize(&self) -> Result<Vec<u8>, StrandError> {
        let doc = self.0.to_public_key_der()
            .map_err(|e| StrandError::Generic(e.to_string()))?;
        
        Ok(doc.as_bytes().to_vec())
    }
}

impl StrandDeserialize for StrandSignaturePk {
    fn strand_deserialize(bytes: &[u8]) -> Result<Self, StrandError> {
        let sk = VerifyingKey::from_public_key_der(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(StrandSignaturePk(sk))
    }
}

impl StrandSerialize for StrandSignature {
    fn strand_serialize(&self) -> Result<Vec<u8>, StrandError> {
        Ok(self.0.to_bytes().to_vec())
    }
}

impl StrandDeserialize for StrandSignature {
    fn strand_deserialize(bytes: &[u8]) -> Result<Self, StrandError> {
        let signature = Signature::try_from(bytes)
            .map_err(|e| StrandError::Generic(e.to_string()))?;

        Ok(StrandSignature(signature))
    }    
}

impl TryFrom<String> for StrandSignaturePk {
    type Error = StrandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let bytes: Vec<u8> = general_purpose::STANDARD.decode(value)?;
        // StrandSignaturePk::strand_deserialize(&bytes)
        StrandSignaturePk::from_der(&bytes)
    }
}

impl TryFrom<StrandSignaturePk> for String {
    type Error = StrandError;

    fn try_from(value: StrandSignaturePk) -> Result<Self, Self::Error> {
        // let bytes = value.strand_serialize()?;
        let bytes = value.to_der()?;
        Ok(general_purpose::STANDARD.encode(bytes))
    }
}

impl TryFrom<String> for StrandSignatureSk {
    type Error = StrandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let bytes: Vec<u8> = general_purpose::STANDARD.decode(value)?;
        StrandSignatureSk::from_der(&bytes)
        // StrandSignatureSk::strand_deserialize(&bytes)
    }
}

impl TryFrom<StrandSignatureSk> for String {
    type Error = StrandError;

    fn try_from(value: StrandSignatureSk) -> Result<Self, Self::Error> {
        // let bytes = value.strand_serialize()?;
        let bytes = value.to_der()?;
        Ok(general_purpose::STANDARD.encode(bytes))
    }
}

impl TryFrom<String> for StrandSignature {
    type Error = StrandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let bytes: Vec<u8> = general_purpose::STANDARD.decode(value)?;
        // StrandSignature::strand_deserialize(&bytes)
        let bytes = util::to_u8_array(&bytes)?;
        StrandSignature::from_bytes(bytes)
    }
}

impl TryFrom<StrandSignature> for String {
    type Error = StrandError;

    fn try_from(value: StrandSignature) -> Result<Self, Self::Error> {
        // let bytes = value.strand_serialize()?;
        let bytes = value.to_bytes();
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

    #[test]
    fn test_openssl_compat() {
        let message = b"ok\n";
        /* 
        openssl genpkey -algorithm ed25519 -outform DER -out test25519.der
        openssl base64 -in test25519.der -out test25519.b64
        openssl pkey -in test25519.der -outform der -pubout -out pk.der
        openssl base64 -in pk.der -out pk.b64
        */
        let secret_key_string = "MC4CAQAwBQYDK2VwBCIEII6bMx4lMnY83pVId7YbeOYGHoSZAnP7KjR/WsjaXkc9".to_string();
        let public_key_string = "MCowBQYDK2VwAyEApnH8A4iAauMx0tZOx9JrpnG37adrUPiXg5klJ7fZRLU=".to_string();
        
        /* 
        data.txt contained one line with "ok" 
        openssl pkeyutl -sign -out data.txt.signature -in data.txt -inkey test25519.der -rawin
        base64 data.txt.signature data.txt.signature.b64

        the signature can be verified with 
        openssl pkeyutl -verify -pubin -inkey pk.der -rawin -in data.txt -sigfile data.txt.signature
        */
        let signature_string = "nMJ6twxCU1fogkNNNsmvdlTsdeiYn5SnDrjF0Jy5zURG/Z0ZdSY3JIj7Z2pQ4ANHMTBXzRDF60AtQ8EW7WQQBQ==".to_string();

        let secret_key: StrandSignatureSk =
            secret_key_string.try_into().unwrap();
        
        let public_key: StrandSignaturePk =
            public_key_string.try_into().unwrap();

        let sig = secret_key.sign(message).unwrap();
        let ok = public_key.verify(&sig, message);

        assert!(ok.is_ok());
        
        let signature: StrandSignature = signature_string.try_into().unwrap();

        let ok = public_key.verify(&signature, message);
        
        assert!(ok.is_ok());

    }
}

pub fn info() -> String {
    format!("{}, FIPS_ENABLED: FALSE", module_path!())
}