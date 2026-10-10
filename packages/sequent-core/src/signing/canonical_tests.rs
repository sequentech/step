// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use chrono::TimeZone;
use serde_json::{json, Map};

// Expected bytes computed outside Rust with Python:
// json.dumps(v, sort_keys=True, separators=(",", ":"), ensure_ascii=False).
const NESTED: &str = "{\"A_1\":false,\"B\":-7,\"a\":{\"c\":\"\u{e9}\\n\\\"q\\\"\\u0001/\",\"d\":[3,{\"y\":true,\"z\":null}]},\"b\":1}";

fn nested() -> Value {
    json!({
        "b": 1,
        "a": {"d": [3, {"z": null, "y": true}], "c": "é\n\"q\"\u{1}/"},
        "B": -7,
        "A_1": false
    })
}

#[test]
fn keys_are_sorted_recursively_without_whitespace() {
    assert_eq!(canonical_json(&nested()).unwrap(), NESTED);
}

/// Insertion order must not leak into the output, whether or not
/// serde_json keeps it (`preserve_order` is on in some feature sets).
#[test]
fn insertion_order_does_not_matter() {
    let mut forward = Map::new();
    let mut backward = Map::new();
    for key in ["a", "b", "c"] {
        forward.insert(key.to_string(), json!(key));
    }
    for key in ["c", "b", "a"] {
        backward.insert(key.to_string(), json!(key));
    }
    let expected = r#"{"a":"a","b":"b","c":"c"}"#;
    assert_eq!(canonical_json(&Value::Object(forward)).unwrap(), expected);
    assert_eq!(canonical_json(&Value::Object(backward)).unwrap(), expected);
}

#[test]
fn floats_are_refused() {
    assert_eq!(
        canonical_json(&json!({"ok": 1, "bad": [1.5]})),
        Err(CanonicalJsonError::Float("1.5".to_string()))
    );
}

/// Only integers every JSON reader holds exactly (as an IEEE double) are
/// signed, so a browser re-rendering the payload gets the same bytes.
#[test]
fn integers_must_be_exact_in_a_double() {
    const MAX_SAFE: i64 = 9_007_199_254_740_991;
    assert_eq!(
        canonical_json(&json!([MAX_SAFE, -MAX_SAFE])).unwrap(),
        "[9007199254740991,-9007199254740991]"
    );
    for number in [json!(MAX_SAFE + 1), json!(-MAX_SAFE - 1), json!(u64::MAX)] {
        assert_eq!(
            canonical_json(&json!({ "n": number })),
            Err(CanonicalJsonError::UnsafeInteger(number.to_string()))
        );
    }
}

/// Keys are ASCII, so byte order and RFC 8785's UTF-16 order agree.
#[test]
fn non_ascii_keys_are_refused() {
    assert_eq!(
        canonical_json(&json!({"ok": {"é": 1}})),
        Err(CanonicalJsonError::NonAsciiKey("é".to_string()))
    );
    // Non-ASCII values are fine.
    assert_eq!(canonical_json(&json!({"k": "é"})).unwrap(), "{\"k\":\"é\"}");
}

#[test]
fn scalars_and_empty_containers() {
    assert_eq!(canonical_json(&json!(null)).unwrap(), "null");
    assert_eq!(canonical_json(&json!("x")).unwrap(), "\"x\"");
    assert_eq!(canonical_json(&json!({})).unwrap(), "{}");
    assert_eq!(canonical_json(&json!([])).unwrap(), "[]");
}

#[test]
fn errors_name_what_was_refused() {
    assert_eq!(
        CanonicalJsonError::Float("0.5".to_string()).to_string(),
        "a canonical payload can't hold the float 0.5"
    );
    assert_eq!(
        CanonicalJsonError::UnsafeInteger("9007199254740992".to_string())
            .to_string(),
        "a canonical payload can't hold the integer 9007199254740992 exactly"
    );
    assert_eq!(
        CanonicalJsonError::NonAsciiKey("é".to_string()).to_string(),
        "a canonical payload key must be ASCII: é"
    );
}

const REQUEST_ID: &str = "3f2b8c1e-5d4a-4e6f-9a7b-1c2d3e4f5a6b";

fn open_voting_fields() -> SigningPayloadFields {
    SigningPayloadFields {
        tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5".to_string(),
        election_event_id: "e1".to_string(),
        request_id: Uuid::parse_str(REQUEST_ID).unwrap(),
        action: SigningAction::OpenVoting,
        election_id: Some("post-1".to_string()),
        area_id: None,
        subject: json!({"channel": "online"}),
        config_revision: None,
        rule_revision: 3,
        requested_by: "u1".to_string(),
        // Sub-second precision is dropped from the signed times.
        created_at: Utc
            .with_ymd_and_hms(2028, 5, 2, 11, 4, 5)
            .unwrap()
            .checked_add_signed(chrono::Duration::milliseconds(789))
            .unwrap(),
        expires_at: Some(Utc.with_ymd_and_hms(2028, 5, 2, 11, 34, 5).unwrap()),
    }
}

#[test]
fn the_payload_has_the_common_fields_in_canonical_form() {
    let payload = SigningPayload::new(open_voting_fields()).unwrap();
    // The code is derived, never passed in.
    assert_eq!(payload.code(), "T2YD-0ADH");
    assert_eq!(payload.fields().rule_revision, 3);
    assert_eq!(
        payload.canonical().unwrap(),
        concat!(
            r#"{"action":"open-voting","area_id":null,"code":"T2YD-0ADH","#,
            r#""config_revision":null,"created_at":"2028-05-02T11:04:05Z","#,
            r#""domain":"step-signing/v1","election_event_id":"e1","#,
            r#""election_id":"post-1","expires_at":"2028-05-02T11:34:05Z","#,
            r#""request_id":"3f2b8c1e-5d4a-4e6f-9a7b-1c2d3e4f5a6b","#,
            r#""requested_by":"u1","rule_revision":3,"#,
            r#""subject":{"channel":"online"},"#,
            r#""tenant_id":"90505c8a-23a9-4cdf-a26b-4e19f6a097d5"}"#
        )
    );
}

#[test]
fn a_request_without_expiry_signs_a_null_expiry() {
    let payload = SigningPayload::new(SigningPayloadFields {
        expires_at: None,
        ..open_voting_fields()
    })
    .unwrap();
    assert!(payload
        .canonical()
        .unwrap()
        .contains(r#""expires_at":null,"#));
}

#[test]
fn a_float_in_the_subject_is_refused() {
    assert_eq!(
        SigningPayload::new(SigningPayloadFields {
            subject: json!({"ratio": 0.5}),
            ..open_voting_fields()
        }),
        Err(CanonicalJsonError::Float("0.5".to_string()))
    );
}

#[test]
fn an_unsafe_revision_is_refused() {
    let payload = SigningPayload::new(SigningPayloadFields {
        rule_revision: i64::MAX,
        ..open_voting_fields()
    })
    .unwrap();
    assert_eq!(
        payload.canonical(),
        Err(CanonicalJsonError::UnsafeInteger(i64::MAX.to_string()))
    );
}
