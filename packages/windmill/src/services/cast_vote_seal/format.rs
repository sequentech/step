// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! An independent implementation of the documented `hsm-seal-v1` format,
//! which an HSM service opens with its device's ECDH:
//!
//! ```text
//! 0x01 || ephemeral P-256 point (65, uncompressed) || nonce (12) || AES-256-GCM ciphertext and tag
//! key = HKDF-SHA256(salt = "", ikm = ECDH(ephemeral, device key),
//!                   info = "sequent-hsm-seal-v1" || u16be(len(key id)) || key id || u32be(version))
//! ```
//!
//! The caller's associated data is the GCM associated data. Sealing needs
//! only the device key's public point, so it runs here in software; opening
//! needs the device's ECDH, which only the installation's opener has.

use anyhow::{anyhow, ensure, Result};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::agreement::{agree_ephemeral, EphemeralPrivateKey, UnparsedPublicKey, ECDH_P256};
use ring::hkdf::{Salt, HKDF_SHA256};
use ring::rand::{SecureRandom, SystemRandom};

/// The first byte of every `hsm-seal-v1` value.
pub const VERSION: u8 = 1;
/// An uncompressed P-256 point.
pub const POINT_LEN: usize = 65;
pub const NONCE_LEN: usize = 12;
pub const TAG_LEN: usize = 16;
const HEADER: usize = 1 + POINT_LEN + NONCE_LEN;
const INFO_PREFIX: &[u8] = b"sequent-hsm-seal-v1";

fn info(key_id: &str, version: u32) -> Result<Vec<u8>> {
    let length = u16::try_from(key_id.len()).map_err(|_| anyhow!("The key id is too long"))?;
    let mut info = INFO_PREFIX.to_vec();
    info.extend_from_slice(&length.to_be_bytes());
    info.extend_from_slice(key_id.as_bytes());
    info.extend_from_slice(&version.to_be_bytes());
    Ok(info)
}

fn derive(shared: &[u8], key_id: &str, version: u32) -> Result<LessSafeKey> {
    let info = info(key_id, version)?;
    let parts = [info.as_slice()];
    let prk = Salt::new(HKDF_SHA256, &[]).extract(shared);
    let okm = prk
        .expand(&parts, &AES_256_GCM)
        .map_err(|_| anyhow!("Deriving the sealing key failed"))?;
    let key = UnboundKey::from(okm);
    Ok(LessSafeKey::new(key))
}

fn check_point(point: &[u8]) -> Result<()> {
    ensure!(
        point.len() == POINT_LEN && point[0] == 0x04,
        "The key is not an uncompressed P-256 point"
    );
    Ok(())
}

/// Seals `plaintext` to the device key whose public point is `recipient`,
/// under a fresh ephemeral key and nonce.
pub fn seal(
    recipient: &[u8],
    key_id: &str,
    version: u32,
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>> {
    check_point(recipient)?;
    let rng = SystemRandom::new();
    let ephemeral = EphemeralPrivateKey::generate(&ECDH_P256, &rng)
        .map_err(|_| anyhow!("Generating the ephemeral key failed"))?;
    let public = ephemeral
        .compute_public_key()
        .map_err(|_| anyhow!("Computing the ephemeral point failed"))?;
    let mut nonce = [0u8; NONCE_LEN];
    rng.fill(&mut nonce)
        .map_err(|_| anyhow!("Generating the nonce failed"))?;
    let peer = UnparsedPublicKey::new(&ECDH_P256, recipient);
    agree_ephemeral(ephemeral, &peer, |shared| {
        seal_with_shared(
            public.as_ref(),
            shared,
            nonce,
            key_id,
            version,
            plaintext,
            aad,
        )
    })
    .map_err(|_| anyhow!("The key is not a valid P-256 point"))?
}

/// The deterministic part of [`seal`], given the ephemeral point, the ECDH
/// shared secret and the nonce. For known-answer vectors only: production
/// code seals with [`seal`], which never reuses an ephemeral key or nonce.
pub fn seal_with_shared(
    ephemeral_point: &[u8],
    shared: &[u8],
    nonce: [u8; NONCE_LEN],
    key_id: &str,
    version: u32,
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>> {
    check_point(ephemeral_point)?;
    let key = derive(shared, key_id, version)?;
    let mut body = plaintext.to_vec();
    key.seal_in_place_append_tag(
        Nonce::assume_unique_for_key(nonce),
        Aad::from(aad),
        &mut body,
    )
    .map_err(|_| anyhow!("Sealing failed"))?;
    let mut sealed = Vec::with_capacity(HEADER + body.len());
    sealed.push(VERSION);
    sealed.extend_from_slice(ephemeral_point);
    sealed.extend_from_slice(&nonce);
    sealed.extend(body);
    Ok(sealed)
}

/// The ephemeral point a sealed value carries, for the device's ECDH.
pub fn ephemeral_point(sealed: &[u8]) -> Result<&[u8]> {
    ensure!(
        sealed.len() >= HEADER + TAG_LEN && sealed[0] == VERSION,
        "The sealed value is malformed"
    );
    Ok(&sealed[1..1 + POINT_LEN])
}

/// Opens with the shared secret the device computed for [`ephemeral_point`].
pub fn open_with_shared(
    shared: &[u8],
    key_id: &str,
    version: u32,
    sealed: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>> {
    ephemeral_point(sealed)?;
    let key = derive(shared, key_id, version)?;
    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&sealed[1 + POINT_LEN..HEADER]);
    let mut body = sealed[HEADER..].to_vec();
    let length = key
        .open_in_place(
            Nonce::assume_unique_for_key(nonce),
            Aad::from(aad),
            &mut body,
        )
        .map_err(|_| anyhow!("The sealed value does not open with this key and associated data"))?
        .len();
    body.truncate(length);
    Ok(body)
}

#[cfg(test)]
#[path = "format_tests.rs"]
mod tests;
