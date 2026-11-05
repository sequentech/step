// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use serde_json::json;

pub(crate) const COMELEC_FIELDS: [&str; 5] = [
    "firstName",
    "middleName",
    "lastName",
    "dateOfBirth",
    "embassy",
];

pub(crate) fn comelec_version_1() -> serde_json::Value {
    json!({
        "compared_fields": COMELEC_FIELDS,
        "rules": [
            {"when": {"already_enrolled": true, "differing": "at_most_1"}, "then": {"decision": "REJECTED", "reason": "ALREADY_APPROVED"}},
            {"when": {"identity": "MANUAL_ENTRY"}, "then": {"decision": "PENDING", "reason": "IDENTITY_NOT_VERIFIED"}},
            {"when": {"differing": "none"}, "then": {"decision": "ACCEPTED"}},
            {"when": {"differing": "exactly_1", "fields": {"embassy": "DIFFERS"}}, "then": {"decision": "ACCEPTED"}},
            {"when": {"differing": "exactly_1", "fields": {"embassy": "MATCHES"}}, "then": {"decision": "PENDING", "reason": "NO_VOTER"}},
            {"when": {"differing": "exactly_2", "fields": {"embassy": "DIFFERS"}}, "then": {"decision": "PENDING", "reason": "NO_VOTER"}},
            {"when": {"differing": "exactly_2", "fields": {"middleName": "DIFFERS", "lastName": "DIFFERS"}}, "then": {"decision": "PENDING", "reason": "NO_VOTER"}}
        ],
        "otherwise": {"decision": "REJECTED", "reason": "NO_VOTER"}
    })
}

fn matrix(value: serde_json::Value) -> ApprovalMatrix {
    serde_json::from_value(value).unwrap()
}

fn codes(matrix: &ApprovalMatrix) -> Vec<(MatrixErrorCode, Option<usize>)> {
    matrix
        .validate()
        .into_iter()
        .map(|error| (error.code, error.rule))
        .collect()
}

fn with_rule(rule: serde_json::Value) -> ApprovalMatrix {
    let mut value = comelec_version_1();
    value["rules"] = json!([rule]);
    matrix(value)
}

#[test]
fn the_built_in_matrix_is_comelec_version_1() {
    let fields = COMELEC_FIELDS
        .iter()
        .map(|field| field.to_string())
        .collect();
    let built_in = ApprovalMatrix::built_in(fields);

    assert_eq!(built_in, matrix(comelec_version_1()));
    assert_eq!(
        serde_json::to_value(&built_in).unwrap(),
        comelec_version_1()
    );
    assert_eq!(built_in.validate(), vec![]);
}

#[test]
fn unknown_keys_and_values_are_refused() {
    for (path, value) in [
        (vec!["unknown"], json!(1)),
        (vec!["otherwise", "decision"], json!("APPROVED")),
        (vec!["otherwise", "reason"], json!("BECAUSE")),
        (vec!["rules", "0", "when", "differing"], json!("exactly_4")),
        (vec!["rules", "0", "when", "identity"], json!("SELFIE")),
        (vec!["rules", "0", "when", "nationality"], json!("PH")),
        (
            vec!["rules", "3", "when", "fields", "embassy"],
            json!("SIMILAR"),
        ),
        (vec!["rules", "0", "then", "notify"], json!(true)),
    ] {
        let mut document = comelec_version_1();
        let mut target = &mut document;
        for key in &path[..path.len() - 1] {
            target = match key.parse::<usize>() {
                Ok(index) => &mut target[index],
                Err(_) => &mut target[*key],
            };
        }
        target[path[path.len() - 1]] = value;

        assert!(
            serde_json::from_value::<ApprovalMatrix>(document).is_err(),
            "{path:?} was accepted"
        );
    }
}

#[test]
fn a_rule_on_a_field_that_is_not_compared_is_refused() {
    let matrix = with_rule(json!({
        "when": {"fields": {"placeOfBirth": "DIFFERS"}},
        "then": {"decision": "PENDING", "reason": "NO_VOTER"}
    }));

    assert_eq!(
        matrix.validate(),
        vec![MatrixError {
            code: MatrixErrorCode::UNKNOWN_FIELD,
            rule: Some(1),
            field: Some("placeOfBirth".to_string()),
        }]
    );
}

#[test]
fn compared_fields_are_required_and_unique() {
    let mut empty = matrix(comelec_version_1());
    empty.compared_fields = vec![];
    empty.rules = vec![];
    assert_eq!(
        codes(&empty),
        vec![(MatrixErrorCode::NO_COMPARED_FIELDS, None)]
    );

    let mut repeated = matrix(comelec_version_1());
    repeated.compared_fields.push("embassy".to_string());
    assert_eq!(
        codes(&repeated),
        vec![(MatrixErrorCode::DUPLICATE_COMPARED_FIELD, None)]
    );

    let mut blank = matrix(comelec_version_1());
    blank.compared_fields.push(" ".to_string());
    assert_eq!(codes(&blank), vec![(MatrixErrorCode::UNKNOWN_FIELD, None)]);
}

#[test]
fn a_rule_that_accepts_a_manual_entry_is_refused() {
    let matrix = with_rule(json!({
        "when": {"identity": "MANUAL_ENTRY", "differing": "none"},
        "then": {"decision": "ACCEPTED"}
    }));

    assert_eq!(
        codes(&matrix),
        vec![(MatrixErrorCode::ACCEPTS_MANUAL_ENTRY, Some(1))]
    );
}

#[test]
fn a_rule_that_accepts_an_enrolled_voter_is_refused() {
    let matrix = with_rule(json!({
        "when": {"already_enrolled": true},
        "then": {"decision": "ACCEPTED"}
    }));

    assert_eq!(
        codes(&matrix),
        vec![(MatrixErrorCode::ACCEPTS_ALREADY_ENROLLED, Some(1))]
    );
}

#[test]
fn a_rule_that_accepts_without_a_voter_is_refused() {
    let matrix = with_rule(json!({
        "when": {"voter_found": false},
        "then": {"decision": "ACCEPTED"}
    }));

    assert_eq!(
        codes(&matrix),
        vec![(MatrixErrorCode::ACCEPTS_WITHOUT_VOTER, Some(1))]
    );
}

#[test]
fn an_accepting_last_rule_is_refused() {
    let mut value = comelec_version_1();
    value["otherwise"] = json!({"decision": "ACCEPTED"});

    assert_eq!(
        codes(&matrix(value)),
        vec![(MatrixErrorCode::OTHERWISE_ACCEPTS, None)]
    );
}

#[test]
fn a_decision_without_its_reason_is_refused() {
    let mut value = comelec_version_1();
    value["rules"][4]["then"] = json!({"decision": "PENDING"});
    value["rules"][2]["then"] = json!({"decision": "ACCEPTED", "reason": "OTHER"});
    value["otherwise"] = json!({"decision": "REJECTED"});

    assert_eq!(
        codes(&matrix(value)),
        vec![
            (MatrixErrorCode::UNEXPECTED_REASON, Some(3)),
            (MatrixErrorCode::MISSING_REASON, Some(5)),
            (MatrixErrorCode::MISSING_REASON, None),
        ]
    );
}

#[test]
fn errors_name_their_rule() {
    let error = MatrixError {
        code: MatrixErrorCode::UNKNOWN_FIELD,
        rule: Some(2),
        field: Some("placeOfBirth".to_string()),
    };
    assert_eq!(error.to_string(), "rule 2: UNKNOWN_FIELD (placeOfBirth)");

    let error = MatrixError {
        code: MatrixErrorCode::OTHERWISE_ACCEPTS,
        rule: None,
        field: None,
    };
    assert_eq!(error.to_string(), "OTHERWISE_ACCEPTS");
}

#[test]
fn differing_counts() {
    use DifferingFields::*;
    let included = |condition: DifferingFields| -> Vec<usize> {
        (0..5).filter(|count| condition.includes(*count)).collect()
    };

    assert_eq!(included(None), vec![0]);
    assert_eq!(included(Exactly1), vec![1]);
    assert_eq!(included(AtMost1), vec![0, 1]);
    assert_eq!(included(Exactly2), vec![2]);
    assert_eq!(included(AtMost2), vec![0, 1, 2]);
    assert_eq!(included(AtLeast3), vec![3, 4]);
}

#[test]
fn the_janitor_presets_can_be_saved() {
    let comelec = matrix(
        serde_json::from_str(include_str!(
            "../../../external-bin/janitor/templates/COMELEC/approvalMatrix.json"
        ))
        .unwrap(),
    );
    assert_eq!(comelec, matrix(comelec_version_1()));
    assert_eq!(comelec.validate(), vec![]);

    let association = matrix(
        serde_json::from_str(include_str!(
            "../../../external-bin/janitor/templates/association/approvalMatrix.json"
        ))
        .unwrap(),
    );
    assert_eq!(
        association.compared_fields,
        vec!["firstName", "lastName", "dateOfBirth"]
    );
    assert_eq!(association.rules.len(), 3);
    assert_eq!(
        association.otherwise,
        RuleOutcome::pending(ApplicationRejectReason::NO_VOTER)
    );
    assert_eq!(association.validate(), vec![]);
}
