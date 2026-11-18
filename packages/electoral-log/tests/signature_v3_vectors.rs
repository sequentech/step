// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The log signature scheme v3 against vectors made by an independent
//! implementation (`tests/data/gen_signature_v3_vectors.mjs`, Node's
//! Ed25519): this crate builds the same signed messages and the same
//! signatures with the same key, and verifies them.

use electoral_log::signature::{sign, signed_message, verify, Domain};
use serde::Deserialize;
use voting_crypto::signing::{Signature, SigningKey, VerificationKey};

#[derive(Deserialize)]
struct File {
    signing_key_seed_hex: String,
    public_key_b64: String,
    vectors: Vec<Vector>,
}

#[derive(Deserialize)]
struct Vector {
    name: String,
    domain: String,
    canonical_hex: String,
    signed_message_hex: String,
    signature_b64: String,
}

#[test]
fn the_scheme_matches_the_independent_vectors() {
    let file: File = serde_json::from_str(include_str!("data/signature_v3_vectors.json")).unwrap();
    let seed: [u8; 32] = hex::decode(&file.signing_key_seed_hex)
        .unwrap()
        .try_into()
        .unwrap();
    let key = SigningKey::from_bytes(seed).unwrap();
    let public = VerificationKey::from_sk(&key).unwrap();
    assert_eq!(public.to_b64_string().unwrap(), file.public_key_b64);
    let mut domains = Vec::new();
    for vector in &file.vectors {
        let domain = Domain::parse(&vector.domain)
            .unwrap_or_else(|| panic!("{}: unknown domain", vector.name));
        domains.push(domain);
        let canonical = hex::decode(&vector.canonical_hex).unwrap();
        assert_eq!(
            hex::encode(signed_message(domain, &canonical)),
            vector.signed_message_hex,
            "{}",
            vector.name
        );
        let expected = Signature::from_b64_string(&vector.signature_b64).unwrap();
        assert_eq!(
            sign(&key, domain, &canonical).unwrap(),
            expected,
            "{}",
            vector.name
        );
        verify(&public, &expected, domain, &canonical).unwrap();
    }
    for domain in Domain::ALL {
        assert!(domains.contains(&domain), "no vector for {domain:?}");
    }
}

#[test]
fn the_checkpoint_vector_is_this_crates_checkpoint() {
    use electoral_log::proofs::{checkpoint_signing_bytes, Checkpoint, Uuid};
    let file: File = serde_json::from_str(include_str!("data/signature_v3_vectors.json")).unwrap();
    let vector = file
        .vectors
        .iter()
        .find(|v| v.name == "checkpoint")
        .unwrap();
    let checkpoint = Checkpoint {
        log_name: "board".into(),
        log_uid: Uuid::from_u128(3),
        tree_size: 9,
        root: vec![5; 32],
    };
    let bytes = checkpoint_signing_bytes(
        &checkpoint,
        electoral_log::messages::newtypes::ElectoralLogCheckpointReason::TallyCompleted,
    )
    .unwrap();
    assert_eq!(hex::encode(bytes), vector.canonical_hex);
}

/// The statement of the "statement-real" vector: a real log statement with
/// fixed contents, so another implementation is checked on its record bytes
/// too, not only on the signing rule.
fn real_statement() -> electoral_log::messages::statement::Statement {
    use electoral_log::messages::newtypes::{ElectionIdString, EventIdString};
    use electoral_log::messages::statement::{Statement, StatementBody, StatementHead};
    let body = StatementBody::TallyOpen(ElectionIdString(Some("election-1".into())));
    let head = StatementHead {
        timestamp: 1_700_000_000,
        ..StatementHead::from_body(EventIdString("event-1".into()), &body)
    };
    Statement::new(head, body)
}

#[test]
fn the_real_statement_vector_is_this_crates_statement() {
    use voting_crypto::RecordSerialize;
    let file: File = serde_json::from_str(include_str!("data/signature_v3_vectors.json")).unwrap();
    let bytes = real_statement().record_serialize().unwrap();
    let vector = file
        .vectors
        .iter()
        .find(|v| v.name == "statement-real")
        .unwrap_or_else(|| {
            panic!(
                "add a statement-real vector with canonical_hex {}",
                hex::encode(&bytes)
            )
        });
    assert_eq!(hex::encode(&bytes), vector.canonical_hex);
    assert_eq!(vector.domain, Domain::Statement.as_str());
}
