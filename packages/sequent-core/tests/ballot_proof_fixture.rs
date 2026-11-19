// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
#![cfg(feature = "default_features")]
//! The portals' ballot proof story fixture
//! (`voting-portal/src/components/__stories__/ballotProof.json`) passes the
//! real verifier: its log entries carry the current (v3) log signatures.
//!
//! Its synthetic system key is not kept. After a change to the log's
//! signature scheme, regenerate it with
//! `cargo test -p sequent-core --features default_features --test ballot_proof_fixture -- --ignored`:
//! that re-signs the same statements with a fresh synthetic system key and
//! records its public key as the fixture's trusted key. The statements, and
//! the ballot box receipts inside them, are unchanged.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use electoral_log::messages::message::{Message, Sender};
use electoral_log::signature::{sign, Domain};
use sequent_core::ballot_privacy::{
    verify_ballot_log_proof, BallotPrivacyProof, BallotPrivacyTrust,
    CheckStatus,
};
use serde_json::Value;
use voting_crypto::signing::{SigningKey, VerificationKey};
use voting_crypto::{RecordDeserialize, RecordSerialize};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../voting-portal/src/components/__stories__/ballotProof.json"
);

fn read() -> Value {
    serde_json::from_str(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap()
}

#[test]
fn the_story_fixture_passes_the_verifier() {
    let fixture = read();
    let trusted: BallotPrivacyTrust =
        serde_json::from_value(fixture["trusted"].clone()).unwrap();
    let proof: BallotPrivacyProof =
        serde_json::from_value(fixture["proof"].clone()).unwrap();
    let report = verify_ballot_log_proof(&trusted, &proof);
    for status in [
        report.checks.binding,
        report.checks.received,
        report.checks.ballot_id,
        report.checks.cast,
    ] {
        assert_eq!(status, CheckStatus::Passed, "{report:?}");
    }
}

fn resign(entry: &str, key: &SigningKey) -> String {
    let bytes = STANDARD.decode(entry).unwrap();
    // A message from before `signature_version` lacks its leading byte.
    let current = Message::record_deserialize(&bytes).ok().filter(|m| {
        m.signature_version == electoral_log::signature::SIGNATURE_VERSION
    });
    let mut message = current.unwrap_or_else(|| {
        let mut upgraded = vec![electoral_log::signature::SIGNATURE_VERSION];
        upgraded.extend_from_slice(&bytes);
        Message::record_deserialize(&upgraded).unwrap()
    });
    message.signature_version = electoral_log::signature::SIGNATURE_VERSION;
    let bytes = message.statement.record_serialize().unwrap();
    let signature = sign(key, Domain::Statement, &bytes).unwrap();
    message.sender = Sender::new(
        message.sender.name.clone(),
        VerificationKey::from_sk(key).unwrap(),
    );
    message.sender_signature = signature.clone();
    message.system_signature = signature;
    STANDARD.encode(message.record_serialize().unwrap())
}

#[test]
#[ignore = "rewrites the story fixture with a fresh synthetic system key"]
fn regenerate_the_story_fixture() {
    let mut fixture = read();
    let key = SigningKey::generate().unwrap();
    for field in ["received_entry", "cast_entry"] {
        if let Some(entry) = fixture["proof"][field].as_str() {
            fixture["proof"][field] = Value::String(resign(entry, &key));
        }
    }
    fixture["trusted"]["system_public_key"] = Value::String(
        VerificationKey::from_sk(&key)
            .unwrap()
            .to_b64_string()
            .unwrap(),
    );
    std::fs::write(
        FIXTURE,
        serde_json::to_string_pretty(&fixture).unwrap() + "\n",
    )
    .unwrap();
}
