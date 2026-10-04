// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use chrono::{TimeZone, Utc};
use serde_json::json;

fn set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| value.to_string()).collect()
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn a_whole_post_report_records_every_country_and_initializes_the_post() {
    assert_eq!(
        initialization_plan(
            &set(&["a", "b"]),
            &strings(&["a", "b", "z"]),
            false,
            &set(&[])
        ),
        InitializationPlan {
            areas: vec![Some("a".to_string()), Some("b".to_string())],
            post_initialized: true,
        }
    );
}

#[test]
fn a_post_without_countries_records_one_row_for_the_post() {
    assert_eq!(
        initialization_plan(&set(&[]), &[], false, &set(&[])),
        InitializationPlan {
            areas: vec![None],
            post_initialized: true,
        }
    );
}

#[test]
fn a_country_report_initializes_the_post_with_its_last_country() {
    let post = set(&["a", "b"]);
    assert_eq!(
        initialization_plan(&post, &strings(&["a"]), true, &set(&[])),
        InitializationPlan {
            areas: vec![Some("a".to_string())],
            post_initialized: false,
        }
    );
    assert_eq!(
        initialization_plan(&post, &strings(&["b"]), true, &set(&["a"])),
        InitializationPlan {
            areas: vec![Some("b".to_string())],
            post_initialized: true,
        }
    );
    // Initializing a country again keeps the Post as it was.
    assert_eq!(
        initialization_plan(&post, &strings(&["a"]), true, &set(&["a"])),
        InitializationPlan {
            areas: vec![Some("a".to_string())],
            post_initialized: false,
        }
    );
}

#[test]
fn the_area_filter_is_read_from_the_session_annotations() {
    let session = |annotations: serde_json::Value| -> TallySession {
        serde_json::from_value(json!({
            "id": "s",
            "tenant_id": "t",
            "election_event_id": "e",
            "annotations": annotations,
            "is_execution_completed": false,
            "keys_ceremony_id": "k",
            "threshold": 1,
        }))
        .unwrap()
    };
    assert_eq!(
        initialization_area_filter(&session(json!({ "executer_username": "admin" }))),
        None
    );
    assert_eq!(
        initialization_area_filter(&session(
            json!({ INITIALIZATION_AREA_IDS_ANNOTATION: ["a"] })
        )),
        Some(strings(&["a"]))
    );
}

fn row(area: Option<Uuid>, area_name: Option<&str>, hash: Option<&str>) -> ElectionInitialization {
    ElectionInitialization {
        id: Uuid::from_u128(9),
        tenant_id: Uuid::nil(),
        election_event_id: Uuid::nil(),
        election_id: Uuid::from_u128(1),
        area_id: area,
        tally_session_id: Uuid::from_u128(2),
        results_event_id: None,
        report_hash: hash.map(str::to_string),
        document_id: Some(Uuid::from_u128(3)),
        created_by: Some("user".to_string()),
        created_by_username: Some("signer".to_string()),
        election_name: Some("Post A".to_string()),
        area_name: area_name.map(str::to_string),
        created_at: Utc.with_ymd_and_hms(2028, 4, 8, 19, 41, 0).unwrap(),
        log_staged_at: None,
    }
}

#[test]
fn each_row_logs_one_step_naming_the_post_the_country_and_the_hash() {
    let area = Uuid::from_u128(10);
    let step = initialization_log_step(&row(Some(area), Some("Country B"), Some("h1")));
    assert_eq!(step.kind, SigningStatementKind::ElectionInitialized);
    assert_eq!(step.system, SystemOutcome::Info);
    assert_eq!(step.user.username, "signer");
    assert_eq!(step.scope.election_id, Some(Uuid::from_u128(1)));
    assert_eq!(step.scope.area_id, Some(area));
    assert_eq!(
        step.description,
        "Initialized Post Post A, country Country B at 2028-04-08T19:41:00+00:00 with initialization report hash h1."
    );
    assert_eq!(step.details["report_hash"], json!("h1"));
    assert_eq!(step.details["document_id"], json!(Uuid::from_u128(3)));
    assert_eq!(step.details["tally_session_id"], json!(Uuid::from_u128(2)));
    assert_eq!(step.details["initialization_id"], json!(Uuid::from_u128(9)));
}

#[test]
fn a_missing_hash_is_logged_as_an_error_that_says_so() {
    let step = initialization_log_step(&row(None, None, None));
    assert_eq!(step.system, SystemOutcome::Error);
    assert_eq!(
        step.description,
        "Initialized Post Post A at 2028-04-08T19:41:00+00:00; the initialization report hash is missing (the report's results have none)."
    );
    assert_eq!(step.scope.area_id, None);
}

#[test]
fn a_whole_post_report_cannot_claim_a_retained_country_it_did_not_cover() {
    let plan = initialization_plan(&set(&["a", "b"]), &strings(&["a"]), false, &set(&[]));
    assert!(!plan.post_initialized);
    assert_eq!(plan.areas, vec![Some("a".into())]);
}

#[test]
fn a_whole_post_report_needs_actual_coverage_or_previously_initialized_countries() {
    assert!(!initialization_plan(&set(&["a", "b"]), &[], false, &set(&[])).post_initialized);
    assert!(
        initialization_plan(&set(&["a", "b"]), &strings(&["a"]), false, &set(&["b"]))
            .post_initialized
    );
}
