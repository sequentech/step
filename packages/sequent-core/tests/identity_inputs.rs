// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Claims parsing is not signature verification. These tests exercise malformed
//! data and the post-verification freshness policy using synthetic identities.

#![cfg(all(feature = "default_features", feature = "keycloak"))]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(private_interfaces, private_bounds, unnameable_types)]
#![deny(rustdoc::missing_crate_level_docs, rustdoc::broken_intra_doc_links)]
#![deny(
    clippy::missing_docs_in_private_items,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::unwrap_used,
    clippy::panic,
    clippy::shadow_unrelated,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::indexing_slicing,
    clippy::future_not_send,
    clippy::arithmetic_side_effects,
    clippy::suspicious,
    clippy::complexity,
    clippy::style,
    clippy::perf,
    clippy::pedantic
)]

use support::TestResult;

mod support;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{TimeZone as _, Utc};
use sequent_core::services::date::ISO8601;
use sequent_core::services::jwt::{
    decode_jwt, decode_permission_labels, has_gold_permission, JwtClaims,
};
use sequent_core::services::replace_uuids::replace_uuids;
use sequent_core::types::date_time::{DateFormat, TimeZone};
use sequent_core::types::permissions::Permissions;
use sequent_core::util::date_time::{
    generate_timestamp, verify_date_format_ymd,
};
use serde_json::json;

fn claims() -> TestResult<JwtClaims> {
    Ok(serde_json::from_value(json!({
        "exp": 2_000_000_000, "iat": 0, "jti": "fixture-token",
        "iss": "https://identity.invalid", "sub": "fixture-user",
        "typ": "Bearer", "azp": "voting-portal", "acr": "1",
        "allowed-origins": [], "scope": "openid", "email_verified": false,
        "https://hasura.io/jwt/claims": {
            "x-hasura-default-role": "user", "x-hasura-tenant-id": "fixture-tenant",
            "x-hasura-user-id": "fixture-user", "x-hasura-allowed-roles": ["user"]
        }
    }))?)
}

#[test]
fn parsing_distinguishes_missing_payload_bad_encoding_and_invalid_claims(
) -> TestResult {
    for token in ["", "no-separators", "header.!.signature"] {
        assert!(decode_jwt(token).is_err());
    }
    for bytes in [&[0xff][..], b"{", b"{}"] {
        let token =
            format!("fixture.{}.unsigned", URL_SAFE_NO_PAD.encode(bytes));
        assert!(decode_jwt(&token).is_err());
    }

    let payload = serde_json::to_vec(&claims()?)?;
    let token = format!("fixture.{}.unsigned", URL_SAFE_NO_PAD.encode(payload));
    let parsed = decode_jwt(&token)?;
    assert_eq!(parsed.hasura_claims.tenant_id, "fixture-tenant");
    assert_eq!(parsed.hasura_claims.allowed_roles, vec!["user"]);
    Ok(())
}

#[test]
fn permission_labels_preserve_names_while_discarding_empty_items() -> TestResult
{
    let mut claims = claims()?;
    assert!(decode_permission_labels(&claims).is_empty());
    for (input, expected) in [
        (r#" { "north", "south" } "#, vec!["north", "south"]),
        ("north, south", vec!["north", "south"]),
        ("{ , , }", vec![]),
        ("", vec![]),
    ] {
        claims.hasura_claims.permission_labels = Some(input.into());
        assert_eq!(decode_permission_labels(&claims), expected);
    }
    Ok(())
}

#[test]
fn gold_access_requires_the_role_and_recent_authentication() -> TestResult {
    let mut claims = claims()?;
    claims.auth_time = Some(Utc::now().timestamp());
    assert!(!has_gold_permission(&claims));
    claims.acr = Permissions::GOLD.to_string();
    assert!(has_gold_permission(&claims));
    claims.auth_time = Some(Utc::now().timestamp() - 3600);
    assert!(!has_gold_permission(&claims));

    // Older identity providers omit auth_time; iat is the documented fallback.
    claims.auth_time = None;
    claims.iat = Utc::now().timestamp();
    assert!(has_gold_permission(&claims));
    claims.iat -= 3600;
    assert!(!has_gold_permission(&claims));
    Ok(())
}

#[test]
fn out_of_range_identity_timestamps_fail_without_integer_overflow() -> TestResult
{
    let mut claims = claims()?;
    claims.acr = Permissions::GOLD.to_string();
    for timestamp in [i64::MIN, i64::MAX] {
        assert!(ISO8601::timestamp_secs_utc_to_date_opt(timestamp).is_err());
        assert!(ISO8601::timestamp_ms_utc_to_date_opt(timestamp).is_err());
        claims.auth_time = Some(timestamp);
        assert!(!has_gold_permission(&claims));
        claims.auth_time = None;
        claims.iat = timestamp;
        assert!(!has_gold_permission(&claims));
    }
    Ok(())
}

#[test]
fn timestamp_formats_apply_offsets_without_changing_the_instant() -> TestResult
{
    let instant = Utc
        .with_ymd_and_hms(2024, 2, 29, 23, 30, 0)
        .single()
        .ok_or("fixed UTC timestamp is invalid")?;
    for (format, expected) in [
        (DateFormat::DdMmYyHhMm, "01/03/24 01:30"),
        (DateFormat::DdMmYyyyHhMm, "01/03/2024 01:30"),
        (DateFormat::MmDdYyHhMm, "03/01/24 01:30"),
        (DateFormat::MmDdYyyyHhMm, "03/01/2024 01:30"),
        (DateFormat::Custom("%Y-%m-%d".into()), "2024-03-01"),
        (DateFormat::Default, "01/03/2024 01:30"),
    ] {
        assert_eq!(
            generate_timestamp(
                Some(TimeZone::Offset(2)),
                Some(format),
                Some(instant)
            ),
            expected
        );
    }
    assert_eq!(
        generate_timestamp(None, None, Some(instant)),
        "29/02/2024 23:30"
    );
    assert_eq!(
        generate_timestamp(Some(TimeZone::Offset(24)), None, Some(instant)),
        "29/02/2024 23:30"
    );
    let local = ISO8601::to_date("2024-03-01T01:30:00+02:00")?;
    assert_eq!(local.timestamp(), instant.timestamp());
    assert_eq!(ISO8601::to_date_utc(&ISO8601::to_string(&local))?, instant);
    assert_eq!(
        ISO8601::timestamp_secs_utc_to_date_opt(instant.timestamp())?,
        local
    );
    assert_eq!(
        ISO8601::timestamp_ms_utc_to_date(instant.timestamp_millis()),
        local
    );
    assert!(ISO8601::to_date("not-a-date").is_err());
    assert!(ISO8601::to_date_utc("not-a-date").is_err());
    Ok(())
}

#[test]
fn birth_dates_reject_bad_calendar_dates_and_future_dates() -> TestResult {
    for (input, expected) in [
        ("2024/02/29", "Invalid date format"),
        ("year-02-29", "Invalid year"),
        ("2024-month-29", "Invalid month"),
        ("2024-02-day", "Invalid day"),
        ("1900-02-29", "Invalid date"),
        ("2024-13-01", "Invalid date"),
        ("9999-01-01", "Date is in the future"),
    ] {
        assert_eq!(
            verify_date_format_ymd(input)
                .err()
                .ok_or("expected the invalid input to be rejected")?,
            expected
        );
    }
    assert_eq!(
        verify_date_format_ymd("2000-02-29")?,
        Utc.with_ymd_and_hms(2000, 2, 29, 0, 0, 0)
            .single()
            .ok_or("fixed UTC timestamp is invalid")?
    );
    Ok(())
}

#[test]
fn copying_an_event_preserves_references_and_explicitly_kept_identifiers(
) -> TestResult {
    const REPLACED: &str = "a1000000-0000-4000-8000-000000000000";
    const KEPT: &str = "b1000000-0000-4000-8000-000000000000";
    let input =
        json!({"id": REPLACED, "reference": REPLACED, "tenant_id": KEPT})
            .to_string();
    let (copied, replacements) = replace_uuids(&input, vec![KEPT.into()]);
    let copied: serde_json::Value = serde_json::from_str(&copied)?;
    let identifier = copied
        .get("id")
        .and_then(serde_json::Value::as_str)
        .ok_or("copied identifier must be a string")?;
    assert_eq!(
        Some(identifier),
        copied.get("reference").and_then(serde_json::Value::as_str)
    );
    assert_ne!(identifier, REPLACED);
    assert_eq!(copied.get("tenant_id"), Some(&json!(KEPT)));
    assert_eq!(replacements.len(), 1);
    assert_eq!(
        replacements.get(REPLACED).map(String::as_str),
        Some(identifier)
    );
    assert!(uuid::Uuid::parse_str(identifier).is_ok());
    Ok(())
}
