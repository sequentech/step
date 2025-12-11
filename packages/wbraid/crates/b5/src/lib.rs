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
use cryptography::utils::hash::Hasher as HasherTrait;

#[cfg(feature = "native")]
use std::time::{SystemTime, UNIX_EPOCH};

/// The Hasher instance as defined by the cryptography library.
pub type Hasher = cryptography::context::CryptographicHasher;

/// The Hash output type as defined by the cryptography library.
pub type CryptographicHash = sha3::digest::Output<Hasher>;

#[cfg(feature = "native")]
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

/// Hash bytes to produce a CryptographicHash
pub fn hash_bytes(bytes: &[u8]) -> CryptographicHash {
    use sha3::Digest;
    let mut hasher = Hasher::hasher();
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

// Re-export HTTP message types for convenience
pub use messages::http_message::{HttpB5Message, HttpBoardMessages};
