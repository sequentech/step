// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`].

use super::*;
use crate::monitoring::problem::{Code, Severity};

const TURNOUT_BY_GROUP: &str = include_str!("fixtures/turnout_by_group.yaml");
const REQ_0260: &str = include_str!("fixtures/req_0260.yaml");
const THEME: &str = include_str!("fixtures/theme_default.yaml");
const SETTINGS: &str = include_str!("fixtures/settings.yaml");

const MINIMAL: &str = "
id: w
title: W
source: voter_turnout
query: {template: summary, measures: [voted]}
chart: {charts: {k: {type: kpi, query: data, value: voted}}, rows: [k]}
";

fn codes(report: &Report) -> Vec<(Code, String)> {
    report
        .problems
        .iter()
        .map(|problem| (problem.code, problem.path.clone()))
        .collect()
}

#[test]
fn accepts_each_kind_of_valid_document() {
    for (kind, yaml) in [
        ("widget", TURNOUT_BY_GROUP),
        ("dashboard", REQ_0260),
        ("theme", THEME),
        ("settings", SETTINGS),
    ] {
        let report = check_document(kind, "", yaml, "");
        assert!(report.is_accepted(), "{kind}: {report}");
    }
}

#[test]
fn reports_what_the_policy_refuses() {
    let yaml = MINIMAL.replace(
        "query: {template: summary, measures: [voted]}",
        "query: {template: summary, measures: [voted], sql: select 1}",
    );
    let report = check_document("widget", "w", &yaml, "");
    assert!(!report.is_accepted());
    assert!(
        report
            .problems
            .iter()
            .any(|problem| problem.path.starts_with("query")),
        "{report}"
    );
}

#[test]
fn reports_unreadable_yaml_rather_than_failing() {
    let report = check_document("widget", "w", "id: [unclosed", "");
    assert_eq!(report.problems[0].code, Code::Unreadable);
}

#[test]
fn refuses_an_unknown_kind() {
    let report = check_document("chart", "w", MINIMAL, "");
    assert_eq!(codes(&report), vec![(Code::InvalidValue, String::new())]);
}

#[test]
fn checks_a_document_against_the_rest_of_the_event() {
    let set = serde_json::json!({
        "widgets": {},
        "themes": {},
        "dashboards": {},
    });
    let report =
        check_document("dashboard", "req-0260", REQ_0260, &set.to_string());
    let errors: Vec<_> = report
        .problems
        .iter()
        .filter(|problem| problem.severity == Severity::Error)
        .collect();
    assert!(
        errors
            .iter()
            .any(|problem| problem.code == Code::DanglingReference
                && problem.path.starts_with("layout[")),
        "paths are relative to the document: {report}"
    );
    assert!(errors
        .iter()
        .all(|problem| !problem.path.starts_with("dashboards.")));
}

#[test]
fn replaces_the_stored_copy_of_the_document_being_edited() {
    let theme: serde_yaml::Value = serde_yaml::from_str(THEME).unwrap();
    let stale_widget: serde_yaml::Value =
        serde_yaml::from_str(&MINIMAL.replace("title: W", "title: Old"))
            .unwrap();
    let set = serde_json::json!({
        "widgets": {"w": stale_widget},
        "themes": {"default": theme},
    });
    let report = check_document("widget", "w", MINIMAL, &set.to_string());
    assert!(report.is_accepted(), "{report}");
}

#[test]
fn ignores_problems_of_other_documents() {
    let set = serde_json::json!({
        "dashboards": {"other": {"id": "other", "title": "O", "layout": [{"widget": "missing", "width": 6}]}},
    });
    let report = check_document("widget", "w", MINIMAL, &set.to_string());
    assert!(report.is_accepted(), "{report}");
}

#[test]
fn says_so_when_the_event_documents_are_unreadable() {
    let report = check_document("widget", "w", MINIMAL, "{not json");
    assert_eq!(codes(&report), vec![(Code::Unreadable, String::new())]);
    assert_eq!(report.problems[0].severity, Severity::Warning);
}

#[test]
fn a_draft_that_changes_its_id_replaces_the_document_it_was_opened_as() {
    let theme: serde_yaml::Value = serde_yaml::from_str(THEME).unwrap();
    let stored: serde_yaml::Value = serde_yaml::from_str(MINIMAL).unwrap();
    let set = serde_json::json!({
        "widgets": {"w": stored},
        "themes": {"default": theme},
    });
    let renamed = MINIMAL.replace("id: w", "id: w2");
    let report = check_document("widget", "w", &renamed, &set.to_string());
    assert_eq!(
        codes(&report),
        vec![(Code::InvalidId, "id".to_string())],
        "saved as 'w', the draft must keep that id: {report}"
    );
}
