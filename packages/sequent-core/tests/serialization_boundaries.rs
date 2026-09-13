// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Wire-format checks use hand-written bytes, so a matching encoder/decoder
//! mistake cannot make a round-trip test pass unnoticed.

#![cfg(feature = "default_features")]

use sequent_core::serialization::base64::{Base64Deserialize, Base64Serialize};
use sequent_core::serialization::deserialize_with_path::{
    deserialize_str, deserialize_value,
};
use sequent_core::util::convert_vec::{convert_map, IntoVec};
use sequent_core::util::integrity_check::{
    integrity_check, HashFileVerifyError,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::io::Write;
use strand::serialization::StrandSerialize;
use strand::util::StrandError;
use tempfile::NamedTempFile;

#[test]
fn base64_uses_unpadded_standard_encoding_of_little_endian_borsh() {
    assert_eq!(
        Base64Serialize::serialize(&0x01020304_u32).unwrap(),
        "BAMCAQ"
    );
    assert_eq!(
        <u32 as Base64Deserialize>::deserialize("BAMCAQ".into()).unwrap(),
        0x01020304
    );
}

#[test]
fn malformed_base64_and_truncated_or_trailing_borsh_are_rejected() {
    for encoded in ["!", "BAMCAQ==", "BA", "BAMCAQA"] {
        assert!(
            <u32 as Base64Deserialize>::deserialize(encoded.into()).is_err(),
            "accepted {encoded}"
        );
    }
}

/// Fault injection at the serialization boundary checks that a lower-level
/// error is preserved instead of exporting a success-shaped empty string.
struct Unserializable;

impl StrandSerialize for Unserializable {
    fn strand_serialize(&self) -> Result<Vec<u8>, StrandError> {
        Err(std::io::Error::other("fixture serialization failure").into())
    }
}

#[test]
fn serialization_errors_reach_the_caller() {
    let error = Base64Serialize::serialize(&Unserializable).unwrap_err();
    assert!(error.to_string().contains("fixture serialization failure"));
}

#[derive(Debug, Deserialize, PartialEq)]
struct Configuration {
    contests: Vec<SelectionLimit>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct SelectionLimit {
    max_votes: u32,
}

#[test]
fn configuration_errors_identify_the_nested_field_and_array_index() {
    let invalid =
        json!({"contests": [{"max_votes": 1}, {"max_votes": "many"}]});
    let from_value =
        deserialize_value::<Configuration>(invalid.clone()).unwrap_err();
    let text = invalid.to_string();
    let from_text = deserialize_str::<Configuration>(&text).unwrap_err();
    assert_eq!(from_value.path().to_string(), "contests[1].max_votes");
    assert_eq!(from_text.path().to_string(), "contests[1].max_votes");

    let valid = r#"{"contests":[{"max_votes":2}]}"#;
    assert_eq!(
        deserialize_str::<Configuration>(valid).unwrap(),
        Configuration {
            contests: vec![SelectionLimit { max_votes: 2 }],
        }
    );
    assert!(deserialize_str::<Configuration>("{").is_err());
}

#[test]
fn user_attribute_conversion_keeps_strings_without_coercing_other_json() {
    let cases = [
        (json!("one"), vec!["one"]),
        (json!(["one", 2, null, "two", false]), vec!["one", "two"]),
        (json!({"nested": "value"}), vec![]),
        (json!(null), vec![]),
    ];
    for (value, expected) in cases {
        assert_eq!(value.clone().into_vec(), expected);
        let converted =
            convert_map(HashMap::from([("attribute".into(), value)]));
        assert_eq!(converted["attribute"], expected);
    }
    assert_eq!("one".to_string().into_vec(), vec!["one"]);
    assert_eq!(
        vec!["one".to_string(), "two".to_string()].into_vec(),
        vec!["one", "two"]
    );
}

#[test]
fn file_integrity_checks_an_external_digest_and_detects_changed_bytes() {
    // SHA-256("abc") is a published standard vector. Computing the expected
    // digest with the production hash function would weaken this assertion.
    const ABC_SHA256: &str =
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(b"abc").unwrap();
    integrity_check(&file, ABC_SHA256.into()).unwrap();
    integrity_check(&file, ABC_SHA256.to_uppercase()).unwrap();

    file.write_all(b"changed").unwrap();
    assert!(matches!(
        integrity_check(&file, ABC_SHA256.into()),
        Err(HashFileVerifyError::HashMismatch(_, _))
    ));

    std::fs::remove_file(file.path()).unwrap();
    assert!(matches!(
        integrity_check(&file, ABC_SHA256.into()),
        Err(HashFileVerifyError::IoError(_, _))
    ));
}
