// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signer, SigningKey};
use sequent_core::ballot::{
    AuditableBallot, HashableBallot, SignedHashableBallot,
};
use sequent_core::wasm::wasm::verify_ballot_signature_js;
use serde_json::{json, Value};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
use web_sys::js_sys::JSON;

wasm_bindgen_test_configure!(run_in_browser);
const LEGACY: &str = include_str!("fixtures/legacy_ballot_v1.json");

fn signed_fixture() -> Value {
    let mut value: Value = serde_json::from_str(LEGACY).unwrap();
    value["version"] = json!(2);
    value["contests"] = json!([]);
    value["config"]["contests"] = json!([]);
    value["voter_signing_pk"] = Value::Null;
    value["voter_ballot_signature"] = Value::Null;
    let audit: AuditableBallot = serde_json::from_value(value.clone()).unwrap();
    let signed = SignedHashableBallot::try_from(&audit).unwrap();
    let hashable = HashableBallot::try_from(&signed).unwrap();
    let payload = borsh::to_vec(&hashable).unwrap();
    // Frame the documented signature message independently of Core's signing
    // helper: each UTF-8 identity and the payload have a u64 little-endian length.
    let mut message = Vec::new();
    for bytes in [
        b"ballot".as_slice(),
        b"election".as_slice(),
        payload.as_slice(),
    ] {
        message.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        message.extend_from_slice(bytes);
    }
    let key = SigningKey::from_bytes(&[7; 32]);
    let signature = key.sign(&message);
    // RFC 8410 SubjectPublicKeyInfo prefix for an Ed25519 public key.
    let mut der = vec![
        0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
    ];
    der.extend_from_slice(key.verifying_key().as_bytes());
    value["voter_signing_pk"] = json!(STANDARD.encode(der));
    value["voter_ballot_signature"] =
        json!(STANDARD.encode(signature.to_bytes()));
    value
}

fn verify(
    ballot: JsValue,
    election: JsValue,
    value: &Value,
) -> Result<JsValue, JsValue> {
    verify_ballot_signature_js(
        ballot,
        election,
        JSON::parse(&value.to_string()).unwrap(),
    )
}

#[wasm_bindgen_test]
fn current_signed_ballot_verifies_through_the_javascript_boundary() {
    let result =
        verify("ballot".into(), "election".into(), &signed_fixture()).unwrap();
    assert_eq!(result.as_bool(), Some(true));
}

#[wasm_bindgen_test]
fn signatures_reject_replay_and_changed_content() {
    let valid = signed_fixture();
    for (ballot, election) in [("other", "election"), ("ballot", "other")] {
        let error = verify(ballot.into(), election.into(), &valid)
            .unwrap_err()
            .as_string()
            .unwrap();
        assert!(error.starts_with(
            "Error verifying the ballot: Failed to verify signature:"
        ));
    }
    let mut changed = valid.clone();
    changed["issue_date"] = json!("tampered");
    assert!(verify("ballot".into(), "election".into(), &changed)
        .unwrap_err()
        .as_string()
        .unwrap()
        .contains("Failed to verify signature"));
}

#[wasm_bindgen_test]
fn unsigned_and_incomplete_pairs_have_distinct_results() {
    let valid = signed_fixture();
    for field in ["voter_signing_pk", "voter_ballot_signature"] {
        let mut partial = valid.clone();
        partial[field] = Value::Null;
        assert!(verify("ballot".into(), "election".into(), &partial)
            .unwrap_err()
            .as_string()
            .unwrap()
            .contains("Incomplete ballot signature"));
    }
    let mut unsigned = valid;
    unsigned["voter_signing_pk"] = Value::Null;
    unsigned["voter_ballot_signature"] = Value::Null;
    assert_eq!(
        verify("ballot".into(), "election".into(), &unsigned)
            .unwrap()
            .as_bool(),
        Some(false)
    );
}

#[wasm_bindgen_test]
fn javascript_type_errors_and_obsolete_versions_are_rejected() {
    let valid = signed_fixture();
    assert!(verify(JsValue::NULL, "election".into(), &valid)
        .unwrap_err()
        .as_string()
        .unwrap()
        .starts_with("Error deserializing ballot_id:"));
    assert!(verify("ballot".into(), JsValue::from_f64(7.0), &valid)
        .unwrap_err()
        .as_string()
        .unwrap()
        .starts_with("Error deserializing election_id:"));
    assert!(verify_ballot_signature_js(
        "ballot".into(),
        "election".into(),
        "{ not json }".into()
    )
    .is_err());
    let legacy: Value = serde_json::from_str(LEGACY).unwrap();
    assert!(verify("ballot".into(), "election".into(), &legacy)
        .unwrap_err()
        .as_string()
        .unwrap()
        .contains("Unexpected version 1, expected 2"));
}
