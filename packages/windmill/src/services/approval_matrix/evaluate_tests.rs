// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::services::approval_matrix::model_tests::{comelec_version_1, COMELEC_FIELDS};
use crate::services::approval_matrix::ApprovalRule;
use crate::types::application::ApplicationRejectReason::*;
use crate::types::application::ApplicationStatus::*;
use serde_json::json;

fn version_1() -> MatrixVersion {
    MatrixVersion::built_in(
        COMELEC_FIELDS
            .iter()
            .map(|field| field.to_string())
            .collect(),
    )
}

/// A registry voter whose listed fields differ from the enrollment.
fn voter(id: &str, differing: &[&str]) -> Candidate {
    let fields = COMELEC_FIELDS
        .iter()
        .map(|field| {
            let result = if differing.contains(field) {
                FieldMatch::DIFFERS
            } else {
                FieldMatch::MATCHES
            };
            (field.to_string(), result)
        })
        .collect();
    Candidate {
        user_id: Some(id.to_string()),
        username: Some(format!("voter-{id}")),
        inputs: RuleInputs {
            voter_found: true,
            fields,
            ..Default::default()
        }
        .with_counted_differences(),
    }
}

fn enrolled(mut candidate: Candidate) -> Candidate {
    candidate.inputs.already_enrolled = true;
    candidate
}

fn manual(mut candidate: Candidate) -> Candidate {
    candidate.inputs.identity = Some(IdentityMethod::MANUAL_ENTRY);
    candidate
}

fn outcome(
    evaluation: &Evaluation,
) -> (
    ApplicationStatus,
    Option<ApplicationRejectReason>,
    Option<usize>,
) {
    (
        evaluation.record.decision.clone(),
        evaluation.record.reason.clone(),
        evaluation.record.rule,
    )
}

fn one(candidate: Candidate) -> Evaluation {
    let identity = candidate.inputs.identity;
    evaluate(&version_1(), identity, None, vec![candidate])
}

/// What enrollment decided for one registry voter before matrices were
/// configurable.
fn decision_before_matrices(
    mismatches: usize,
    already_enrolled: bool,
    matches: impl Fn(&str) -> bool,
) -> (ApplicationStatus, Option<ApplicationRejectReason>) {
    if mismatches == 0 {
        if already_enrolled {
            (REJECTED, Some(ALREADY_APPROVED))
        } else {
            (ACCEPTED, None)
        }
    } else if mismatches == 1 {
        if already_enrolled {
            (REJECTED, Some(ALREADY_APPROVED))
        } else if !matches("embassy") {
            (ACCEPTED, None)
        } else {
            (PENDING, Some(NO_VOTER))
        }
    } else if mismatches == 2 && !matches("embassy") {
        (PENDING, Some(NO_VOTER))
    } else if mismatches == 2 && !matches("middleName") && !matches("lastName") {
        (PENDING, Some(NO_VOTER))
    } else {
        (REJECTED, Some(NO_VOTER))
    }
}

#[test]
fn version_1_decides_every_voter_as_before() {
    for combination in 0..(1u32 << COMELEC_FIELDS.len()) {
        let differing: Vec<&str> = COMELEC_FIELDS
            .iter()
            .enumerate()
            .filter(|(index, _)| combination & (1 << index) != 0)
            .map(|(_, field)| *field)
            .collect();
        for already_enrolled in [false, true] {
            let mut candidate = voter("a", &differing);
            candidate.inputs.already_enrolled = already_enrolled;

            let evaluation = one(candidate);

            let expected = decision_before_matrices(differing.len(), already_enrolled, |field| {
                !differing.contains(&field)
            });
            assert_eq!(
                (
                    evaluation.record.decision.clone(),
                    evaluation.record.reason.clone()
                ),
                expected,
                "differing {differing:?}, already enrolled {already_enrolled}"
            );
            assert_eq!(evaluation.record.invariant, None);
        }
    }
}

#[test]
fn version_1_decides_documents_without_a_middle_name_as_before() {
    // Driver's License and Seafarer's Book compare first and middle name
    // together, so the middle name has no result of its own.
    let fields: Vec<&str> = COMELEC_FIELDS
        .iter()
        .copied()
        .filter(|field| *field != "middleName")
        .collect();
    for combination in 0..(1u32 << fields.len()) {
        let differing: Vec<&str> = fields
            .iter()
            .enumerate()
            .filter(|(index, _)| combination & (1 << index) != 0)
            .map(|(_, field)| *field)
            .collect();
        let mut candidate = voter("a", &differing);
        candidate.inputs.fields.remove("middleName");

        let evaluation = one(candidate);

        let expected = decision_before_matrices(differing.len(), false, |field| {
            field != "middleName" && !differing.contains(&field)
        });
        assert_eq!(
            (
                evaluation.record.decision.clone(),
                evaluation.record.reason.clone()
            ),
            expected,
            "differing {differing:?}"
        );
    }
}

#[test]
fn each_branch_is_decided_by_its_rule() {
    for (candidate, expected) in [
        (
            enrolled(voter("a", &[])),
            (REJECTED, Some(ALREADY_APPROVED), Some(1)),
        ),
        (
            enrolled(voter("a", &["lastName"])),
            (REJECTED, Some(ALREADY_APPROVED), Some(1)),
        ),
        (
            manual(voter("a", &[])),
            (PENDING, Some(IDENTITY_NOT_VERIFIED), Some(2)),
        ),
        (voter("a", &[]), (ACCEPTED, None, Some(3))),
        (voter("a", &["embassy"]), (ACCEPTED, None, Some(4))),
        (
            voter("a", &["dateOfBirth"]),
            (PENDING, Some(NO_VOTER), Some(5)),
        ),
        (
            voter("a", &["embassy", "firstName"]),
            (PENDING, Some(NO_VOTER), Some(6)),
        ),
        (
            voter("a", &["middleName", "lastName"]),
            (PENDING, Some(NO_VOTER), Some(7)),
        ),
        (
            voter("a", &["firstName", "lastName"]),
            (REJECTED, Some(NO_VOTER), None),
        ),
        (
            voter("a", &["firstName", "lastName", "embassy"]),
            (REJECTED, Some(NO_VOTER), None),
        ),
    ] {
        let label = format!("{:?}", candidate.inputs);
        assert_eq!(outcome(&one(candidate)), expected, "{label}");
    }
}

#[test]
fn one_mismatch_other_than_the_post_goes_to_manual_review() {
    // meta#4134: these were accepted automatically.
    for field in ["firstName", "middleName", "lastName", "dateOfBirth"] {
        let evaluation = one(voter("a", &[field]));

        assert_eq!(
            outcome(&evaluation),
            (PENDING, Some(NO_VOTER), Some(5)),
            "{field}"
        );
        assert_eq!(evaluation.voter, None);
    }
}

#[test]
fn a_first_name_mismatch_is_not_rejected_by_other_registry_voters() {
    // meta#5652: a voter whose only mismatch is the first name goes to
    // manual review whichever other voters the registry returns, instead
    // of being rejected.
    let near = voter("b", &["firstName"]);
    let far = voter("a", &["firstName", "lastName", "dateOfBirth"]);
    let other = voter("c", &["firstName", "middleName", "dateOfBirth", "embassy"]);

    for candidates in [
        vec![near.clone(), far.clone(), other.clone()],
        vec![far.clone(), near.clone(), other.clone()],
        vec![other, far, near.clone()],
    ] {
        let evaluation = evaluate(&version_1(), None, None, candidates);

        assert_eq!(outcome(&evaluation), (PENDING, Some(NO_VOTER), Some(5)));
        assert_eq!(evaluation.record.inputs, near.inputs);
        assert_eq!(evaluation.record.candidates, 3);
    }
}

#[test]
fn the_decision_does_not_depend_on_the_order_of_registry_voters() {
    let candidates = vec![
        voter("d", &["firstName", "lastName", "dateOfBirth"]),
        voter("b", &["lastName"]),
        enrolled(voter("c", &["embassy"])),
        voter("a", &["embassy"]),
    ];
    let expected = evaluate(&version_1(), None, None, candidates.clone());
    assert_eq!(outcome(&expected), (ACCEPTED, None, Some(4)));
    assert_eq!(
        expected
            .voter
            .as_ref()
            .and_then(|voter| voter.user_id.clone()),
        Some("a".to_string())
    );

    let mut rotated = candidates.clone();
    for _ in 0..candidates.len() {
        rotated.rotate_left(1);
        assert_eq!(
            evaluate(&version_1(), None, None, rotated.clone()),
            expected
        );
        let mut reversed = rotated.clone();
        reversed.reverse();
        assert_eq!(evaluate(&version_1(), None, None, reversed), expected);
    }
}

#[test]
fn several_accepted_voters_go_to_manual_review() {
    let evaluation = evaluate(
        &version_1(),
        None,
        None,
        vec![
            voter("b", &[]),
            voter("a", &[]),
            voter("c", &["lastName", "firstName"]),
        ],
    );

    assert_eq!(outcome(&evaluation), (PENDING, Some(OTHER), Some(3)));
    assert_eq!(evaluation.voter, None);
    assert_eq!(evaluation.record.candidates, 3);
    assert_eq!(evaluation.record.accepted_candidates, 2);
}

#[test]
fn manual_review_wins_over_rejection() {
    let evaluation = evaluate(
        &version_1(),
        None,
        None,
        vec![
            enrolled(voter("a", &[])),
            voter("b", &["embassy", "lastName"]),
        ],
    );

    assert_eq!(outcome(&evaluation), (PENDING, Some(NO_VOTER), Some(6)));
    assert_eq!(evaluation.voter, None);
}

#[test]
fn a_rejection_prefers_the_voter_who_is_already_enrolled() {
    let already = enrolled(voter("b", &["lastName"]));
    let evaluation = evaluate(
        &version_1(),
        None,
        None,
        vec![
            voter("a", &["firstName", "lastName", "embassy"]),
            already.clone(),
        ],
    );

    assert_eq!(
        outcome(&evaluation),
        (REJECTED, Some(ALREADY_APPROVED), Some(1))
    );
    assert_eq!(evaluation.voter, Some(already));
}

#[test]
fn without_a_registry_voter_only_rules_that_compare_nothing_apply() {
    let evaluation = evaluate(
        &version_1(),
        None,
        Some("philippinePassport".into()),
        vec![],
    );
    assert_eq!(outcome(&evaluation), (REJECTED, Some(NO_VOTER), None));
    assert_eq!(
        evaluation.record.inputs,
        RuleInputs {
            voter_found: false,
            valid_id: Some("philippinePassport".into()),
            ..Default::default()
        }
    );
    assert_eq!(evaluation.record.candidates, 0);

    let evaluation = evaluate(
        &version_1(),
        Some(IdentityMethod::MANUAL_ENTRY),
        None,
        vec![],
    );
    assert_eq!(
        outcome(&evaluation),
        (PENDING, Some(IDENTITY_NOT_VERIFIED), Some(2))
    );
}

fn matrix_with(rules: serde_json::Value, otherwise: serde_json::Value) -> MatrixVersion {
    let mut value = comelec_version_1();
    value["rules"] = rules;
    value["otherwise"] = otherwise;
    MatrixVersion {
        version: 4,
        source: MatrixSource::SAVED,
        matrix: serde_json::from_value(value).unwrap(),
    }
}

#[test]
fn the_evaluator_never_accepts_what_the_invariants_forbid() {
    let accept_everything = matrix_with(
        json!([{"when": {}, "then": {"decision": "ACCEPTED"}}]),
        json!({"decision": "REJECTED", "reason": "NO_VOTER"}),
    );

    let manual_entry = manual(voter("a", &[]));
    let evaluation = evaluate(
        &accept_everything,
        Some(IdentityMethod::MANUAL_ENTRY),
        None,
        vec![manual_entry],
    );
    assert_eq!(
        outcome(&evaluation),
        (PENDING, Some(IDENTITY_NOT_VERIFIED), Some(1))
    );
    assert_eq!(
        evaluation.record.invariant,
        Some(Invariant::MANUAL_ENTRY_NOT_ACCEPTED)
    );
    assert_eq!(evaluation.voter, None);

    let evaluation = evaluate(
        &accept_everything,
        None,
        None,
        vec![enrolled(voter("a", &[]))],
    );
    assert_eq!(
        outcome(&evaluation),
        (REJECTED, Some(ALREADY_APPROVED), Some(1))
    );
    assert_eq!(
        evaluation.record.invariant,
        Some(Invariant::ALREADY_ENROLLED_NOT_ACCEPTED)
    );

    let evaluation = evaluate(&accept_everything, None, None, vec![]);
    assert_eq!(outcome(&evaluation), (REJECTED, Some(NO_VOTER), Some(1)));
    assert_eq!(
        evaluation.record.invariant,
        Some(Invariant::NO_VOTER_NOT_ACCEPTED)
    );

    let accepting_otherwise = matrix_with(json!([]), json!({"decision": "ACCEPTED"}));
    let evaluation = evaluate(&accepting_otherwise, None, None, vec![voter("a", &[])]);
    assert_eq!(outcome(&evaluation), (PENDING, Some(OTHER), None));
    assert_eq!(
        evaluation.record.invariant,
        Some(Invariant::OTHERWISE_NOT_ACCEPTED)
    );
}

#[test]
fn rules_can_require_the_identity_document_and_the_identity_method() {
    let matrix = matrix_with(
        json!([
            {"when": {"valid_id": "philippinePassport", "identity": "VERIFIED", "differing": "at_most_1"}, "then": {"decision": "ACCEPTED"}},
            {"when": {"voter_found": true, "already_enrolled": false}, "then": {"decision": "PENDING", "reason": "INSUFFICIENT_INFORMATION"}}
        ]),
        json!({"decision": "REJECTED", "reason": "NO_VOTER"}),
    );
    let with = |identity: Option<IdentityMethod>, valid_id: Option<&str>| {
        let mut candidate = voter("a", &["lastName"]);
        candidate.inputs.identity = identity;
        candidate.inputs.valid_id = valid_id.map(str::to_string);
        outcome(&evaluate(
            &matrix,
            identity,
            valid_id.map(str::to_string),
            vec![candidate],
        ))
    };

    assert_eq!(
        with(Some(IdentityMethod::VERIFIED), Some("philippinePassport")),
        (ACCEPTED, None, Some(1))
    );
    assert_eq!(
        with(Some(IdentityMethod::VERIFIED), Some("driversLicense")),
        (PENDING, Some(INSUFFICIENT_INFORMATION), Some(2))
    );
    assert_eq!(
        with(None, Some("philippinePassport")),
        (PENDING, Some(INSUFFICIENT_INFORMATION), Some(2))
    );
    assert_eq!(
        outcome(&evaluate(
            &matrix,
            None,
            None,
            vec![enrolled(voter("a", &[]))]
        )),
        (REJECTED, Some(NO_VOTER), None)
    );
}

#[test]
fn the_record_names_the_matrix_the_rule_and_the_inputs() {
    let candidate = voter("a", &["dateOfBirth"]);
    let evaluation = evaluate(&version_1(), None, None, vec![candidate.clone()]);

    assert_eq!(
        serde_json::to_value(&evaluation.record).unwrap(),
        json!({
            "matrix_version": 1,
            "matrix_source": "BUILT_IN",
            "rule": 5,
            "conditions": {"differing": "exactly_1", "fields": {"embassy": "MATCHES"}},
            "decision": "PENDING",
            "reason": "NO_VOTER",
            "inputs": {
                "identity": null,
                "voter_found": true,
                "already_enrolled": false,
                "valid_id": null,
                "fields": {
                    "firstName": "MATCHES",
                    "middleName": "MATCHES",
                    "lastName": "MATCHES",
                    "dateOfBirth": "DIFFERS",
                    "embassy": "MATCHES"
                },
                "differing": 1
            },
            "candidates": 1,
            "accepted_candidates": 0
        })
    );
}

#[test]
fn a_dry_run_with_the_recorded_inputs_returns_the_recorded_decision() {
    let matrix = version_1();
    for candidates in [
        vec![],
        vec![voter("a", &[])],
        vec![voter("a", &["embassy"])],
        vec![voter("a", &["lastName"])],
        vec![enrolled(voter("a", &[]))],
        vec![manual(voter("a", &[]))],
        vec![voter("a", &["firstName", "lastName", "dateOfBirth"])],
    ] {
        let record = evaluate(&matrix, None, None, candidates).record;

        let dry_run = decide(&matrix.matrix, &record.inputs);

        assert_eq!(dry_run.rule, record.rule);
        assert_eq!(dry_run.outcome.decision, record.decision);
        assert_eq!(dry_run.outcome.reason, record.reason);
    }
}

#[test]
fn rules_are_checked_in_order() {
    let mut matrix = version_1();
    let pending: ApprovalRule = serde_json::from_value(json!({
        "when": {"differing": "exactly_1", "fields": {"embassy": "DIFFERS"}},
        "then": {"decision": "PENDING", "reason": "NO_VOTER"}
    }))
    .unwrap();
    matrix.matrix.rules.insert(0, pending);

    let evaluation = evaluate(&matrix, None, None, vec![voter("a", &["embassy"])]);

    assert_eq!(outcome(&evaluation), (PENDING, Some(NO_VOTER), Some(1)));
}
