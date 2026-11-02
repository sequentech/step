// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use base64::{engine::general_purpose::STANDARD, Engine as _};
use cryptography::context::{Context, RistrettoCtx};
use cryptography::cryptosystem::{elgamal, naoryung};
use cryptography::groups::Ristretto255Group;
use cryptography::utils::serialization::{Deserializable, Serializable};
use voting_crypto::ballot::{encrypt_payload, verify_opening, BallotCiphertext};
use wbraid::trustee::ballot_encryption_context;

#[test]
fn voter_ciphertext_is_read_and_decrypted_by_wbraid() {
    let keypair = elgamal::KeyPair::<RistrettoCtx>::generate();
    let joint_key = keypair.pkey.y.clone();
    let configuration_id = 0x112233445566778899aabbccddeeff00;
    let payload = [0x5a; 30];
    let (ciphertext, randomness) = encrypt_payload(configuration_id, &joint_key, &payload).unwrap();
    let entry = BallotCiphertext::new(vec!["contest".to_string()], ciphertext);
    let bytes = STANDARD.decode(entry.encode()).unwrap();
    // This literal prefix pins the canonical wire contract independently of
    // the platform encoder: one ID, then seven ASCII bytes, in big endian.
    assert_eq!(&bytes[..23], b"\0\0\0\0\0\0\0\x01\0\0\0\0\0\0\0\x07contest");
    let mut cursor = bytes.as_slice();
    assert_eq!(Vec::<String>::read(&mut cursor).unwrap(), ["contest"]);
    let ciphertext = naoryung::Ciphertext::<RistrettoCtx, 1>::deser(cursor).unwrap();
    let label = ballot_encryption_context::<RistrettoCtx>(configuration_id, &joint_key);
    let keypair = naoryung::KeyPair::augment(&keypair, &label).unwrap();
    let plaintext = keypair.decrypt(&ciphertext, &label).unwrap();
    assert_eq!(
        Ristretto255Group::decode_30_bytes(&plaintext[0]).unwrap(),
        payload
    );
    verify_opening(&entry, configuration_id, &joint_key, &payload, &randomness).unwrap();
}

#[test]
fn a_valid_ballot_rejects_a_foreign_execution_key_and_a_tampered_proof() {
    let joint_key = RistrettoCtx::random_element();
    let (ciphertext, _) = encrypt_payload(123, &joint_key, &[7; 30]).unwrap();
    let entry = BallotCiphertext::new(vec!["contest".to_string()], ciphertext);
    entry.verify(123, &joint_key).unwrap();
    assert!(entry.verify(124, &joint_key).is_err());
    assert!(entry.verify(123, &RistrettoCtx::random_element()).is_err());
    let mut tampered = entry.clone();
    tampered.ciphertext.v_b[0] = RistrettoCtx::random_element();
    assert!(tampered.verify(123, &joint_key).is_err());
}

#[test]
fn audit_opening_rejects_changed_plaintext_and_randomness() {
    let key = RistrettoCtx::random_element();
    let payload = [0x33; 30];
    let (ciphertext, randomness) = encrypt_payload(9, &key, &payload).unwrap();
    let entry = BallotCiphertext::new(vec!["contest".to_string()], ciphertext);
    verify_opening(&entry, 9, &key, &payload, &randomness).unwrap();
    assert!(verify_opening(&entry, 9, &key, &[0x34; 30], &randomness).is_err());
    assert!(verify_opening(&entry, 9, &key, &payload, &RistrettoCtx::random_scalar()).is_err());
}

#[test]
fn stored_ballots_reject_truncation_and_trailing_bytes() {
    let key = RistrettoCtx::random_element();
    let (ciphertext, _) = encrypt_payload(9, &key, &[0; 30]).unwrap();
    let entry = BallotCiphertext::new(vec!["contest".to_string()], ciphertext);
    let bytes = entry.ser();
    BallotCiphertext::<RistrettoCtx>::parse(&STANDARD.encode(&bytes)).unwrap();
    assert!(
        BallotCiphertext::<RistrettoCtx>::parse(&STANDARD.encode(&bytes[..bytes.len() - 1]))
            .is_err()
    );
    let mut trailing = bytes;
    trailing.push(0);
    assert!(BallotCiphertext::<RistrettoCtx>::parse(&STANDARD.encode(trailing)).is_err());
}
