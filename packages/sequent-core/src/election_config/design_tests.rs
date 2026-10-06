// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`]. The digests of real bundles, through the platform's
//! ballot-style builder, are in `preview_tests.rs`.

use super::*;
use serde_json::json;

fn digest(sha256: &str) -> DesignDigest {
    DesignDigest {
        area: "North".to_string(),
        election: "officers".to_string(),
        sha256: sha256.to_string(),
    }
}

fn design(version: u32, sha256: &str) -> BallotDesign {
    BallotDesign {
        area: "North".to_string(),
        election: "officers".to_string(),
        version,
        sha256: sha256.to_string(),
    }
}

#[test]
fn a_new_design_is_version_one() {
    assert_eq!(versioned(&[], vec![digest("aa")]), vec![design(1, "aa")]);
}

#[test]
fn an_unchanged_design_keeps_its_version() {
    assert_eq!(
        versioned(&[design(4, "aa")], vec![digest("aa")]),
        vec![design(4, "aa")]
    );
}

#[test]
fn a_changed_design_goes_up_by_one() {
    assert_eq!(
        versioned(&[design(4, "aa")], vec![digest("bb")]),
        vec![design(5, "bb")]
    );
}

#[test]
fn a_design_is_matched_by_area_and_election() {
    let mut other = design(7, "aa");
    other.area = "South".to_string();
    assert_eq!(
        versioned(&[other], vec![digest("aa")]),
        vec![design(1, "aa")]
    );
}

#[test]
fn the_approved_design_published_as_it_is_matches() {
    assert!(mismatches(&[design(3, "aa")], &[digest("aa")]).is_empty());
}

#[test]
fn a_design_edited_after_import_is_named() {
    let found = mismatches(&[design(3, "aa")], &[digest("bb")]);
    assert_eq!(
        found,
        vec![DesignMismatch {
            area: "North".to_string(),
            election: "officers".to_string(),
            expected: Some("aa".to_string()),
            actual: "bb".to_string(),
        }]
    );
    assert!(found[0].to_string().contains("not the approved design"));
}

#[test]
fn a_ballot_the_manifest_never_approved_is_named() {
    let found = mismatches(&[], &[digest("aa")]);
    assert_eq!(found[0].expected, None);
    assert!(found[0]
        .to_string()
        .contains("not in the signed configuration"));
}

#[test]
fn a_bucket_path_is_its_file_name() {
    assert_eq!(
        public_file_name("tenant-1/document-2/face.png"),
        Some("face.png")
    );
    assert_eq!(public_file_name("https://example.org/face.png"), None);
    assert_eq!(public_file_name("tenant-1/document-2/"), None);
    assert_eq!(public_file_name("tenant-1/document-2/a/b.png"), None);
}

#[test]
fn an_election_without_an_external_id_has_no_designs() {
    let election = crate::types::hasura::core::Election {
        id: "e1".to_string(),
        external_id: None,
        ..serde_json::from_value(json!({
            "id": "e1",
            "tenant_id": "t",
            "election_event_id": "ev",
            "name": "Officers"
        }))
        .unwrap()
    };
    let problem =
        DesignKeys::of_entities(&[], &[election], &[], &[]).unwrap_err();
    assert_eq!(problem.id.as_deref(), Some("design.no-stable-key"));
    assert_eq!(problem.code, Code::MissingField);
}

#[test]
fn ids_volatile_fields_and_entity_order_are_normalised() {
    let keys = DesignKeys::default()
        .with_document("c1", "contest:president")
        .with_document("c2", "contest:secretary");
    let one = json!({
        "id": "style-1",
        "tenant_id": "t1",
        "contests": [
            {"id": "c1", "created_at": "2026-01-01", "max_votes": 1},
            {"id": "c2", "created_at": "2026-01-02", "max_votes": 2}
        ],
        "election_dates": {"start_date": "2026-03-01"}
    });
    let two = json!({
        "id": "style-2",
        "tenant_id": "t2",
        "contests": [
            {"id": "c2", "created_at": "2027-01-02", "max_votes": 2},
            {"id": "c1", "created_at": "2027-01-01", "max_votes": 1}
        ]
    });
    let bytes = |value: Value| {
        let mut normalised = normalise(value, &keys);
        normalised.as_object_mut().unwrap().remove("id");
        let mut out = Vec::new();
        write_sorted(&normalised, &mut out);
        String::from_utf8(out).unwrap()
    };
    assert_eq!(bytes(one), bytes(two));
}

#[test]
fn a_real_change_is_not_normalised_away() {
    let keys = DesignKeys::default();
    let bytes = |value: Value| {
        let mut out = Vec::new();
        write_sorted(&normalise(value, &keys), &mut out);
        out
    };
    assert_ne!(
        bytes(json!({"contests": [{"id": "c1", "max_votes": 1}]})),
        bytes(json!({"contests": [{"id": "c1", "max_votes": 2}]}))
    );
}

#[test]
fn keys_are_sorted_whatever_the_map_order() {
    let mut out = Vec::new();
    write_sorted(
        &json!({"b": 1, "a": [1.5, {"d": null, "c": true}]}),
        &mut out,
    );
    assert_eq!(
        String::from_utf8(out).unwrap(),
        r#"{"a":[1.5,{"c":true,"d":null}],"b":1}"#
    );
}

fn written(value: Value, keys: &DesignKeys) -> String {
    let mut out = Vec::new();
    write_sorted(&normalise(value, keys), &mut out);
    String::from_utf8(out).unwrap()
}

fn president() -> DesignKeys {
    let mut keys = DesignKeys::default();
    keys.insert(EntityKind::Contest, "c1", Some("president"))
        .unwrap();
    keys
}

#[test]
fn a_label_that_reads_like_an_id_is_hashed_as_written() {
    let keys = president();
    let style = |label: &str| {
        json!({"contests": [{
            "id": "c1",
            "name": label,
            "name_i18n": {"en": label},
            "annotations": {"note": label}
        }]})
    };
    let text = written(style("c1"), &keys);
    assert!(text.contains(r#""id":"contest:president""#), "{text}");
    assert!(text.contains(r#""name":"c1""#), "{text}");
    assert!(text.contains(r#""en":"c1""#), "{text}");
    assert!(text.contains(r#""note":"c1""#), "{text}");
    assert_ne!(text, written(style("contest:president"), &keys));
}

#[test]
fn an_annotation_named_like_an_id_keeps_its_name() {
    let text = written(json!({"annotations": {"c1": "x"}}), &president());
    assert_eq!(text, r#"{"annotations":{"c1":"x"}}"#);
}

#[test]
fn a_translation_into_indonesian_is_not_an_id() {
    let text =
        written(json!({"name_i18n": {"en": "x", "id": "c1"}}), &president());
    assert_eq!(text, r#"{"name_i18n":{"en":"x","id":"c1"}}"#);
}

#[test]
fn references_are_replaced_wherever_a_field_holds_an_id() {
    let keys = president().with_document("d1", "face.png");
    let text = written(
        json!({
            "contest_id": "c1",
            "image_document_id": "d1",
            "contest_ids": ["c1", "unknown"],
            "election_id": "unknown"
        }),
        &keys,
    );
    assert_eq!(
        text,
        r#"{"contest_id":"contest:president","contest_ids":["contest:president","unknown"],"election_id":"unknown","image_document_id":"document:face.png"}"#
    );
}

#[test]
fn a_bucket_path_is_replaced_in_a_url_and_nowhere_else() {
    let path = "tenant-1/document-2/face.png";
    let text = written(
        json!({"url": path, "logo_url": path, "description": path}),
        &DesignKeys::default(),
    );
    assert_eq!(
        text,
        r#"{"description":"tenant-1/document-2/face.png","logo_url":"document:face.png","url":"document:face.png"}"#
    );
}
