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
    // And not as a missing field at any severity: areas are not required.
    assert!(!report
        .problems
        .iter()
        .any(|problem| problem.path == "areas"
            && problem.code == Code::MissingField));
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

#[test]
fn an_area_inheriting_its_parents_contests_is_not_empty() {
    // The platform walks from the root down to each area and gathers every
    // `area_contest` on the way, so a child with no links of its own still gets
    // everything its parent votes on.
    //
    // This check used to look only at direct links and warned that such an
    // area's voters "would see an empty ballot". Found by previewing a two-area
    // plan through the platform's own ballot-style builder and getting two
    // ballots next to a warning that said there would be one.
    let mut bundle = sound();

    let parent_id = bundle.areas[0].id.clone();
    let mut child = bundle.areas[1].clone();
    child.id = "a3000000-0000-5000-8000-000000000000".into();
    child.name = Some("Inherits".into());
    child.parent_id = Some(parent_id.clone());
    bundle.areas.push(child);

    // The parent votes on something; the child links to nothing.
    bundle
        .area_contests
        .iter_mut()
        .for_each(|link| link.area_id = parent_id.clone());

    let report = validate(&bundle);
    assert!(!report.has_errors());
    assert!(
        !report.warnings().any(|problem| {
            problem.code == Code::BallotCoverage
                && problem.message.contains("Inherits")
        }),
        "an area inheriting a contest is not empty; got:\n{report}"
    );
}

// -- ways of voting ---------------------------------------------------------

#[test]
fn a_bundle_that_names_no_channels_is_left_alone() {
    // Absent is not the same as empty. `election_event.hbs` always writes the
    // block, but a bundle from elsewhere need not, and "no way of voting is open"
    // would be a wrong thing to say about a bundle that never raised the subject —
    // the platform's own column default is `{"kiosk": true, "online": true}`.
    let bundle = sound();
    assert!(bundle.election_event.voting_channels.is_none());
    assert!(!validate(&bundle).has_errors());
}

#[test]
fn a_channel_nothing_reads_is_reported_without_refusing_the_bundle() {
    // `paper` is in `hasura_core::VotingChannels` and in nothing else — no
    // `VotingStatusChannel` variant, no status block, no Publish control, no
    // label. It does no harm sitting in the JSON, so this is a warning; but a
    // bundle that names it was written by somebody who believes they arranged a
    // way of voting, and they should hear otherwise before election day.
    let mut bundle = sound();
    bundle.election_event.voting_channels =
        Some(serde_json::json!({"online": true, "paper": true}));

    let report = validate(&bundle);
    assert!(!report.has_errors(), "{report}");
    assert!(report
        .problems
        .iter()
        .any(|problem| problem.message.contains("nothing reads 'paper'")));
}

#[test]
fn an_early_voting_policy_the_platform_does_not_know_is_refused() {
    // Two values, and neither is the one somebody guesses. An existing builder
    // test overrides `area.hbs` with `early_voting_allowed` — a plausible
    // rearrangement of the real `allow_early_voting` — and until now nothing
    // anywhere would have refused it. The Voting Portal compares the string
    // exactly, so a near miss reads as "no early voting" in silence.
    let mut bundle = sound();
    bundle.election_event.voting_channels =
        Some(serde_json::json!({"online": true, "early_voting": true}));
    bundle.areas[1].presentation =
        Some(serde_json::json!({"allow_early_voting": "early_voting_allowed"}));

    let report = validate(&bundle);
    assert!(report.has_errors(), "{report}");
    assert!(report.problems.iter().any(|problem| problem
        .message
        .contains("is not a valid allow_early_voting")));
}

#[test]
fn an_election_whose_channels_differ_from_the_events_is_reported() {
    // No per-election channel editor exists anywhere in the platform, so a bundle
    // in this state was written by hand — and the Publish screen reads the block
    // off whichever record it is showing, meaning the Kiosk control appears at one
    // level and not the other with nothing to explain why.
    let mut bundle = sound();
    bundle.election_event.voting_channels =
        Some(serde_json::json!({"online": true, "kiosk": true}));
    bundle.elections[0].voting_channels =
        Some(serde_json::json!({"online": true, "kiosk": false}));

    let report = validate(&bundle);
    assert!(!report.has_errors(), "{report}");
    assert!(report.problems.iter().any(|problem| problem
        .message
        .contains("kiosk differs from the event")));
}

#[test]
fn a_channel_named_at_one_level_and_omitted_at_the_other_is_not_a_difference() {
    // `election_event.hbs` names three channels and `election.hbs` names four, so
    // comparing which keys are present rather than which are on would report every
    // ordinary build as inconsistent about `telephone`. The first version of the
    // check above did exactly that.
    let mut bundle = sound();
    bundle.election_event.voting_channels =
        Some(serde_json::json!({"online": true, "kiosk": false}));
    bundle.elections[0].voting_channels = Some(
        serde_json::json!({"online": true, "kiosk": false, "telephone": false}),
    );

    let report = validate(&bundle);
    assert!(
        !report
            .problems
            .iter()
            .any(|problem| problem.message.contains("differs from the event")),
        "{report}"
    );
}

// -- images, and the two references that have to agree ----------------------

const DOC: &str = "d9000000-0000-5000-8000-000000000000";

/// A candidate photograph as the platform stores one.
fn with_image(url: &str) -> ImportElectionEventSchema {
    let mut bundle = sound();
    bundle.candidates[0].image_document_id = Some(DOC.into());
    bundle.candidates[0].presentation = Some(serde_json::json!({
        "urls": [{"url": url, "is_image": true}],
    }));
    bundle
}

#[test]
fn a_photograph_whose_two_references_agree_is_accepted() {
    // Images travel inside the archive. An earlier version of this file asserted
    // the opposite, on the grounds that `replace_uuids` rewrites every identifier
    // and `ImportElectionEventSchema` has no `documents` array — both true, and
    // both beside the point, because the bytes travel as a **zip entry** rather
    // than in the JSON. `replace_ids` returns the `old -> new` map, the `images/`
    // branch hands that same map to `process_s3_file`, and the document is created
    // with the new identifier. So the two references and the file move together.
    let report =
        validate(&with_image(&format!("tenant-x/document-{DOC}/a.png")));
    assert!(!report.has_errors(), "{report}");
    assert!(
        !report
            .problems
            .iter()
            .any(|problem| problem.path.starts_with("candidates[0]")),
        "{report}"
    );
}

#[test]
fn a_ballot_pointing_at_a_different_document_is_refused() {
    // The one that genuinely breaks. Import rewrites both references through one
    // map, which keeps them together only if they were the same string to start
    // with — so a url naming a different identifier ends up pointing somewhere
    // else entirely.
    let other = "dabcdef0-0000-5000-8000-000000000000";
    let report =
        validate(&with_image(&format!("tenant-x/document-{other}/a.png")));
    assert!(report.has_errors(), "{report}");
    assert!(report
        .problems
        .iter()
        .any(|problem| problem.message.contains("point at different files")));
}

#[test]
fn a_ballot_picture_with_no_document_named_is_reported_but_allowed() {
    // The ballot works: the url is the reference a voter's ballot reads. What is
    // lost is the Admin Portal's ability to change or remove it later, since that
    // is what `image_document_id` is for.
    let mut bundle = with_image(&format!("tenant-x/document-{DOC}/a.png"));
    bundle.candidates[0].image_document_id = None;

    let report = validate(&bundle);
    assert!(!report.has_errors(), "{report}");
    assert!(report
        .problems
        .iter()
        .any(|problem| problem.message.contains("changed or removed")));
}

#[test]
fn a_document_with_no_ballot_entry_is_reported_but_allowed() {
    // Worse in practice: nothing renders `image_document_id`, so the picture is
    // uploaded and appears on no ballot.
    let mut bundle = sound();
    bundle.candidates[0].image_document_id = Some(DOC.into());

    let report = validate(&bundle);
    assert!(!report.has_errors(), "{report}");
    assert!(report
        .problems
        .iter()
        .any(|problem| problem.message.contains("never shown")));
}

#[test]
fn a_url_that_is_not_an_image_is_left_alone() {
    // `presentation.urls` is not only for pictures — `getLinkUrl` looks one up by
    // title, for a candidate's own page. No document, nothing to agree with.
    let mut bundle = sound();
    bundle.candidates[0].presentation = Some(serde_json::json!({
        "urls": [{"url": "https://example.org/alice", "title": "URL", "is_image": false}],
    }));

    let report = validate(&bundle);
    assert!(
        !report
            .problems
            .iter()
            .any(|problem| problem.path.starts_with("candidates[0]")),
        "{report}"
    );
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

/// A policy the Admin Portal does not know imports without complaint and then
/// behaves as whatever the voting portal falls back to. Both hand-written
/// mappings that fed this format emitted at least one such value.
#[test]
fn a_contest_with_a_policy_the_platform_does_not_have_is_refused() {
    let mut bundle = sound();
    bundle.contests[0].presentation = Some(serde_json::json!({
        // `EUnderVotePolicy` has no `not-allowed` — an under-vote cannot be
        // refused, only warned about.
        "under_vote_policy": "not-allowed"
    }));

    let report = validate(&bundle);
    assert!(report.has_errors());
    assert!(report.problems.iter().any(|problem| problem
        .message
        .contains("is not a valid under_vote_policy")));
}

#[test]
fn a_contest_with_no_presentation_policies_is_fine() {
    // The platform has its own default for each; a bundle need not state one.
    let mut bundle = sound();
    bundle.contests[0].presentation = Some(serde_json::json!({}));
    assert!(!validate(&bundle).has_errors());
}

#[test]
fn a_tie_breaking_policy_the_platform_does_not_have_is_refused() {
    // `ITieBreakingPolicy` has two values. A third imports cleanly and then the
    // tally settles a tie by whatever its fallback is — which is a result nobody
    // chose, on the one question most likely to be challenged.
    let mut bundle = sound();
    bundle.contests[0].tally_configuration = Some(serde_json::json!({
        "tie_breaking_policy": "coin-toss"
    }));

    let report = validate(&bundle);
    assert!(report.has_errors());
    assert!(report.problems.iter().any(|problem| problem
        .message
        .contains("is not a tie-breaking policy")));
    // On the path the wizard can route to, nested where the contest keeps it.
    assert!(report.problems.iter().any(|problem| problem
        .path
        .ends_with("tally_configuration.tie_breaking_policy")));
}

#[test]
fn either_real_tie_breaking_policy_is_accepted() {
    for value in ["random", "external-procedure"] {
        let mut bundle = sound();
        bundle.contests[0].tally_configuration =
            Some(serde_json::json!({"tie_breaking_policy": value}));
        assert!(!validate(&bundle).has_errors(), "{value} was refused");
    }
}

#[test]
fn a_contest_laid_out_in_no_columns_is_refused() {
    let mut bundle = sound();
    bundle.contests[0].presentation = Some(serde_json::json!({"columns": 0}));

    let report = validate(&bundle);
    assert!(report.has_errors());
    assert!(report
        .problems
        .iter()
        .any(|problem| problem.message.contains("is not a layout")));
}

#[test]
fn a_contest_in_too_many_columns_is_a_warning_rather_than_a_refusal() {
    // It renders. It renders unusably on a phone, which is how most voters vote,
    // and that is a judgement about the electorate rather than about the bundle
    // — so it is said and not enforced.
    let mut bundle = sound();
    bundle.contests[0].presentation = Some(serde_json::json!({"columns": 6}));

    let report = validate(&bundle);
    assert!(!report.has_errors());
    assert!(report
        .problems
        .iter()
        .any(|problem| problem.message.contains("unreadable on a phone")));
}

#[test]
fn a_per_type_cap_that_can_never_bind_is_pointed_out() {
    // Somebody who sets a cap of five in a contest where a voter may choose
    // three believes a limit is in force that never applies.
    // The fixture's own maximum, plus one. Raising `max_votes` instead would
    // trip the arithmetic rules — a contest may not let a voter choose more
    // candidates than are standing — and the failure would be about that.
    let mut bundle = sound();
    let max = bundle.contests[0].max_votes.unwrap_or(1);
    bundle.contests[0].presentation =
        Some(serde_json::json!({"max_selections_per_type": max + 1}));

    let report = validate(&bundle);
    assert!(!report.has_errors());
    assert!(report
        .problems
        .iter()
        .any(|problem| problem.message.contains("never applies")));
}

#[test]
fn a_collapsible_list_setting_the_platform_does_not_have_is_refused() {
    let mut bundle = sound();
    bundle.contests[0].presentation =
        Some(serde_json::json!({"collapsible_lists": "enabled"}));

    let report = validate(&bundle);
    assert!(report.has_errors());
    assert!(report.problems.iter().any(|problem| problem
        .message
        .contains("is not a valid collapsible_lists")));
}

#[test]
fn an_ordinary_one_column_contest_passes() {
    let mut bundle = sound();
    bundle.contests[0].presentation = Some(serde_json::json!({
        "columns": 1,
        "collapsible_lists": "disabled",
        "enable_checkable_lists": "disabled",
        "max_selections_per_type": 0
    }));
    assert!(validate(&bundle).problems.is_empty());
}

#[test]
fn write_ins_allowed_with_nowhere_to_type_are_refused() {
    // Provable from the codec rather than guessed: `contest_context::bases`
    // reserves an encoding slot per candidate marked `is_write_in`, and only
    // then. The switch on with no such candidate reserves nothing, so a voter is
    // offered a feature with nowhere to put it — and nothing downstream refuses
    // the bundle.
    let mut bundle = sound();
    bundle.contests[0].presentation =
        Some(serde_json::json!({"allow_writeins": true}));

    let report = validate(&bundle);
    assert!(report.has_errors());
    assert!(report
        .problems
        .iter()
        .any(|problem| problem.message.contains("nowhere to type a name")));
}

#[test]
fn a_write_in_slot_on_a_contest_that_forbids_them_is_refused() {
    // The other half. A marked candidate with the switch off gets no encoding
    // slot either, and appears on the ballot as an ordinary option nobody named.
    let mut bundle = sound();
    let contest_id = bundle.contests[0].id.clone();
    let mut slot = bundle.candidates[0].clone();
    slot.id = "write-in-1".to_string();
    slot.contest_id = Some(contest_id);
    slot.presentation = Some(serde_json::json!({"is_write_in": true}));
    bundle.candidates.push(slot);

    let report = validate(&bundle);
    assert!(report.has_errors());
    assert!(report
        .problems
        .iter()
        .any(|problem| problem.message.contains("does not allow write-ins")));
}

#[test]
fn a_contest_with_both_halves_agreeing_passes() {
    let mut bundle = sound();
    let contest_id = bundle.contests[0].id.clone();
    bundle.contests[0].presentation =
        Some(serde_json::json!({"allow_writeins": true}));
    let mut slot = bundle.candidates[0].clone();
    slot.id = "write-in-1".to_string();
    slot.contest_id = Some(contest_id);
    slot.presentation = Some(serde_json::json!({"is_write_in": true}));
    bundle.candidates.push(slot);

    assert!(!validate(&bundle).has_errors());
}

#[test]
fn a_write_in_slot_is_not_somebody_standing() {
    // The arithmetic rules ask whether enough candidates are available to fill
    // the seats. A blank line is not a candidate, so counting it would let a
    // contest with one real candidate and two write-in slots claim it can elect
    // three people.
    let mut bundle = sound();
    let contest_id = bundle.contests[0].id.clone();
    bundle.contests[0].presentation =
        Some(serde_json::json!({"allow_writeins": true}));
    bundle.contests[0].winning_candidates_num =
        Some(bundle.candidates.len() as i64 + 1);

    let mut slot = bundle.candidates[0].clone();
    slot.id = "write-in-1".to_string();
    slot.contest_id = Some(contest_id);
    slot.presentation = Some(serde_json::json!({"is_write_in": true}));
    bundle.candidates.push(slot);

    let report = validate(&bundle);
    assert!(
        report.has_errors(),
        "a write-in slot was counted as a candidate"
    );
}

/// Zero casts means unlimited, not none.
///
/// The field is called `num_allowed_revotes` and the number is *casts*. The
/// Voting Portal is unambiguous — `castVotes.length < num_allowed_revotes`, with
/// "If num_allowed_revotes is 0, allow voting" above it — so zero is unlimited.
///
/// This test exists because the first version of the check read the name instead
/// of the portal and refused zero as "an election nobody can vote in". An
/// existing build fixture caught it, which is the only reason it did not ship.
#[test]
fn unlimited_revoting_is_not_an_error() {
    let mut bundle = sound();
    bundle.elections[0].num_allowed_revotes = Some(0);
    assert!(!validate(&bundle).has_errors());
}

#[test]
fn a_negative_number_of_votes_is_refused() {
    let mut bundle = sound();
    bundle.elections[0].num_allowed_revotes = Some(-1);
    let report = validate(&bundle);
    assert!(report.has_errors());
    assert!(report
        .problems
        .iter()
        .any(|problem| problem.message.contains("not a number of votes")));
}

#[test]
fn spoiling_a_ballot_with_no_second_cast_is_pointed_out() {
    // The voter discards their only vote and cannot replace it, which is not what
    // anybody ticking the box intended.
    let mut bundle = sound();
    bundle.elections[0].num_allowed_revotes = Some(1);
    bundle.elections[0].spoil_ballot_option = Some(true);

    let report = validate(&bundle);
    assert!(!report.has_errors());
    assert!(report.problems.iter().any(|problem| problem
        .message
        .contains("no second attempt to replace it")));
}

#[test]
fn spoiling_is_fine_where_there_is_another_cast_to_make() {
    for casts in [0, 2] {
        let mut bundle = sound();
        bundle.elections[0].num_allowed_revotes = Some(casts);
        bundle.elections[0].spoil_ballot_option = Some(true);
        assert!(
            !validate(&bundle)
                .problems
                .iter()
                .any(|problem| problem.message.contains("no second attempt")),
            "{casts} casts should allow spoiling"
        );
    }
}

#[test]
fn a_grace_period_policy_the_platform_does_not_have_is_refused() {
    let mut bundle = sound();
    bundle.elections[0].presentation = Some(
        serde_json::json!({"grace_period_policy": "grace-period-with-alert"}),
    );

    let report = validate(&bundle);
    assert!(report.has_errors());
    assert!(report.problems.iter().any(|problem| problem
        .message
        .contains("is not a valid grace_period_policy")));
}

#[test]
fn a_grace_period_of_no_seconds_is_pointed_out() {
    // Somebody set the policy and left the length at zero, so they believe voting
    // stays open a little longer than it does — found by a voter at one minute
    // past the close.
    let mut bundle = sound();
    bundle.elections[0].presentation = Some(serde_json::json!({
        "grace_period_policy": "grace-period-without-alert",
        "grace_period_secs": 0
    }));

    let report = validate(&bundle);
    assert!(!report.has_errors());
    assert!(report
        .problems
        .iter()
        .any(|problem| problem.message.contains("no grace period")));
}

#[test]
fn seconds_of_grace_with_no_grace_period_are_pointed_out() {
    // The same mistake the other way round.
    let mut bundle = sound();
    bundle.elections[0].presentation = Some(serde_json::json!({
        "grace_period_policy": "no-grace-period",
        "grace_period_secs": 300
    }));

    let report = validate(&bundle);
    assert!(!report.has_errors());
    assert!(report.problems.iter().any(|problem| problem
        .message
        .contains("voting closes on the deadline")));
}

#[test]
fn a_real_grace_period_passes() {
    let mut bundle = sound();
    bundle.elections[0].presentation = Some(serde_json::json!({
        "grace_period_policy": "grace-period-without-alert",
        "grace_period_secs": 300,
        "start_screen_title_policy": "election"
    }));
    assert!(validate(&bundle).problems.is_empty());
}

#[test]
fn an_ordering_the_platform_does_not_have_is_refused() {
    // The same three values in three places — the event's `elections_order`, an
    // election's `contests_order`, a contest's `candidates_order` — because
    // `ui-core` sorts all three through one helper. A fourth value would be
    // ignored rather than refused, which is the quiet kind of wrong.
    let mut bundle = sound();
    bundle.election_event.presentation =
        Some(serde_json::json!({"elections_order": "by-date"}));

    let report = validate(&bundle);
    assert!(report.has_errors());
    assert!(report.problems.iter().any(|problem| problem
        .message
        .contains("is not a valid elections_order")));

    let mut other = sound();
    other.elections[0].presentation =
        Some(serde_json::json!({"contests_order": "by-date"}));
    assert!(validate(&other).has_errors());
}

#[test]
fn every_real_ordering_is_accepted_in_all_three_places() {
    for value in ["custom", "alphabetical", "random"] {
        let mut bundle = sound();
        bundle.election_event.presentation =
            Some(serde_json::json!({"elections_order": value}));
        bundle.elections[0].presentation =
            Some(serde_json::json!({"contests_order": value}));
        bundle.contests[0].presentation =
            Some(serde_json::json!({"candidates_order": value}));
        assert!(!validate(&bundle).has_errors(), "{value} was refused");
    }
}

#[test]
fn a_cast_vote_logs_policy_the_platform_does_not_have_is_refused() {
    // Whether a voter can look up a ballot they cast, on the portal's
    // ballot-locator page. Two values; a third shows no tab and says nothing.
    let mut bundle = sound();
    bundle.election_event.presentation =
        Some(serde_json::json!({"show_cast_vote_logs": "show"}));

    let report = validate(&bundle);
    assert!(report.has_errors());
    assert!(report.problems.iter().any(|problem| problem
        .message
        .contains("is not a valid show_cast_vote_logs")));
}

#[test]
fn a_voter_can_look_up_their_ballot_unless_asked_otherwise() {
    for value in ["show-logs-tab", "hide-logs-tab"] {
        let mut bundle = sound();
        bundle.election_event.presentation =
            Some(serde_json::json!({"show_cast_vote_logs": value}));
        assert!(!validate(&bundle).has_errors(), "{value} was refused");
    }
}

/// The picker's list is a subset of the validator's, and each is still real.
///
/// The failure this prevents is a rename. Somebody corrects a spelling in
/// `COUNTING_ALGORITHMS` — where it matters, because the importer reads it — and the
/// wizard's dropdown then offers a value nothing accepts, which is exactly what
/// `INV-8` exists to stop. The other direction is deliberate and not asserted: the
/// offered list is *meant* to be shorter.
#[test]
fn every_algorithm_a_dropdown_offers_is_one_the_platform_accepts() {
    for offered in super::validate::OFFERED_COUNTING_ALGORITHMS {
        assert!(
            super::validate::COUNTING_ALGORITHMS.contains(offered),
            "the wizard offers {offered}, which is not a counting algorithm"
        );
    }
}

/// A plan naming one of the four still validates, still builds, still counts.
///
/// Narrowing what a dropdown offers must not narrow what the platform takes: a
/// client whose rules name `desborda2` has a plan somebody wrote by hand or through
/// `step-cli`, and turning that into a validation error would break a real election
/// to tidy a menu.
#[test]
fn an_algorithm_the_wizard_does_not_offer_is_still_a_valid_plan() {
    for hidden in ["borda-mas-madrid", "desborda", "desborda2", "desborda3"] {
        assert!(
            !super::validate::OFFERED_COUNTING_ALGORITHMS.contains(&hidden),
            "{hidden} is offered, so this test is asserting nothing"
        );
        assert!(
            super::validate::COUNTING_ALGORITHMS.contains(&hidden),
            "{hidden} is not a counting algorithm at all"
        );
        // And it is one of the ranked ones, which is what decides ballot encoding —
        // so a plan naming it needs `preferential`, and that pair is what
        // `check_tally` refuses to get wrong.
        assert!(
            super::validate::PREFERENTIAL_ALGORITHMS.contains(&hidden),
            "{hidden} is not preferential, so the pairing rule differs"
        );
    }
}
