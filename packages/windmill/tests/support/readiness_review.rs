// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use serde_json::{json, Value};

#[test]
fn tested_completion_requires_an_actual_matching_publication() {
    let control = tests::evidence();
    assert!(
        evaluate(
            serde_json::from_value(control.clone()).unwrap(),
            "now".into()
        )
        .passed
    );
    for publication in [
        Value::Null,
        json!({"publication_id":"publication", "matches_manifest":false, "design_count":2}),
        json!({"publication_id":"publication", "matches_manifest":true, "design_count":0}),
    ] {
        let mut evidence = control.clone();
        evidence["tested"]["publication"] = publication;
        let result = evaluate(serde_json::from_value(evidence).unwrap(), "now".into());
        assert!(
            result
                .checks
                .iter()
                .any(|check| check.kind == CheckKind::TestedFinished && !check.passed),
            "{:#?}",
            result.checks
        );
        assert!(!result.passed);
    }
}

#[test]
fn official_publication_is_not_a_readiness_predicate() {
    for publication in [
        Value::Null,
        json!({"publication_id":"publication", "matches_manifest":false, "design_count":2}),
        json!({"publication_id":"publication", "matches_manifest":true, "design_count":0}),
    ] {
        let mut evidence = tests::evidence();
        evidence["official"]["publication"] = publication;
        let result = evaluate(serde_json::from_value(evidence).unwrap(), "now".into());
        assert!(result.passed, "{:#?}", result.checks);
        assert_eq!(
            result.checks.iter().map(|check| check.kind).collect::<Vec<_>>(),
            [
                CheckKind::Configuration,
                CheckKind::CertifiedVoters,
                CheckKind::OfficialZero,
                CheckKind::TestedFinished,
            ]
        );
    }
}
