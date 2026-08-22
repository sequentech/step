// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super::validate`].
//!
//! Each starts from a bundle that passes and breaks exactly one thing, so a
//! failure names the rule rather than a pile of unrelated problems. The builder
//! below is deliberately the smallest bundle that validates cleanly — if it ever
//! stops doing so, [`a_sound_bundle_has_no_errors`] fails first and says why.

use super::problem::{Code, Severity};
use super::schema::ImportElectionEventSchema;
use super::validate::{validate, COUNTING_ALGORITHMS, PREFERENTIAL_ALGORITHMS};
use crate::types::ceremonies::CountingAlgType;
use std::str::FromStr;

const TENANT: &str = "3f0c9d21-7b4e-4a55-9c3a-1d2e5f6a7b80";

/// A bundle that validates cleanly: one election, one contest with two
/// candidates, a parent area and a leaf that carries the ballot.
fn sound() -> ImportElectionEventSchema {
    let json = serde_json::json!({
        "tenant_id": TENANT,
        "keycloak_event_realm": null,
        "election_event": {
            "id": "e0000000-0000-5000-8000-000000000000",
            "tenant_id": TENANT,
            "is_archived": false,
            "encryption_protocol": "RSA256"
        },
        "elections": [{
            "id": "e1000000-0000-5000-8000-000000000000",
            "tenant_id": TENANT,
            "election_event_id": "e0000000-0000-5000-8000-000000000000",
            "external_id": "officers"
        }],
        "contests": [{
            "id": "c1000000-0000-5000-8000-000000000000",
            "tenant_id": TENANT,
            "election_event_id": "e0000000-0000-5000-8000-000000000000",
            "election_id": "e1000000-0000-5000-8000-000000000000",
            "external_id": "president",
            "min_votes": 0,
            "max_votes": 1,
            "winning_candidates_num": 1,
            "voting_type": "non-preferential",
            "counting_algorithm": "plurality-at-large"
        }],
        "candidates": [
            {
                "id": "d1000000-0000-5000-8000-000000000000",
                "tenant_id": TENANT,
                "election_event_id": "e0000000-0000-5000-8000-000000000000",
                "contest_id": "c1000000-0000-5000-8000-000000000000",
                "external_id": "pres-a"
            },
            {
                "id": "d2000000-0000-5000-8000-000000000000",
                "tenant_id": TENANT,
                "election_event_id": "e0000000-0000-5000-8000-000000000000",
                "contest_id": "c1000000-0000-5000-8000-000000000000",
                "external_id": "pres-b"
            }
        ],
        "areas": [
            {
                "id": "a1000000-0000-5000-8000-000000000000",
                "tenant_id": TENANT,
                "election_event_id": "e0000000-0000-5000-8000-000000000000",
                "name": "North Region"
            },
            {
                "id": "a2000000-0000-5000-8000-000000000000",
                "tenant_id": TENANT,
                "election_event_id": "e0000000-0000-5000-8000-000000000000",
                "name": "North Local 1",
                "parent_id": "a1000000-0000-5000-8000-000000000000"
            }
        ],
        "area_contests": [{
            "id": "b1000000-0000-5000-8000-000000000000",
            "area_id": "a2000000-0000-5000-8000-000000000000",
            "contest_id": "c1000000-0000-5000-8000-000000000000"
        }],
        "scheduled_events": null,
        "reports": [],
        "keys_ceremonies": [],
        "applications": []
    });
    serde_json::from_value(json).expect("the sound fixture should deserialize")
}

/// Every error code the report carries, for terse assertions.
fn error_codes(bundle: &ImportElectionEventSchema) -> Vec<Code> {
    validate(bundle)
        .problems
        .iter()
        .filter(|problem| problem.severity == Severity::Error)
        .map(|problem| problem.code)
        .collect()
}

#[test]
fn a_sound_bundle_has_no_errors() {
    let report = validate(&sound());
    assert!(
        !report.has_errors(),
        "the fixture should validate cleanly, but got:\n{report}"
    );
}

// -- identity ---------------------------------------------------------------

#[test]
fn a_malformed_tenant_id_is_reported_readably() {
    // The schema carries this as a String so the module stays WASM-safe; this is
    // the check that replaces the one a Uuid field used to do at parse time.
    let mut bundle = sound();
    bundle.tenant_id = "not-a-uuid".into();
    assert!(error_codes(&bundle).contains(&Code::InvalidValue));
}

#[test]
fn an_empty_tenant_id_is_reported_as_missing_not_malformed() {
    let mut bundle = sound();
    bundle.tenant_id = "  ".into();
    assert!(error_codes(&bundle).contains(&Code::MissingField));
}

#[test]
fn a_uuid_is_accepted_in_any_case() {
    let mut bundle = sound();
    bundle.tenant_id = TENANT.to_uppercase();
    assert!(!validate(&bundle).has_errors());
}

#[test]
fn an_event_with_no_elections_is_rejected() {
    let mut bundle = sound();
    bundle.elections.clear();
    bundle.contests.clear();
    bundle.candidates.clear();
    bundle.area_contests.clear();
    assert!(error_codes(&bundle).contains(&Code::MissingField));
}

#[test]
fn an_event_with_no_areas_is_a_warning_not_an_error() {
    // Reported: a bundle with no areas would not import. It is consistent and the
    // platform takes it; it just means nobody can be given a ballot yet.
    let mut bundle = sound();
    bundle.areas.clear();
    bundle.area_contests.clear();

    let report = validate(&bundle);
    assert!(!report.has_errors(), "expected no errors, got:\n{report}");
    assert!(report
        .warnings()
        .any(|problem| problem.code == Code::BallotCoverage
            && problem.path == "areas"));
}

// -- references -------------------------------------------------------------

#[test]
fn a_contest_pointing_at_no_election_is_rejected() {
    let mut bundle = sound();
    bundle.contests[0].election_id =
        "f0000000-0000-5000-8000-000000000000".into();
    assert!(error_codes(&bundle).contains(&Code::DanglingReference));
}

#[test]
fn a_candidate_pointing_at_no_contest_is_rejected() {
    let mut bundle = sound();
    bundle.candidates[0].contest_id =
        Some("f0000000-0000-5000-8000-000000000000".into());
    assert!(error_codes(&bundle).contains(&Code::DanglingReference));
}

#[test]
fn a_candidate_with_no_contest_at_all_is_rejected() {
    let mut bundle = sound();
    bundle.candidates[0].contest_id = None;
    assert!(error_codes(&bundle).contains(&Code::MissingField));
}

#[test]
fn a_problem_names_the_external_id_not_the_uuid() {
    // Import regenerates every UUID, so the id in the bundle means nothing to
    // whoever has to fix the source. The external_id is what they typed.
    let mut bundle = sound();
    bundle.contests[0].election_id =
        "f0000000-0000-5000-8000-000000000000".into();
    let report = validate(&bundle);
    let problem = report
        .problems
        .iter()
        .find(|problem| problem.code == Code::DanglingReference)
        .expect("expected a dangling reference");
    assert_eq!(problem.external_id.as_deref(), Some("president"));
    assert_eq!(problem.path, "contests[0].election_id");
}

// -- area tree --------------------------------------------------------------

#[test]
fn an_area_pointing_at_no_parent_is_rejected() {
    let mut bundle = sound();
    bundle.areas[1].parent_id =
        Some("f0000000-0000-5000-8000-000000000000".into());
    assert!(error_codes(&bundle).contains(&Code::DanglingReference));
}

#[test]
fn an_area_cycle_is_rejected() {
    // An infinite tree hangs the Admin Portal rather than failing the import.
    let mut bundle = sound();
    let leaf = bundle.areas[1].id.clone();
    bundle.areas[0].parent_id = Some(leaf);
    assert!(error_codes(&bundle).contains(&Code::AreaCycle));
}

#[test]
fn an_area_that_is_its_own_parent_is_rejected() {
    let mut bundle = sound();
    let own = bundle.areas[1].id.clone();
    bundle.areas[1].parent_id = Some(own);
    assert!(error_codes(&bundle).contains(&Code::AreaCycle));
}

// -- contest arithmetic -----------------------------------------------------

#[test]
fn min_votes_above_max_votes_is_rejected() {
    let mut bundle = sound();
    bundle.contests[0].min_votes = Some(2);
    assert!(error_codes(&bundle).contains(&Code::ContestArithmetic));
}

#[test]
fn a_contest_missing_a_vote_count_is_rejected() {
    let mut bundle = sound();
    bundle.contests[0].max_votes = None;
    assert!(error_codes(&bundle).contains(&Code::MissingField));
}

#[test]
fn electing_more_winners_than_there_are_candidates_is_rejected() {
    let mut bundle = sound();
    bundle.contests[0].winning_candidates_num = Some(5);
    bundle.contests[0].max_votes = Some(5);
    assert!(error_codes(&bundle).contains(&Code::ContestArithmetic));
}

#[test]
fn allowing_more_selections_than_there_are_candidates_is_rejected() {
    let mut bundle = sound();
    bundle.contests[0].max_votes = Some(9);
    assert!(error_codes(&bundle).contains(&Code::ContestArithmetic));
}

#[test]
fn a_negative_vote_count_is_refused() {
    for field in ["min_votes", "max_votes", "winning_candidates_num"] {
        let mut bundle = sound();
        match field {
            "min_votes" => bundle.contests[0].min_votes = Some(-1),
            "max_votes" => bundle.contests[0].max_votes = Some(-1),
            _ => bundle.contests[0].winning_candidates_num = Some(-1),
        }

        let report = validate(&bundle);
        assert!(
            report
                .errors()
                .any(|problem| problem.code == Code::InvalidValue
                    && problem.path.ends_with(field)),
            "a negative {field} should be reported as an invalid value"
        );

        // No wraparound either: i64 to usize would make -1 enormous and trip "more
        // winners than candidates" instead. That field only — a negative `max_votes`
        // legitimately trips `min_votes > max_votes` as well.
        if field == "winning_candidates_num" {
            assert!(!report
                .errors()
                .any(|problem| problem.code == Code::ContestArithmetic));
        }
    }
}

#[test]
fn zero_is_a_count_a_contest_may_legitimately_carry() {
    // The bound is negative, not "not positive": a contest a voter may abstain in has
    // `min_votes` 0.
    let mut bundle = sound();
    bundle.contests[0].min_votes = Some(0);
    assert!(!error_codes(&bundle).contains(&Code::InvalidValue));
}

#[test]
fn a_contest_with_no_candidates_is_a_warning_not_an_error() {
    // An event still being configured has these, and the platform's own export of
    // one has to re-import. See the round-trip test at the bottom.
    let mut bundle = sound();
    bundle.candidates.clear();
    let report = validate(&bundle);
    assert!(!report.has_errors());
    assert!(report
        .warnings()
        .any(|problem| problem.code == Code::BallotCoverage));
}

// -- tally system -----------------------------------------------------------

#[test]
fn the_algorithm_list_is_the_enum_and_nothing_else() {
    // Fails if a variant's `strum` spelling ever differs from its `serde` one.
    for value in COUNTING_ALGORITHMS {
        assert!(
            CountingAlgType::from_str(value).is_ok(),
            "{value} is offered but the platform cannot parse it"
        );
    }
}

#[test]
fn the_preferential_list_matches_the_enum() {
    // Both directions: a variant wrongly listed, and one wrongly left out.
    for value in COUNTING_ALGORITHMS {
        let algorithm = CountingAlgType::from_str(value)
            .expect("every offered algorithm parses");
        assert_eq!(
            PREFERENTIAL_ALGORITHMS.contains(value),
            algorithm.is_preferential(),
            "{value}: the list and CountingAlgType::is_preferential disagree"
        );
    }
}

#[test]
fn an_unknown_counting_algorithm_is_rejected() {
    // single-transferable-vote is not a CountingAlgType variant, however
    // plausible it sounds.
    let mut bundle = sound();
    bundle.contests[0].counting_algorithm =
        Some("single-transferable-vote".into());
    assert!(error_codes(&bundle).contains(&Code::InvalidValue));
}

#[test]
fn an_unknown_voting_type_is_rejected() {
    let mut bundle = sound();
    bundle.contests[0].voting_type = Some("ranked".into());
    assert!(error_codes(&bundle).contains(&Code::InvalidValue));
}

#[test]
fn preferential_counted_by_plurality_is_rejected() {
    // Imports cleanly and then tallies wrongly: ballot encoding follows the
    // algorithm, so the rankings a voter entered are read as unordered picks.
    let mut bundle = sound();
    bundle.contests[0].voting_type = Some("preferential".into());
    assert!(error_codes(&bundle).contains(&Code::TallyMismatch));
}

#[test]
fn non_preferential_counted_by_irv_is_rejected() {
    let mut bundle = sound();
    bundle.contests[0].counting_algorithm = Some("instant-runoff".into());
    assert!(error_codes(&bundle).contains(&Code::TallyMismatch));
}

#[test]
fn every_preferential_algorithm_is_accepted_with_ranked_voting() {
    for algorithm in super::validate::PREFERENTIAL_ALGORITHMS {
        let mut bundle = sound();
        bundle.contests[0].voting_type = Some("preferential".into());
        bundle.contests[0].counting_algorithm = Some((*algorithm).into());
        assert!(
            !validate(&bundle).has_errors(),
            "{algorithm} should be valid for a preferential contest"
        );
    }
}

#[test]
fn every_non_preferential_algorithm_is_accepted_with_unranked_voting() {
    let preferential = super::validate::PREFERENTIAL_ALGORITHMS;
    for algorithm in super::validate::COUNTING_ALGORITHMS {
        if preferential.contains(algorithm) {
            continue;
        }
        let mut bundle = sound();
        bundle.contests[0].counting_algorithm = Some((*algorithm).into());
        assert!(
            !validate(&bundle).has_errors(),
            "{algorithm} should be valid for a non-preferential contest"
        );
    }
}

// -- ballot coverage --------------------------------------------------------

#[test]
fn a_contest_on_no_ballot_is_a_warning_not_an_error() {
    // Does not break the import; means nobody can vote in it.
    let mut bundle = sound();
    bundle.area_contests.clear();
    let report = validate(&bundle);
    assert!(!report.has_errors());
    assert!(report
        .warnings()
        .any(|problem| problem.code == Code::BallotCoverage));
}

#[test]
fn a_leaf_area_with_no_ballot_is_a_warning_not_an_error() {
    let mut bundle = sound();
    let orphan = bundle.areas[1].clone();
    let mut orphan = orphan;
    orphan.id = "a3000000-0000-5000-8000-000000000000".into();
    orphan.name = Some("Nowhere".into());
    orphan.parent_id = None;
    bundle.areas.push(orphan);

    let report = validate(&bundle);
    assert!(!report.has_errors());
    assert!(report
        .warnings()
        .any(|problem| problem.code == Code::BallotCoverage
            && problem.message.contains("Nowhere")));
}

#[test]
fn a_parent_area_needs_no_ballot_of_its_own() {
    // A parent is a grouping; only leaves carry contests. The sound fixture has
    // one, so this is really asserting the fixture stays representative.
    assert!(!validate(&sound()).has_errors());
}

// -- permission labels ------------------------------------------------------

#[test]
fn a_permission_label_is_a_warning_not_an_error() {
    // The bundle cannot know who holds which label, so this must not block a
    // build — but it is the single most expensive thing to discover after import.
    let mut bundle = sound();
    bundle.elections[0].permission_label = Some("officers".into());

    let report = validate(&bundle);
    assert!(!report.has_errors());
    assert_eq!(
        report
            .warnings()
            .filter(|problem| problem.code == Code::PermissionLabel)
            .count(),
        1
    );
}

#[test]
fn the_warning_names_the_labels_in_use() {
    let mut bundle = sound();
    bundle.elections[0].permission_label = Some("officers".into());
    let report = validate(&bundle);
    let warning = report
        .warnings()
        .find(|problem| problem.code == Code::PermissionLabel)
        .expect("expected a permission label warning");
    assert!(warning.message.contains("officers"));
}

#[test]
fn no_labels_means_no_warning() {
    assert_eq!(
        validate(&sound())
            .warnings()
            .filter(|problem| problem.code == Code::PermissionLabel)
            .count(),
        0
    );
}

/// A report definition carrying one permission label.
///
/// Built rather than taken from the fixture: `reports` is normally empty, because
/// definitions travel in `export_reports-<uuid>.csv`. See `schema::reports`.
fn labelled_report(
    bundle: &ImportElectionEventSchema,
    label: &str,
) -> crate::election_config::report::Report {
    crate::election_config::report::Report {
        id: "11111111-1111-4111-8111-111111111111".into(),
        election_event_id: bundle.election_event.id.clone(),
        tenant_id: bundle.election_event.tenant_id.clone(),
        election_id: None,
        report_type: "results".into(),
        template_alias: None,
        encryption_policy:
            crate::election_config::report::EReportEncryption::Unencrypted,
        cron_config: None,
        created_at: chrono::DateTime::UNIX_EPOCH,
        permission_label: Some(vec![label.into()]),
    }
}

#[test]
fn a_label_on_a_report_names_the_reports_collection() {
    let mut bundle = sound();
    bundle.reports = vec![labelled_report(&bundle, "auditors")];

    let report = validate(&bundle);
    let warning = report
        .warnings()
        .find(|problem| problem.code == Code::PermissionLabel)
        .expect("expected a permission label warning");

    assert_eq!(warning.path, "reports[].permission_label");
    assert!(warning.message.contains("auditors"));
}

#[test]
fn labels_on_both_are_reported_once_each() {
    let mut bundle = sound();
    bundle.elections[0].permission_label = Some("officers".into());
    bundle.reports = vec![labelled_report(&bundle, "auditors")];

    let report = validate(&bundle);
    let paths: Vec<&str> = report
        .warnings()
        .filter(|problem| problem.code == Code::PermissionLabel)
        .map(|problem| problem.path.as_str())
        .collect();

    assert_eq!(
        paths,
        vec!["elections[].permission_label", "reports[].permission_label"]
    );
}

// -- identifiers ------------------------------------------------------------

#[test]
fn two_entities_sharing_an_id_are_rejected() {
    let mut bundle = sound();
    let duplicate = bundle.candidates[0].id.clone();
    bundle.candidates[1].id = duplicate;
    assert!(error_codes(&bundle).contains(&Code::DuplicateId));
}

#[test]
fn an_id_shared_across_collections_is_rejected() {
    // A contest and an area colliding is just as bad as two contests colliding.
    let mut bundle = sound();
    let contest_id = bundle.contests[0].id.clone();
    bundle.areas[0].id = contest_id;
    assert!(error_codes(&bundle).contains(&Code::DuplicateId));
}

// -- reporting --------------------------------------------------------------

#[test]
fn every_problem_is_reported_not_just_the_first() {
    // Fixing a configuration one error per run is miserable.
    let mut bundle = sound();
    bundle.tenant_id = "nope".into();
    bundle.contests[0].voting_type = Some("ranked".into());
    bundle.contests[0].min_votes = Some(99);
    assert!(error_codes(&bundle).len() >= 3);
}

#[test]
fn the_report_serializes_for_a_front_end() {
    let mut bundle = sound();
    bundle.contests[0].counting_algorithm = Some("nonsense".into());
    let report = validate(&bundle);
    let json = serde_json::to_value(&report).unwrap();
    let first = &json["problems"][0];
    assert!(first["code"].is_string());
    assert!(first["severity"].is_string());
    assert!(first["path"].is_string());
    assert!(first["message"].is_string());
}

// -- round tripping ---------------------------------------------------------

#[test]
fn an_event_still_being_configured_can_be_re_imported() {
    // The property that decides where the severity line falls. windmill refuses an
    // import on errors, so anything the platform can itself export must validate
    // without them — otherwise this check breaks disaster recovery to enforce a
    // rule about authoring.
    //
    // A half-built event: a contest with no candidates yet, one not yet on a
    // ballot, and an area nobody has assigned contests to.
    let mut bundle = sound();
    bundle.candidates.clear();
    bundle.area_contests.clear();

    let report = validate(&bundle);
    assert!(
        !report.has_errors(),
        "a mid-configuration export must still import, but got:\n{report}"
    );
    assert!(
        !report.is_empty(),
        "it should still be reported, just not fatally"
    );
}

#[test]
fn an_inconsistent_bundle_is_still_refused() {
    // The other side of that line: warnings are for consistent-but-odd, not for
    // anything goes.
    let mut bundle = sound();
    bundle.candidates[0].contest_id =
        Some("f0000000-0000-5000-8000-000000000000".into());
    assert!(validate(&bundle).has_errors());
}
