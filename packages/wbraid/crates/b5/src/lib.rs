// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod messages;
pub mod api_types;

// Native-only modules
#[cfg(feature = "native")]
pub mod db;
#[cfg(feature = "native")]
pub mod handlers;
#[cfg(feature = "native")]
pub mod s3;
#[cfg(feature = "native")]
pub mod state;

use anyhow::Result;

use crate::messages::newtypes::Timestamp;

use cryptography::context::{Context, RistrettoCtx};

/// The concrete cryptographic context used throughout B5.
/// This fixes SHA3-512 (64-byte hashes), Ristretto255, Ed25519, and ChaCha20Poly1305.
pub type CryptographicContext = RistrettoCtx;

/// Type aliases for cryptographic primitives (Ed25519 signatures)
/// These extract the concrete types from RistrettoCtx
pub type Signature = <<RistrettoCtx as Context>::SignatureScheme as cryptography::utils::signatures::SignatureScheme<<RistrettoCtx as Context>::Rng>>::Signature;
pub type VerifyingKey = <<RistrettoCtx as Context>::SignatureScheme as cryptography::utils::signatures::SignatureScheme<<RistrettoCtx as Context>::Rng>>::Verifier;
pub type SigningKey = <<RistrettoCtx as Context>::SignatureScheme as cryptography::utils::signatures::SignatureScheme<<RistrettoCtx as Context>::Rng>>::Signer;

/// The Hasher instance as defined by the cryptography context.
pub type Hasher = <CryptographicContext as cryptography::context::Context>::Hasher;

/// The Hash output type as defined by the cryptography context.
pub type CryptographicHash = sha3::digest::Output<Hasher>;

#[cfg(not(target_arch = "wasm32"))]
pub fn timestamp() -> Timestamp {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Impossible with respect to UNIX_EPOCH");

    since_the_epoch.as_secs()
}

#[cfg(target_arch = "wasm32")]
pub fn timestamp() -> Timestamp {
    // Use JavaScript Date.now() for WASM (returns milliseconds since epoch)
    (js_sys::Date::now() / 1000.0) as u64
}

pub fn get_schema_version() -> String {
    "1".to_string()
}

/// Generate a new signing key using the cryptographic context's RNG
pub fn generate_signing_key() -> SigningKey {
    CryptographicContext::gen_signing_key()
}

/// Hash bytes to produce a CryptographicHash
pub fn hash_bytes(bytes: &[u8]) -> CryptographicHash {
    use sha3::Digest;
    let mut hasher = CryptographicContext::get_hasher();
    hasher.update(bytes);
    hasher.finalize()
}

/// Hash bytes and return CryptographicHash
pub fn hash_to_array(bytes: &[u8]) -> Result<CryptographicHash> {
    Ok(hash_bytes(bytes))
}

/// Generate n random ciphertexts for testing
/// This creates dummy ciphertexts with random group elements
pub fn random_ciphertexts<C: cryptography::context::Context, const W: usize>(n: usize) -> Vec<cryptography::cryptosystem::elgamal::Ciphertext<C, W>> {
    use cryptography::traits::groups::GroupElement;
    (0..n)
        .map(|_| {
            let mut rng = C::get_rng();
            let u: [C::Element; W] = <[C::Element; W]>::random(&mut rng);
            let v: [C::Element; W] = <[C::Element; W]>::random(&mut rng);
            cryptography::cryptosystem::elgamal::Ciphertext([u, v])
        })
        .collect()
}

/// Generate an ElGamal keypair with a Schnorr proof of knowledge
/// Generate an ElGamal keypair with a Schnorr proof of knowledge
pub fn gen_elgamal_keypair_with_proof<C: cryptography::context::Context>(
    label: &[u8],
) -> Result<(
    cryptography::cryptosystem::elgamal::KeyPair<C>,
    cryptography::zkp::schnorr::SchnorrProof<C>,
), String> {
    
    let keypair = cryptography::cryptosystem::elgamal::KeyPair::<C>::generate();
    let g = C::generator();
    let y = &keypair.pkey.y;
    let proof = cryptography::zkp::schnorr::SchnorrProof::prove(&g, y, &keypair.skey, label)
        .map_err(|e| format!("Failed to generate schnorr proof: {:?}", e))?;
    
    Ok((keypair, proof))
}

/// Serialize a VerifyingKey to bytes and encode as base64 string
pub fn verifying_key_to_der_b64_string(vk: &VerifyingKey) -> Result<String, String> {
    use base64::{engine::general_purpose, Engine as _};
    
    // Ed25519 public keys are 32 bytes
    let bytes = vk.to_bytes();
    Ok(general_purpose::STANDARD.encode(&bytes))
}

/// Deserialize a VerifyingKey from a base64-encoded byte string
pub fn verifying_key_from_der_b64_string(s: &str) -> Result<VerifyingKey, String> {
    use base64::{engine::general_purpose, Engine as _};
    
    let bytes = general_purpose::STANDARD.decode(s)
        .map_err(|e| format!("Failed to decode base64: {:?}", e))?;
    let bytes_array: [u8; 32] = bytes.try_into()
        .map_err(|_| "Invalid key length: expected 32 bytes".to_string())?;
    VerifyingKey::from_bytes(&bytes_array)
        .map_err(|e| format!("Failed to decode public key: {:?}", e))
}

/// Serialize a SigningKey to bytes and encode as base64 string
pub fn signing_key_to_der_b64_string(sk: &SigningKey) -> Result<String, String> {
    use base64::{engine::general_purpose, Engine as _};
    
    // Ed25519 secret keys are 32 bytes
    let bytes = sk.to_bytes();
    Ok(general_purpose::STANDARD.encode(&bytes))
}

/// Deserialize a SigningKey from a base64-encoded byte string  
pub fn signing_key_from_der_b64_string(s: &str) -> Result<SigningKey, String> {
    use base64::{engine::general_purpose, Engine as _};
    
    let bytes = general_purpose::STANDARD.decode(s)
        .map_err(|e| format!("Failed to decode base64: {:?}", e))?;
    let bytes_array: [u8; 32] = bytes.try_into()
        .map_err(|_| "Invalid key length: expected 32 bytes".to_string())?;
    Ok(SigningKey::from_bytes(&bytes_array))
}

// Re-export HTTP message types for convenience
pub use messages::http_message::{HttpB3Message, HttpBoardMessages};
