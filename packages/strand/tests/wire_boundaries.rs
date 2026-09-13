// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Independent vectors and malformed wire inputs complement the existing
//! randomized protocol tests. No production keys or services are involved.

use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use strand::hash::{self, HashWrapper};
use strand::serialization::{StrandDeserialize, StrandSerialize, StrandVector};
use strand::shuffler_product::StrandRectangle;
use strand::signature::{
    StrandSignature, StrandSignaturePk, StrandSignatureSk,
};
use strand::symm;

const PUBLIC_KEY_HEX: &str =
    "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
const SIGNATURE_HEX: &str = concat!(
    "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155",
    "5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
);

fn reference_key() -> StrandSignaturePk {
    StrandSignaturePk::from_bytes(
        hex::decode(PUBLIC_KEY_HEX).unwrap().try_into().unwrap(),
    )
    .unwrap()
}

#[test]
fn ed25519_verifies_the_rfc8032_empty_message_vector() {
    // RFC 8032, section 7.1, TEST 1. The expected signature is independent of
    // Strand's signer: https://www.rfc-editor.org/rfc/rfc8032.txt
    let public_key = reference_key();
    let signature = StrandSignature::from_bytes(
        hex::decode(SIGNATURE_HEX).unwrap().try_into().unwrap(),
    )
    .unwrap();
    public_key.verify(&signature, b"").unwrap();
    assert!(public_key.verify(&signature, b"changed message").is_err());
    assert_eq!(hex::encode(signature.to_bytes()), SIGNATURE_HEX);
    assert_eq!(
        signature.try_clone().unwrap().to_bytes(),
        signature.to_bytes()
    );
}

#[test]
fn public_keys_round_trip_through_their_documented_hex_json_format() {
    let public_key = reference_key();
    let json = serde_json::to_string(&public_key).unwrap();
    assert_eq!(json, format!("\"{PUBLIC_KEY_HEX}\""));
    let restored: StrandSignaturePk = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, public_key);
}

#[test]
fn public_key_json_rejects_bad_types_nonhex_and_wrong_byte_lengths() {
    for malformed in [
        serde_json::json!(null),
        serde_json::json!([]),
        serde_json::json!(1),
        serde_json::json!("zz"),
    ] {
        assert!(serde_json::from_value::<StrandSignaturePk>(malformed).is_err());
    }
    for bytes in [0, 1, 31, 33, 64] {
        let malformed = serde_json::json!("00".repeat(bytes));
        assert!(
            serde_json::from_value::<StrandSignaturePk>(malformed).is_err(),
            "accepted {bytes} bytes"
        );
    }
}

#[test]
fn signature_and_key_transport_rejects_truncation_and_trailing_bytes() {
    let secret = StrandSignatureSk::generate().unwrap();
    let public = StrandSignaturePk::from_sk(&secret).unwrap();
    let signature = secret.sign(b"fixture message").unwrap();
    let signature_bytes = signature.strand_serialize().unwrap();
    let key_bytes = public.strand_serialize().unwrap();
    assert_eq!(signature_bytes.len(), 64);
    assert_eq!(key_bytes.len(), 32);
    let restored_signature =
        StrandSignature::strand_deserialize(&signature_bytes).unwrap();
    StrandSignaturePk::strand_deserialize(&key_bytes)
        .unwrap()
        .verify(&restored_signature, b"fixture message")
        .unwrap();

    for length in [0, 1, 31, 63] {
        assert!(StrandSignature::strand_deserialize(
            &signature_bytes[..length]
        )
        .is_err());
    }
    for length in [0, 1, 31] {
        assert!(StrandSignaturePk::strand_deserialize(&key_bytes[..length])
            .is_err());
    }
    let mut trailing_signature = signature_bytes;
    trailing_signature.push(0);
    assert!(StrandSignature::strand_deserialize(&trailing_signature).is_err());
    let mut trailing_key = key_bytes;
    trailing_key.push(0);
    assert!(StrandSignaturePk::strand_deserialize(&trailing_key).is_err());

    for malformed in ["!", "AA==", ""] {
        assert!(StrandSignature::from_b64_string(malformed).is_err());
        assert!(StrandSignaturePk::from_der_b64_string(malformed).is_err());
        assert!(StrandSignatureSk::from_der_b64_string(malformed).is_err());
    }
}

#[test]
fn imported_signing_keys_preserve_key_identity_and_signature_validity() {
    let secret = StrandSignatureSk::generate().unwrap();
    let public = StrandSignaturePk::from_sk(&secret).unwrap();
    let imported = StrandSignatureSk::from_der_b64_string(
        &secret.to_der_b64_string().unwrap(),
    )
    .unwrap();
    let imported_public = StrandSignaturePk::from_der_b64_string(
        &public.to_der_b64_string().unwrap(),
    )
    .unwrap();
    imported_public
        .verify(
            &imported.sign(b"key import check").unwrap(),
            b"key import check",
        )
        .unwrap();
    assert_eq!(StrandSignaturePk::from_sk(&imported).unwrap(), public);

    // Trustee-key maps must identify equal keys even when loaded separately.
    let keys =
        HashSet::from([public.clone(), imported_public, reference_key()]);
    assert_eq!(keys.len(), 2);
    assert_eq!(format!("{public:?}").len(), 10);
    assert_eq!(
        secret.to_der().unwrap().len(),
        48,
        "export must retain PKCS#8 v1 compatibility"
    );
}

#[test]
fn sha2_outputs_match_independent_known_vectors() {
    let expected_sha512 = hex::decode(concat!(
        "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a219",
        "2992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
    ))
    .unwrap();
    assert_eq!(hash::hash(b"abc").unwrap(), expected_sha512);
    assert_eq!(
        hash::hash_to_array(b"abc").unwrap().as_slice(),
        expected_sha512
    );
    assert_eq!(
        hash::hash_b64(b"abc").unwrap(),
        STANDARD_NO_PAD.encode(&expected_sha512)
    );
    assert_eq!(
        hex::encode(hash::hash_sha256(b"abc").unwrap()),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

/// Own only the file we created, and remove it even if a later assertion fails.
/// create_new prevents an old file or symlink from being overwritten.
struct TestFile(PathBuf);

impl TestFile {
    fn new(contents: &[u8]) -> Self {
        static NEXT_FILE: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "strand-coverage-{}-{}",
            std::process::id(),
            NEXT_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let owned = Self(path);
        file.write_all(contents).unwrap();
        owned
    }
}

impl Drop for TestFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn file_hashing_reads_past_the_buffer_boundary_and_reports_io_errors() {
    // Constants also checked independently with Python hashlib. The 8,193-byte
    // case needs a second read after the production helper's 8,192-byte buffer.
    for (contents, expected) in [
        (
            vec![],
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ),
        (
            vec![b'a'; 8193],
            "9c10c48d1f1d6618db88fde2c25409181c9201ed34ec6815d62bcf57c10d177b",
        ),
    ] {
        let file = TestFile::new(&contents);
        assert_eq!(
            hex::encode(hash::hash_sha256_file(&file.0).unwrap()),
            expected
        );
        std::fs::remove_file(&file.0).unwrap();
        assert!(hash::hash_sha256_file(&file.0).is_err());
    }
    assert!(hash::hash_sha256_file(&std::env::temp_dir()).is_err());
}

#[test]
fn hash_json_requires_exactly_sixty_four_bytes() {
    let bytes = [42; 64];
    let wrapped = HashWrapper::new(bytes);
    let json = serde_json::to_string(&wrapped).unwrap();
    assert_eq!(
        serde_json::from_str::<HashWrapper>(&json)
            .unwrap()
            .into_inner(),
        bytes
    );
    assert_eq!(wrapped.to_inner(), bytes);
    for length in [0, 1, 63, 65] {
        assert!(
            serde_json::from_value::<HashWrapper>(serde_json::json!(
                vec![42; length]
            ))
            .is_err(),
            "accepted {length} bytes"
        );
    }
    for invalid in [
        serde_json::json!(null),
        serde_json::json!("hash"),
        serde_json::json!(vec![256; 64]),
    ] {
        assert!(serde_json::from_value::<HashWrapper>(invalid).is_err());
    }
}

#[test]
fn symmetric_decryption_rejects_wrong_keys_and_tampering_in_each_component() {
    let key = symm::sk_from_bytes(&[42; 32]).unwrap();
    for plaintext in [b"".as_slice(), b"private trustee share"] {
        let encrypted = symm::encrypt(key, plaintext).unwrap();
        assert_eq!(symm::decrypt(&key, &encrypted).unwrap(), plaintext);
        assert!(symm::decrypt(
            &symm::sk_from_bytes(&[43; 32]).unwrap(),
            &encrypted
        )
        .is_err());

        let mut changed_nonce = encrypted.clone();
        changed_nonce.nonce[0] ^= 1;
        assert!(symm::decrypt(&key, &changed_nonce).is_err());
        for index in 0..encrypted.encrypted_bytes.len() {
            let mut changed = encrypted.clone();
            changed.encrypted_bytes[index] ^= 1;
            assert!(
                symm::decrypt(&key, &changed).is_err(),
                "accepted a change at byte {index}"
            );
        }
        let mut truncated = encrypted.clone();
        truncated.encrypted_bytes.truncate(15);
        assert!(symm::decrypt(&key, &truncated).is_err());
        let constructed = symm::EncryptionData::new(
            encrypted.encrypted_bytes,
            encrypted.nonce.into(),
        );
        assert_eq!(symm::decrypt(&key, &constructed).unwrap(), plaintext);
    }
    for length in [0, 1, 31, 33, 64] {
        assert!(symm::sk_from_bytes(&vec![0; length]).is_err());
    }
}

#[test]
fn serialized_vectors_and_rectangles_preserve_shape_and_reject_bad_elements() {
    let values = StrandVector(vec![1_u32, 2, 3]);
    let encoded = values.strand_serialize().unwrap();
    assert_eq!(
        StrandVector::<u32>::strand_deserialize(&encoded).unwrap().0,
        values.0
    );
    assert!(StrandVector::<u64>::strand_deserialize(&encoded).is_err());
    let rectangle =
        StrandRectangle::new(vec![vec![1_u32, 2], vec![3, 4]]).unwrap();
    assert_eq!(rectangle.width(), 2);
    let restored = StrandRectangle::<u32>::strand_deserialize(
        &rectangle.strand_serialize().unwrap(),
    )
    .unwrap();
    assert_eq!(restored.rows(), rectangle.rows());
    assert!(StrandRectangle::<u32>::new(vec![]).is_err());
    assert!(StrandRectangle::new(vec![vec![1_u32], vec![2, 3]]).is_err());

    // Both formats contain length-prefixed rows. Valid row encodings with an
    // inconsistent width must still be rejected at the rectangle boundary.
    let ragged = StrandVector(vec![vec![1_u32], vec![2, 3]])
        .strand_serialize()
        .unwrap();
    assert!(StrandRectangle::<u32>::strand_deserialize(&ragged).is_err());
}

#[test]
fn backend_diagnostics_name_the_active_implementations() {
    let info = strand::info();
    assert_eq!(info["VERSION"], env!("CARGO_PKG_VERSION"));
    for component in ["HASH", "RNG", "SIGNATURE", "SYMMETRIC"] {
        assert!(info[component].contains("FIPS_ENABLED: FALSE"));
        assert!(strand::info_string().contains(&info[component]));
    }
}
