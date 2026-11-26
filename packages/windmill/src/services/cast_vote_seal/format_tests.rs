// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use hmac::{Hmac, Mac};
use openssl::bn::{BigNum, BigNumContext};
use openssl::derive::Deriver;
use openssl::ec::{EcGroup, EcKey, EcPoint, PointConversionForm};
use openssl::nid::Nid;
use openssl::pkey::{PKey, Private, Public};
use openssl::symm::{decrypt_aead, encrypt_aead, Cipher};
use serde::Deserialize;
use sha2::Sha256;

/// The known-answer vectors other implementations of `hsm-seal-v1` check
/// themselves against.
const VECTORS: &str = include_str!("hsm_seal_v1_vectors.json");

#[derive(Deserialize)]
struct Vector {
    name: String,
    device_private_key: String,
    device_public_key: String,
    ephemeral_private_key: String,
    ephemeral_public_key: String,
    shared_secret: String,
    nonce: String,
    key_id: String,
    key_version: u32,
    aad: String,
    plaintext: String,
    sealed: String,
}

#[derive(Deserialize)]
struct Vectors {
    format: String,
    vectors: Vec<Vector>,
}

fn group() -> EcGroup {
    EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap()
}

/// A P-256 key from its private scalar, and its uncompressed point.
fn key_pair(scalar_hex: &str) -> (PKey<Private>, Vec<u8>) {
    let group = group();
    let mut ctx = BigNumContext::new().unwrap();
    let scalar = BigNum::from_hex_str(scalar_hex).unwrap();
    let mut point = EcPoint::new(&group).unwrap();
    point.mul_generator2(&group, &scalar, &mut ctx).unwrap();
    let bytes = point
        .to_bytes(&group, PointConversionForm::UNCOMPRESSED, &mut ctx)
        .unwrap();
    let key = EcKey::from_private_components(&group, &scalar, &point).unwrap();
    (PKey::from_ec_key(key).unwrap(), bytes)
}

fn public_key(point: &[u8]) -> PKey<Public> {
    let group = group();
    let mut ctx = BigNumContext::new().unwrap();
    let point = EcPoint::from_bytes(&group, point, &mut ctx).unwrap();
    PKey::from_ec_key(EcKey::from_public_key(&group, &point).unwrap()).unwrap()
}

/// ECDH with OpenSSL: the shared point's x coordinate.
fn ecdh(private: &PKey<Private>, point: &[u8]) -> Vec<u8> {
    let peer = public_key(point);
    let mut deriver = Deriver::new(private).unwrap();
    deriver.set_peer(&peer).unwrap();
    deriver.derive_to_vec().unwrap()
}

/// The format as documented, with OpenSSL and RustCrypto's HMAC instead of
/// ring: HKDF-SHA256 with an empty salt (one block of output) and AES-256-GCM.
fn reference_seal(
    ephemeral_point: &[u8],
    shared: &[u8],
    nonce: &[u8],
    key_id: &str,
    version: u32,
    plaintext: &[u8],
    aad: &[u8],
) -> Vec<u8> {
    let mut extract = Hmac::<Sha256>::new_from_slice(&[]).unwrap();
    extract.update(shared);
    let prk = extract.finalize().into_bytes();
    let mut expand = Hmac::<Sha256>::new_from_slice(&prk).unwrap();
    expand.update(b"sequent-hsm-seal-v1");
    expand.update(&(key_id.len() as u16).to_be_bytes());
    expand.update(key_id.as_bytes());
    expand.update(&version.to_be_bytes());
    expand.update(&[1]);
    let key = expand.finalize().into_bytes();
    let mut tag = [0u8; TAG_LEN];
    let ciphertext = encrypt_aead(
        Cipher::aes_256_gcm(),
        &key,
        Some(nonce),
        aad,
        plaintext,
        &mut tag,
    )
    .unwrap();
    let mut sealed = vec![VERSION];
    sealed.extend_from_slice(ephemeral_point);
    sealed.extend_from_slice(nonce);
    sealed.extend(ciphertext);
    sealed.extend_from_slice(&tag);
    // OpenSSL opens it too.
    let opened = decrypt_aead(
        Cipher::aes_256_gcm(),
        &key,
        Some(nonce),
        aad,
        &sealed[HEADER..sealed.len() - TAG_LEN],
        &sealed[sealed.len() - TAG_LEN..],
    )
    .unwrap();
    assert_eq!(opened, plaintext);
    sealed
}

#[test]
fn the_known_answer_vectors_hold_for_ring_and_for_an_openssl_reference() {
    let vectors: Vectors = serde_json::from_str(VECTORS).unwrap();
    assert_eq!(vectors.format, "hsm-seal-v1");
    assert!(!vectors.vectors.is_empty());
    let mut mismatches = Vec::new();
    for vector in &vectors.vectors {
        let hex = |text: &str| hex::decode(text).unwrap();
        let (device, device_point) = key_pair(&vector.device_private_key);
        let (ephemeral, eph_point) = key_pair(&vector.ephemeral_private_key);
        assert_eq!(hex::encode(&device_point), vector.device_public_key);
        assert_eq!(hex::encode(&eph_point), vector.ephemeral_public_key);
        // ECDH agrees from both sides.
        let shared = ecdh(&ephemeral, &device_point);
        assert_eq!(ecdh(&device, &eph_point), shared);
        assert_eq!(
            hex::encode(&shared),
            vector.shared_secret,
            "{}",
            vector.name
        );
        let nonce: [u8; NONCE_LEN] = hex(&vector.nonce).try_into().unwrap();
        let plaintext = hex(&vector.plaintext);
        let aad = hex(&vector.aad);
        let sealed = seal_with_shared(
            &eph_point,
            &shared,
            nonce,
            &vector.key_id,
            vector.key_version,
            &plaintext,
            &aad,
        )
        .unwrap();
        let reference = reference_seal(
            &eph_point,
            &shared,
            &nonce,
            &vector.key_id,
            vector.key_version,
            &plaintext,
            &aad,
        );
        assert_eq!(
            hex::encode(&reference),
            hex::encode(&sealed),
            "{}",
            vector.name
        );
        if hex::encode(&sealed) != vector.sealed {
            mismatches.push(format!("{}: {}", vector.name, hex::encode(&sealed)));
        }
        assert_eq!(ephemeral_point(&sealed).unwrap(), eph_point.as_slice());
        assert_eq!(
            open_with_shared(&shared, &vector.key_id, vector.key_version, &sealed, &aad).unwrap(),
            plaintext
        );
    }
    assert!(
        mismatches.is_empty(),
        "sealed values differ: {mismatches:#?}"
    );
}

#[test]
fn a_sealed_value_opens_only_with_its_device_key_id_version_and_aad() {
    let (device, device_point) =
        key_pair("3c9a1f0e7d2b4c5a6e8f9d0c1b2a39485766a5b4c3d2e1f00f1e2d3c4b5a6978");
    let sealed = seal(&device_point, "cast-vote-seal/a/b", 1, b"ballot", b"row").unwrap();
    assert_eq!(sealed.len(), HEADER + 6 + TAG_LEN);
    let shared = ecdh(&device, ephemeral_point(&sealed).unwrap());
    assert_eq!(
        open_with_shared(&shared, "cast-vote-seal/a/b", 1, &sealed, b"row").unwrap(),
        b"ballot"
    );
    // Sealing twice gives different values: fresh ephemeral key and nonce.
    let again = seal(&device_point, "cast-vote-seal/a/b", 1, b"ballot", b"row").unwrap();
    assert_ne!(again, sealed);
    for (key_id, version, aad) in [
        ("cast-vote-seal/a/c", 1, b"row".as_slice()),
        ("cast-vote-seal/a/b", 2, b"row".as_slice()),
        ("cast-vote-seal/a/b", 1, b"other".as_slice()),
    ] {
        assert!(open_with_shared(&shared, key_id, version, &sealed, aad).is_err());
    }
    let mut tampered = sealed.clone();
    *tampered.last_mut().unwrap() ^= 1;
    assert!(open_with_shared(&shared, "cast-vote-seal/a/b", 1, &tampered, b"row").is_err());
    let mut wrong_version = sealed.clone();
    wrong_version[0] = 2;
    assert!(ephemeral_point(&wrong_version).is_err());
    assert!(ephemeral_point(&sealed[..HEADER + TAG_LEN - 1]).is_err());
    // Another device's shared secret does not open it.
    let (other, _) = key_pair("11223344556677889900aabbccddeeff00112233445566778899aabbccddeeff");
    let wrong = ecdh(&other, ephemeral_point(&sealed).unwrap());
    assert!(open_with_shared(&wrong, "cast-vote-seal/a/b", 1, &sealed, b"row").is_err());
}

#[test]
fn only_uncompressed_points_and_short_key_ids_are_sealed_to() {
    let (_, point) = key_pair("3c9a1f0e7d2b4c5a6e8f9d0c1b2a39485766a5b4c3d2e1f00f1e2d3c4b5a6978");
    let mut compressed = point.clone();
    compressed[0] = 0x02;
    assert!(seal(&compressed, "k", 1, b"x", b"").is_err());
    assert!(seal(&point[..64], "k", 1, b"x", b"").is_err());
    // A point not on the curve.
    let mut off_curve = point.clone();
    off_curve[64] ^= 1;
    assert!(seal(&off_curve, "k", 1, b"x", b"").is_err());
    let long = "k".repeat(usize::from(u16::MAX) + 1);
    assert!(seal(&point, &long, 1, b"x", b"").is_err());
}
