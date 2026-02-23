// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde_json::Value;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
use web_sys::js_sys::JSON;

use sequent_core::wasm::wasm::{
    decode_auditable_plaintext_ballot_js, encode_plaintext_contest_js,
    hash_auditable_plaintext_ballot_js,
    sign_hashable_plaintext_ballot_with_ephemeral_voter_signing_key_js,
    to_hashable_plaintext_ballot_js, verify_plaintext_ballot_signature_js,
};

// Configure tests to run in a browser environment
wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_verify_success() {
    // Encode the plaintext ballot using existing test data
    let decoded_contests_json =
        JSON::parse(PLAINTEXT_DECODED_CONTESTS_JSON).unwrap();
    let election_json = JSON::parse(PLAINTEXT_ELECTION_JSON).unwrap();

    let encode_result =
        encode_plaintext_contest_js(decoded_contests_json, election_json);
    assert!(encode_result.is_ok(), "Encoding should succeed");
    let auditable_ballot = encode_result.unwrap();

    // Get the ballot hash for use as ballot_id
    let hash_result =
        hash_auditable_plaintext_ballot_js(auditable_ballot.clone());
    assert!(hash_result.is_ok(), "Hashing should succeed");
    let ballot_hash: String =
        serde_wasm_bindgen::from_value(hash_result.unwrap()).unwrap();

    let ballot_id = JsValue::from_str(&ballot_hash);
    let election_id = JsValue::from_str("f2f1065e-b784-46d1-b81a-c71bfeb9ad55");

    // Sign the ballot
    let sign_result =
        sign_hashable_plaintext_ballot_with_ephemeral_voter_signing_key_js(
            ballot_id.clone(),
            election_id.clone(),
            auditable_ballot.clone(),
        );
    assert!(
        sign_result.is_ok(),
        "Signing should succeed. Error: {:?}",
        sign_result.err()
    );

    // Get the signed content
    let signed_content: Value =
        serde_wasm_bindgen::from_value(sign_result.unwrap()).unwrap();

    // Update the auditable ballot with signature information
    let mut auditable_ballot_value: Value =
        serde_wasm_bindgen::from_value(auditable_ballot).unwrap();
    auditable_ballot_value["voter_signing_pk"] =
        signed_content["public_key"].clone();
    auditable_ballot_value["voter_ballot_signature"] =
        signed_content["signature"].clone();

    // Convert back to JsValue
    let signed_auditable_ballot =
        JSON::parse(&auditable_ballot_value.to_string()).unwrap();

    // Verify the signature
    let result = verify_plaintext_ballot_signature_js(
        ballot_id,
        election_id,
        signed_auditable_ballot,
    );

    assert!(
        result.is_ok(),
        "Verification should succeed. Error: {:?}",
        result.err()
    );
    let js_val = result.unwrap();
    let verification_result: bool =
        serde_wasm_bindgen::from_value(js_val).unwrap();
    assert_eq!(
        verification_result, true,
        "The verification result should be true"
    );
}

#[wasm_bindgen_test]
fn test_verify_fails_on_bad_signature() {
    // Encode and sign a plaintext ballot using existing test data
    let decoded_contests_json =
        JSON::parse(PLAINTEXT_DECODED_CONTESTS_JSON).unwrap();
    let election_json = JSON::parse(PLAINTEXT_ELECTION_JSON).unwrap();

    let encode_result =
        encode_plaintext_contest_js(decoded_contests_json, election_json);
    assert!(encode_result.is_ok(), "Encoding should succeed");
    let auditable_ballot = encode_result.unwrap();

    // Get the ballot hash
    let hash_result =
        hash_auditable_plaintext_ballot_js(auditable_ballot.clone());
    assert!(hash_result.is_ok(), "Hashing should succeed");
    let ballot_hash: String =
        serde_wasm_bindgen::from_value(hash_result.unwrap()).unwrap();

    let ballot_id = JsValue::from_str(&ballot_hash);
    let election_id = JsValue::from_str("f2f1065e-b784-46d1-b81a-c71bfeb9ad55");

    // Sign the ballot
    let sign_result =
        sign_hashable_plaintext_ballot_with_ephemeral_voter_signing_key_js(
            ballot_id.clone(),
            election_id.clone(),
            auditable_ballot.clone(),
        );
    assert!(sign_result.is_ok(), "Signing should succeed");

    let signed_content: Value =
        serde_wasm_bindgen::from_value(sign_result.unwrap()).unwrap();

    // Create ballot with tampered signature
    let mut auditable_ballot_value: Value =
        serde_wasm_bindgen::from_value(auditable_ballot).unwrap();
    auditable_ballot_value["voter_signing_pk"] =
        signed_content["public_key"].clone();
    // Use a different (invalid) signature
    auditable_ballot_value["voter_ballot_signature"] =
        Value::String("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string());

    let tampered_ballot =
        JSON::parse(&auditable_ballot_value.to_string()).unwrap();

    // Verification should fail
    let result = verify_plaintext_ballot_signature_js(
        ballot_id,
        election_id,
        tampered_ballot,
    );

    assert!(
        result.is_err(),
        "Verification should fail due to bad signature"
    );
    let error_string = result.err().unwrap().as_string().unwrap();
    assert!(
        error_string.contains("Failed to verify signature")
            || error_string.contains("Verification equation was not satisfied")
            || error_string.contains("Failed to deserialize signature"),
        "Error message should mention failed verification. Got: {}",
        error_string
    );
}

#[wasm_bindgen_test]
fn test_fails_on_malformed_auditable_ballot_json() {
    let ballot_id = JsValue::from_str("test-ballot-id");
    let election_id = JsValue::from_str("f2f1065e-b784-46d1-b81a-c71bfeb9ad55");
    let auditable_plaintext_ballot_json =
        JsValue::from_str("{ not valid json }");

    let result = verify_plaintext_ballot_signature_js(
        ballot_id,
        election_id,
        auditable_plaintext_ballot_json,
    );

    assert!(result.is_err(), "Should fail on auditable ballot parsing");
    let error_string = result.err().unwrap().as_string().unwrap();
    assert!(
        error_string.contains("Failed to parse")
            || error_string.contains("Error parsing")
            || error_string.contains("Error deserializing"),
        "Error should mention parsing failure. Got: {}",
        error_string
    );
}

// Test data for plaintext ballot tests
const PLAINTEXT_ELECTION_JSON: &str = r#"{
    "id":"a12b9343-466e-429f-8ab4-99f6e32bf265",
    "tenant_id":"90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
    "election_event_id":"33f18502-a67c-4853-8333-a58630663559",
    "election_id":"f2f1065e-b784-46d1-b81a-c71bfeb9ad55",
    "description":"Test election for plaintext ballots",
    "public_key":{
        "public_key":"/jXUkdSIgz8mXLZ4BIDPQzDx7ZFFIG3MWuacDLyhyhoCAAAAGORKDU/t+8fKNkZMFfXl1IMM+/0VmINTZCcbalZ/NSUi5SbzUTlyzh25lMuVALwvC/lk3j6SHn6BotYphk0QMA",
        "is_demo":true
    },
    "area_id":"2f312a36-f39c-46e4-9670-1d1ce4625745",
    "status":null,
    "contests":[
        {
            "id":"69f2f987-460c-48ac-ac7a-4d44d99b37e6",
            "tenant_id":"90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
            "election_event_id":"33f18502-a67c-4853-8333-a58630663559",
            "election_id":"f2f1065e-b784-46d1-b81a-c71bfeb9ad55",
            "name":"Test Contest",
            "description":"Choose an option",
            "max_votes":1,
            "min_votes":1,
            "winning_candidates_num":1,
            "voting_type":"first-past-the-post",
            "counting_algorithm":"plurality-at-large",
            "is_encrypted":false,
            "candidates":[
                {
                    "id":"a24303de-5798-47cd-9b3e-4f391d1bae7b",
                    "tenant_id":"90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                    "election_event_id":"33f18502-a67c-4853-8333-a58630663559",
                    "election_id":"f2f1065e-b784-46d1-b81a-c71bfeb9ad55",
                    "contest_id":"69f2f987-460c-48ac-ac7a-4d44d99b37e6",
                    "name":"Option A",
                    "description":"First option",
                    "candidate_type":null,
                    "presentation":null
                },
                {
                    "id":"d9249345-11be-4652-ad04-298d70931610",
                    "tenant_id":"90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                    "election_event_id":"33f18502-a67c-4853-8333-a58630663559",
                    "election_id":"f2f1065e-b784-46d1-b81a-c71bfeb9ad55",
                    "contest_id":"69f2f987-460c-48ac-ac7a-4d44d99b37e6",
                    "name":"Option B",
                    "description":"Second option",
                    "candidate_type":null,
                    "presentation":null
                },
                {
                    "id":"1822089d-ae17-4a03-8935-25164b3f2142",
                    "tenant_id":"90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                    "election_event_id":"33f18502-a67c-4853-8333-a58630663559",
                    "election_id":"f2f1065e-b784-46d1-b81a-c71bfeb9ad55",
                    "contest_id":"69f2f987-460c-48ac-ac7a-4d44d99b37e6",
                    "name":"Option C",
                    "description":"Third option",
                    "candidate_type":null,
                    "presentation":null
                }
            ],
            "presentation":null
        }
    ]
}"#;

const PLAINTEXT_DECODED_CONTESTS_JSON: &str = r#"[{
    "contest_id":"69f2f987-460c-48ac-ac7a-4d44d99b37e6",
    "is_explicit_invalid":false,
    "invalid_errors":[],
    "invalid_alerts":[],
    "choices":[
        {"id":"a24303de-5798-47cd-9b3e-4f391d1bae7b","selected":0},
        {"id":"d9249345-11be-4652-ad04-298d70931610","selected":-1},
        {"id":"1822089d-ae17-4a03-8935-25164b3f2142","selected":-1}
    ]
}]"#;

// Helper function to create an auditable plaintext ballot from decoded contests
fn create_auditable_plaintext_ballot() -> JsValue {
    let decoded_contests_json =
        JSON::parse(PLAINTEXT_DECODED_CONTESTS_JSON).unwrap();
    let election_json = JSON::parse(PLAINTEXT_ELECTION_JSON).unwrap();

    let result =
        encode_plaintext_contest_js(decoded_contests_json, election_json);
    assert!(
        result.is_ok(),
        "Failed to create auditable plaintext ballot: {:?}",
        result.err()
    );
    result.unwrap()
}

#[wasm_bindgen_test]
fn test_encode_plaintext_contest_success() {
    let decoded_contests_json =
        JSON::parse(PLAINTEXT_DECODED_CONTESTS_JSON).unwrap();
    let election_json = JSON::parse(PLAINTEXT_ELECTION_JSON).unwrap();

    let result =
        encode_plaintext_contest_js(decoded_contests_json, election_json);

    assert!(
        result.is_ok(),
        "encode_plaintext_contest_js should succeed. Error: {:?}",
        result.err()
    );

    // Verify the result has expected structure
    let auditable_ballot: Value =
        serde_wasm_bindgen::from_value(result.unwrap()).unwrap();
    assert!(
        auditable_ballot.get("version").is_some(),
        "Should have version field"
    );
    assert!(
        auditable_ballot.get("issue_date").is_some(),
        "Should have issue_date field"
    );
    assert!(
        auditable_ballot.get("config").is_some(),
        "Should have config field"
    );
    assert!(
        auditable_ballot.get("contests").is_some(),
        "Should have contests field"
    );
    assert!(
        auditable_ballot.get("ballot_hash").is_some(),
        "Should have ballot_hash field"
    );
}

#[wasm_bindgen_test]
fn test_encode_plaintext_contest_fails_on_invalid_contests() {
    let invalid_contests_json =
        JSON::parse(r#"[{"invalid": "data"}]"#).unwrap();
    let election_json = JSON::parse(PLAINTEXT_ELECTION_JSON).unwrap();

    let result =
        encode_plaintext_contest_js(invalid_contests_json, election_json);

    assert!(result.is_err(), "Should fail on invalid decoded contests");
}

#[wasm_bindgen_test]
fn test_decode_auditable_plaintext_ballot_success() {
    let auditable_ballot = create_auditable_plaintext_ballot();

    let result = decode_auditable_plaintext_ballot_js(auditable_ballot);

    assert!(
        result.is_ok(),
        "decode_auditable_plaintext_ballot_js should succeed. Error: {:?}",
        result.err()
    );

    // Verify the decoded contests structure
    let decoded_contests: Value =
        serde_wasm_bindgen::from_value(result.unwrap()).unwrap();
    assert!(decoded_contests.is_array(), "Result should be an array");
    let contests_array = decoded_contests.as_array().unwrap();
    assert_eq!(contests_array.len(), 1, "Should have one contest");
}

#[wasm_bindgen_test]
fn test_encode_decode_plaintext_roundtrip() {
    // Encode the plaintext ballot
    let decoded_contests_json =
        JSON::parse(PLAINTEXT_DECODED_CONTESTS_JSON).unwrap();
    let election_json = JSON::parse(PLAINTEXT_ELECTION_JSON).unwrap();

    let encode_result =
        encode_plaintext_contest_js(decoded_contests_json, election_json);
    assert!(encode_result.is_ok(), "Encoding should succeed");
    let auditable_ballot = encode_result.unwrap();

    // Decode back
    let decode_result = decode_auditable_plaintext_ballot_js(auditable_ballot);
    assert!(decode_result.is_ok(), "Decoding should succeed");

    // Verify structure matches original
    let decoded_contests: Value =
        serde_wasm_bindgen::from_value(decode_result.unwrap()).unwrap();
    let original_contests: Value =
        serde_json::from_str(PLAINTEXT_DECODED_CONTESTS_JSON).unwrap();

    let decoded_array = decoded_contests.as_array().unwrap();
    let original_array = original_contests.as_array().unwrap();
    assert_eq!(
        decoded_array.len(),
        original_array.len(),
        "Should have same number of contests"
    );
}

#[wasm_bindgen_test]
fn test_to_hashable_plaintext_ballot_success() {
    let auditable_ballot = create_auditable_plaintext_ballot();

    let result = to_hashable_plaintext_ballot_js(auditable_ballot);

    assert!(
        result.is_ok(),
        "to_hashable_plaintext_ballot_js should succeed. Error: {:?}",
        result.err()
    );

    // Verify the hashable ballot structure
    let hashable_ballot: Value =
        serde_wasm_bindgen::from_value(result.unwrap()).unwrap();
    assert!(
        hashable_ballot.get("version").is_some(),
        "Should have version field"
    );
    assert!(
        hashable_ballot.get("issue_date").is_some(),
        "Should have issue_date field"
    );
    assert!(
        hashable_ballot.get("contests").is_some(),
        "Should have contests field"
    );
    assert!(
        hashable_ballot.get("config").is_some(),
        "Should have config field"
    );
    assert!(
        hashable_ballot.get("ballot_style_hash").is_some(),
        "Should have ballot_style_hash field"
    );
}

#[wasm_bindgen_test]
fn test_to_hashable_plaintext_ballot_fails_on_malformed_input() {
    let malformed_ballot = JsValue::from_str("{ not valid json }");

    let result = to_hashable_plaintext_ballot_js(malformed_ballot);

    assert!(result.is_err(), "Should fail on malformed input");
    let error_string = result.err().unwrap().as_string().unwrap();
    assert!(error_string.contains("Failed to parse auditable plaintext ballot"));
}

#[wasm_bindgen_test]
fn test_hash_auditable_plaintext_ballot_success() {
    let auditable_ballot = create_auditable_plaintext_ballot();

    let result = hash_auditable_plaintext_ballot_js(auditable_ballot);

    assert!(
        result.is_ok(),
        "hash_auditable_plaintext_ballot_js should succeed. Error: {:?}",
        result.err()
    );

    // Verify the hash is a non-empty string
    let hash: String = serde_wasm_bindgen::from_value(result.unwrap()).unwrap();
    assert!(!hash.is_empty(), "Hash should not be empty");
    assert_eq!(hash.len(), 64, "Hash should be 64 characters (SHA-256 hex)");
}

#[wasm_bindgen_test]
fn test_hash_auditable_plaintext_ballot_deterministic() {
    // Create two identical ballots and verify they produce the same hash
    let auditable_ballot1 = create_auditable_plaintext_ballot();
    let auditable_ballot2 = create_auditable_plaintext_ballot();

    let result1 = hash_auditable_plaintext_ballot_js(auditable_ballot1);
    let result2 = hash_auditable_plaintext_ballot_js(auditable_ballot2);

    assert!(
        result1.is_ok() && result2.is_ok(),
        "Both hashing operations should succeed"
    );

    let hash1: String =
        serde_wasm_bindgen::from_value(result1.unwrap()).unwrap();
    let hash2: String =
        serde_wasm_bindgen::from_value(result2.unwrap()).unwrap();

    assert_eq!(
        hash1, hash2,
        "Identical ballots should produce identical hashes"
    );
}

#[wasm_bindgen_test]
fn test_sign_plaintext_ballot_success() {
    let auditable_ballot = create_auditable_plaintext_ballot();
    let ballot_id = JsValue::from_str("test-ballot-id-12345");
    let election_id = JsValue::from_str("f2f1065e-b784-46d1-b81a-c71bfeb9ad55");

    let result =
        sign_hashable_plaintext_ballot_with_ephemeral_voter_signing_key_js(
            ballot_id,
            election_id,
            auditable_ballot,
        );

    assert!(
        result.is_ok(),
        "sign_hashable_plaintext_ballot_with_ephemeral_voter_signing_key_js should succeed. Error: {:?}",
        result.err()
    );

    // Verify the signed content structure
    let signed_content: Value =
        serde_wasm_bindgen::from_value(result.unwrap()).unwrap();
    assert!(
        signed_content.get("public_key").is_some(),
        "Should have public_key field"
    );
    assert!(
        signed_content.get("signature").is_some(),
        "Should have signature field"
    );
}

#[wasm_bindgen_test]
fn test_sign_and_verify_plaintext_ballot_roundtrip() {
    // First encode the ballot
    let decoded_contests_json =
        JSON::parse(PLAINTEXT_DECODED_CONTESTS_JSON).unwrap();
    let election_json = JSON::parse(PLAINTEXT_ELECTION_JSON).unwrap();

    let encode_result =
        encode_plaintext_contest_js(decoded_contests_json, election_json);
    assert!(encode_result.is_ok(), "Encoding should succeed");
    let auditable_ballot = encode_result.unwrap();

    // Get the ballot hash for use as ballot_id
    let hash_result =
        hash_auditable_plaintext_ballot_js(auditable_ballot.clone());
    assert!(hash_result.is_ok(), "Hashing should succeed");
    let ballot_hash: String =
        serde_wasm_bindgen::from_value(hash_result.unwrap()).unwrap();

    let ballot_id = JsValue::from_str(&ballot_hash);
    let election_id = JsValue::from_str("f2f1065e-b784-46d1-b81a-c71bfeb9ad55");

    // Sign the ballot
    let sign_result =
        sign_hashable_plaintext_ballot_with_ephemeral_voter_signing_key_js(
            ballot_id.clone(),
            election_id.clone(),
            auditable_ballot.clone(),
        );
    assert!(
        sign_result.is_ok(),
        "Signing should succeed. Error: {:?}",
        sign_result.err()
    );

    // Get the signed content
    let signed_content: Value =
        serde_wasm_bindgen::from_value(sign_result.unwrap()).unwrap();

    // Update the auditable ballot with signature information
    let mut auditable_ballot_value: Value =
        serde_wasm_bindgen::from_value(auditable_ballot).unwrap();
    auditable_ballot_value["voter_signing_pk"] =
        signed_content["public_key"].clone();
    auditable_ballot_value["voter_ballot_signature"] =
        signed_content["signature"].clone();

    // Convert back to JsValue
    let signed_auditable_ballot =
        JSON::parse(&auditable_ballot_value.to_string()).unwrap();

    // Verify the signature
    let verify_result = verify_plaintext_ballot_signature_js(
        ballot_id,
        election_id,
        signed_auditable_ballot,
    );

    assert!(
        verify_result.is_ok(),
        "Verification should succeed. Error: {:?}",
        verify_result.err()
    );

    let is_verified: bool =
        serde_wasm_bindgen::from_value(verify_result.unwrap()).unwrap();
    assert!(is_verified, "Signature verification should return true");
}

#[wasm_bindgen_test]
fn test_verify_plaintext_ballot_fails_with_tampered_signature() {
    // First encode and sign the ballot
    let decoded_contests_json =
        JSON::parse(PLAINTEXT_DECODED_CONTESTS_JSON).unwrap();
    let election_json = JSON::parse(PLAINTEXT_ELECTION_JSON).unwrap();

    let encode_result =
        encode_plaintext_contest_js(decoded_contests_json, election_json);
    assert!(encode_result.is_ok(), "Encoding should succeed");
    let auditable_ballot = encode_result.unwrap();

    // Get the ballot hash
    let hash_result =
        hash_auditable_plaintext_ballot_js(auditable_ballot.clone());
    assert!(hash_result.is_ok(), "Hashing should succeed");
    let ballot_hash: String =
        serde_wasm_bindgen::from_value(hash_result.unwrap()).unwrap();

    let ballot_id = JsValue::from_str(&ballot_hash);
    let election_id = JsValue::from_str("f2f1065e-b784-46d1-b81a-c71bfeb9ad55");

    // Sign the ballot
    let sign_result =
        sign_hashable_plaintext_ballot_with_ephemeral_voter_signing_key_js(
            ballot_id.clone(),
            election_id.clone(),
            auditable_ballot.clone(),
        );
    assert!(sign_result.is_ok(), "Signing should succeed");

    let signed_content: Value =
        serde_wasm_bindgen::from_value(sign_result.unwrap()).unwrap();

    // Create ballot with tampered signature
    let mut auditable_ballot_value: Value =
        serde_wasm_bindgen::from_value(auditable_ballot).unwrap();
    auditable_ballot_value["voter_signing_pk"] =
        signed_content["public_key"].clone();
    // Use a different (invalid) signature
    auditable_ballot_value["voter_ballot_signature"] =
        Value::String("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string());

    let tampered_ballot =
        JSON::parse(&auditable_ballot_value.to_string()).unwrap();

    // Verification should fail
    let verify_result = verify_plaintext_ballot_signature_js(
        ballot_id,
        election_id,
        tampered_ballot,
    );

    assert!(
        verify_result.is_err(),
        "Verification should fail with tampered signature"
    );
}

#[wasm_bindgen_test]
fn test_verify_plaintext_ballot_without_signature() {
    let auditable_ballot = create_auditable_plaintext_ballot();
    let ballot_id = JsValue::from_str("test-ballot-id");
    let election_id = JsValue::from_str("f2f1065e-b784-46d1-b81a-c71bfeb9ad55");

    // Verify a ballot that has no signature should return false (not error)
    let result = verify_plaintext_ballot_signature_js(
        ballot_id,
        election_id,
        auditable_ballot,
    );

    assert!(
        result.is_ok(),
        "Verification of unsigned ballot should not error. Error: {:?}",
        result.err()
    );

    let is_verified: bool =
        serde_wasm_bindgen::from_value(result.unwrap()).unwrap();
    assert!(
        !is_verified,
        "Unsigned ballot verification should return false"
    );
}
