// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use serde_json::json;

// Expected values computed outside Rust with Python hashlib:
// inner = sha256(canonical(subject)).digest()
// code  = crockford32(sha256(b"step-signing-code" + request_id + inner)[:5])
const REQUEST_ID: &str = "3f2b8c1e-5d4a-4e6f-9a7b-1c2d3e4f5a6b";

fn request_id() -> Uuid {
    Uuid::parse_str(REQUEST_ID).unwrap()
}

#[test]
fn the_code_derives_from_the_request_and_its_subject() {
    assert_eq!(
        signing_code(request_id(), &json!({"channel": "online"})).unwrap(),
        "T2YD-0ADH"
    );
    assert_eq!(
        signing_code(request_id(), &json!({"channels": ["online", "kiosk"]}))
            .unwrap(),
        "CSDP-Y0K5"
    );
    assert_eq!(signing_code(Uuid::nil(), &json!({})).unwrap(), "HC2A-1KZN");
}

#[test]
fn the_subject_is_hashed_in_canonical_form() {
    let reordered: Value =
        serde_json::from_str(r#"{"channels":["online","kiosk"]}"#).unwrap();
    assert_eq!(signing_code(request_id(), &reordered).unwrap(), "CSDP-Y0K5");
    assert_eq!(
        sha256_hex(
            canonical_json(&json!({"channel": "online"}))
                .unwrap()
                .as_bytes()
        ),
        "e9b49e40680603b516bc245b07e543b85a74d995928878339559944370e5dd29"
    );
}

/// The id is hashed as lowercase hyphenated text, however it was written.
#[test]
fn the_request_id_is_hashed_in_one_text_form() {
    let upper = Uuid::parse_str(&REQUEST_ID.to_uppercase()).unwrap();
    let simple = Uuid::parse_str(&REQUEST_ID.replace('-', "")).unwrap();
    for id in [upper, simple] {
        assert_eq!(
            signing_code(id, &json!({"channel": "online"})).unwrap(),
            "T2YD-0ADH"
        );
    }
}

#[test]
fn a_float_subject_has_no_code() {
    assert_eq!(
        signing_code(request_id(), &json!({"x": 1.25})),
        Err(CanonicalJsonError::Float("1.25".to_string()))
    );
}

#[test]
fn crockford_base32_uses_the_40_leading_bits() {
    // 0xd0bcd029b1 is the first five bytes of the first code's digest.
    assert_eq!(crockford_code([0xd0, 0xbc, 0xd0, 0x29, 0xb1]), "T2YD-0ADH");
    assert_eq!(crockford_code([0; 5]), "0000-0000");
    assert_eq!(crockford_code([0xff; 5]), "ZZZZ-ZZZZ");
    // 0x0842108421 = 00001 repeated eight times.
    assert_eq!(crockford_code([0x08, 0x42, 0x10, 0x84, 0x21]), "1111-1111");
}

#[test]
fn the_payload_hash_is_lowercase_hex_sha256() {
    let canonical = concat!(
        r#"{"action":"open-voting","area_id":null,"code":"T2YD-0ADH","#,
        r#""config_revision":null,"created_at":"2028-05-02T11:04:05Z","#,
        r#""domain":"step-signing/v1","election_event_id":"e1","#,
        r#""election_id":"post-1","expires_at":"2028-05-02T11:34:05Z","#,
        r#""request_id":"3f2b8c1e-5d4a-4e6f-9a7b-1c2d3e4f5a6b","#,
        r#""requested_by":"u1","rule_revision":3,"#,
        r#""subject":{"channel":"online"},"#,
        r#""tenant_id":"90505c8a-23a9-4cdf-a26b-4e19f6a097d5"}"#
    );
    assert_eq!(
        payload_sha256(canonical),
        "16f70d73cfccd56ecd918eb4c3d7edcc8095444772b8c6bbcb4db2ebf0862aed"
    );
}
