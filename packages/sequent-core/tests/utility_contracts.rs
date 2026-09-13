// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Small boundary helpers still need real failure cases: object conversion,
//! numeric ordering and missing or malformed operator configuration.

#![cfg(feature = "default_features")]

use ordered_float::NotNan;
use sequent_core::types::to_map::ToMap;
use sequent_core::util::{
    external_config::{
        load_external_config, VoterPasswordPolicy, EXTERNAL_CONFIG_FILE_NAME,
    },
    float::FloatWrapper,
};
use serde::{Serialize, Serializer};
use serde_json::json;

#[derive(Clone)]
struct RefusesSerialization;
impl Serialize for RefusesSerialization {
    fn serialize<S: Serializer>(
        &self,
        _serializer: S,
    ) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("synthetic serializer failure"))
    }
}

#[test]
fn object_conversion_preserves_fields_and_returns_non_object_or_serializer_errors(
) {
    assert_eq!(
        json!({"voters": 17, "closed": true}).to_map().unwrap()["voters"],
        17
    );
    for scalar in [json!(null), json!([1, 2]), json!("text"), json!(3)] {
        assert!(scalar.to_map().unwrap_err().to_string().contains("Object"));
    }
    assert!(RefusesSerialization
        .to_map()
        .unwrap_err()
        .to_string()
        .contains("synthetic serializer failure"));
}

#[test]
fn ordered_result_numbers_reject_nan_without_changing_finite_values() {
    for value in [-10.25, 0.0, 42.5, f64::INFINITY, f64::NEG_INFINITY] {
        // NotNan intentionally permits infinities; callers requiring a finite
        // percentage must validate its range separately.
        let converted: NotNan<f64> =
            FloatWrapper::from(value).try_into().unwrap();
        assert_eq!(converted.into_inner(), value);
    }
    assert!(NotNan::<f64>::try_from(FloatWrapper::from(f64::NAN)).is_err());
}

#[test]
fn external_configuration_loads_from_the_requested_directory_and_reports_io_or_json_errors(
) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().to_str().unwrap();
    let path = directory.path().join(EXTERNAL_CONFIG_FILE_NAME);
    assert!(load_external_config(root).is_err());
    std::fs::write(&path, "{").unwrap();
    assert!(load_external_config(root).is_err());
    let mut data = json!({
        "election_event_json_file": "event.json", "realm_name": "tenant-north", "tenant_id": "north",
        "election_event_id": "mayor", "area_id": "city", "election_id": "council",
        "generate_voters": {"csv_file_name": "voters.csv", "fields": ["name"], "excluded_columns": [],
            "email_prefix": "voter", "domain": "example.invalid", "sequence_email_number": true,
            "sequence_start_number": 1, "voter_password": "synthetic-password", "password_salt": "synthetic-salt",
            "hashed_password": "synthetic-hash", "overseas_reference": "", "min_age": 18, "max_age": 120,
            "authorized_elections_count": 1, "email_verified": false},
        "duplicate_votes": {"row_id_to_clone": "synthetic-row"},
        "generate_applications": {"applicant_data": {}, "annotations": {}}
    });
    std::fs::write(&path, data.to_string()).unwrap();
    let loaded = load_external_config(root).unwrap();
    assert_eq!(loaded.tenant_id, "north");
    assert!(matches!(
        loaded.generate_voters.voter_password_policy,
        VoterPasswordPolicy::Fixed
    ));
    data["generate_voters"]["voter_password_policy"] =
        json!({"type": "random-numeric", "digits": 16});
    std::fs::write(&path, data.to_string()).unwrap();
    assert!(matches!(
        load_external_config(root)
            .unwrap()
            .generate_voters
            .voter_password_policy,
        VoterPasswordPolicy::RandomNumeric { digits: 16 }
    ));
}

#[test]
fn time_helpers_preserve_requested_offsets_and_unix_units() {
    use chrono::Utc;
    use sequent_core::util::date::{
        get_current_date, get_seconds_later, timestamp,
    };
    for seconds in [-30, 0, 60] {
        let before = Utc::now() + chrono::Duration::seconds(seconds);
        let shifted = get_seconds_later(seconds);
        let after = Utc::now() + chrono::Duration::seconds(seconds);
        assert!((before..=after).contains(&shifted));
    }
    let before = Utc::now().timestamp() as u64;
    let seconds = timestamp().unwrap();
    assert!((before..=Utc::now().timestamp() as u64).contains(&seconds));
    assert!(
        chrono::NaiveDate::parse_from_str(&get_current_date(), "%d/%m/%Y")
            .is_ok()
    );
}

#[test]
fn authentication_links_choose_the_requested_action_without_switching_event_scope(
) {
    use sequent_core::services::generate_urls::{get_auth_url, AuthAction};
    assert_eq!(
        get_auth_url(
            "https://voting.example.invalid",
            "north",
            "mayor",
            AuthAction::Login
        ),
        "https://voting.example.invalid/tenant/north/event/mayor/login"
    );
    assert_eq!(
        get_auth_url(
            "https://voting.example.invalid",
            "north",
            "mayor",
            AuthAction::Enroll
        ),
        "https://voting.example.invalid/tenant/north/event/mayor/enroll"
    );
}

#[test]
fn current_timestamp_can_be_parsed_and_system_timezone_matches_the_local_clock()
{
    use sequent_core::types::date_time::TimeZone;
    use sequent_core::util::date_time::{
        get_date_and_time, get_system_timezone,
    };
    let before = chrono::Utc::now();
    let timestamp =
        chrono::DateTime::parse_from_rfc3339(&get_date_and_time()).unwrap();
    assert!(timestamp >= before && timestamp <= chrono::Utc::now());
    let hours = chrono::Local::now().offset().local_minus_utc() / 3600;
    match get_system_timezone() {
        TimeZone::UTC => assert_eq!(hours, 0),
        TimeZone::Offset(offset) => assert_eq!(offset, hours),
    }
}

#[test]
fn integrity_checks_distinguish_an_unreadable_file_from_a_wrong_digest() {
    use sequent_core::util::integrity_check::{
        integrity_check, HashFileVerifyError,
    };
    let file = tempfile::NamedTempFile::new().unwrap();
    // Replacing the path with a directory makes open succeed but reading fail
    // on the supported Linux runtime. This needs no permission tricks or root.
    std::fs::remove_file(file.path()).unwrap();
    std::fs::create_dir(file.path()).unwrap();
    let result = integrity_check(&file, "unused".into());
    std::fs::remove_dir(file.path()).unwrap();
    assert!(matches!(result, Err(HashFileVerifyError::IoError(_, _))));
}
