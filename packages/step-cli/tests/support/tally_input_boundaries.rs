// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Validate import inputs before uploading and refuse partial GraphQL success
//! for operations whose result an operator may treat as completed.

use super::*;
use serde_json::json;

const ABC_DIGEST: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

#[test]
fn optional_digests_are_normalized_but_not_repaired_or_truncated() {
    assert_eq!(normalize_sha256(None).unwrap(), None);
    assert_eq!(normalize_sha256(Some(" \t\n")).unwrap(), None);
    assert_eq!(
        normalize_sha256(Some(&format!(" {}\n", ABC_DIGEST.to_uppercase()))).unwrap(),
        Some(ABC_DIGEST.into())
    );
    for invalid in [
        "abc".to_string(),
        "0".repeat(63),
        "0".repeat(65),
        "g".repeat(64),
        "é".repeat(32),
    ] {
        assert!(normalize_sha256(Some(&invalid)).is_err(), "{invalid}");
    }
}

#[test]
fn source_files_are_hashed_across_buffer_boundaries_using_independent_vectors() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("source.csv");
    for (bytes, expected) in [
        (b"abc".to_vec(), ABC_DIGEST),
        (
            vec![b'a'; 10000],
            "27dd1f61b867b6a0f6e9d8a41c43231de52107e53ae424de8f847b821db4b711",
        ),
    ] {
        fs::write(&file, bytes).unwrap();
        assert_eq!(sha256_file(&file).unwrap(), expected);
    }
    assert!(sha256_file(&directory.path().join("missing.csv")).is_err());
}

#[test]
fn contradictory_import_sources_and_digest_mismatches_fail_before_uploading() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("source.csv");
    fs::write(&file, b"different bytes").unwrap();
    for (path, document) in [
        (None, None),
        (Some(file.as_path()), Some("existing-document")),
    ] {
        assert!(resolve_import_document("event-a", path, document, None, false).is_err());
    }
    let error = resolve_import_document("event-a", Some(&file), None, Some(ABC_DIGEST), false)
        .err()
        .unwrap();
    assert!(error.to_string().contains("sha256 mismatch"));
}

#[test]
fn an_existing_document_keeps_its_normalized_digest_without_file_access() {
    let document = resolve_import_document(
        "event-a",
        None,
        Some("existing-document"),
        Some(&ABC_DIGEST.to_uppercase()),
        false,
    )
    .unwrap();
    assert_eq!(document.document_id, "existing-document");
    assert_eq!(document.sha256, Some(ABC_DIGEST.into()));
}

#[test]
fn json_inputs_distinguish_valid_objects_from_malformed_or_missing_files() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("sheet.json");
    fs::write(&file, r#"{"total_votes":7}"#).unwrap();
    assert_eq!(read_json_file(&file).unwrap(), json!({"total_votes":7}));
    fs::write(&file, "{broken").unwrap();
    assert!(read_json_file(&file).is_err());
    assert!(read_json_file(&directory.path().join("missing.json")).is_err());
}

#[test]
fn complete_graphql_data_is_returned_and_error_messages_remain_actionable() {
    let success: Response<Value> =
        serde_json::from_value(json!({"data":{"import_id":"accepted"}})).unwrap();
    assert_eq!(
        response_data(success).unwrap(),
        json!({"import_id":"accepted"})
    );
    // An empty errors list is not a failed operation; only actual errors take
    // precedence over a returned data object.
    let empty_errors: Response<Value> =
        serde_json::from_value(json!({"data":{"ok":true},"errors":[]})).unwrap();
    assert_eq!(response_data(empty_errors).unwrap(), json!({"ok":true}));
    let failure: Response<Value> = serde_json::from_value(
        json!({"errors":[{"message":"import is stale"},{"message":"reload it"}]}),
    )
    .unwrap();
    let error = response_data(failure).unwrap_err().to_string();
    assert!(error.contains("import is stale"));
    assert!(error.contains("reload it"));
    let empty: Response<Value> = serde_json::from_value(json!({})).unwrap();
    assert!(response_data(empty).is_err());
}

#[test]
fn partial_graphql_data_with_errors_is_not_reported_as_success() {
    let response: Response<Value> = serde_json::from_value(json!({
        "data":{"import_id":null}, "errors":[{"message":"approval failed"}]
    }))
    .unwrap();
    let error =
        response_data(response).expect_err("partial data cannot prove the operation completed");
    assert!(error.to_string().contains("approval failed"));
}
