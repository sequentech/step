// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::ballot::{Candidate, CandidatePresentation};
use crate::election_config::slates::parse;
use serde_json::json;

const SINGLE_SEAT_OFFICES: [&str; 4] =
    ["president", "vp", "treasurer", "secretary"];
const TRUSTEES: &str = "trustees";

fn candidate(contest_id: &str, id: &str) -> Candidate {
    Candidate {
        id: id.to_string(),
        contest_id: contest_id.to_string(),
        ..Default::default()
    }
}

fn contest(id: &str, max_votes: i64, candidates: &[&str]) -> Contest {
    Contest {
        id: id.to_string(),
        max_votes,
        candidates: candidates
            .iter()
            .map(|candidate_id| candidate(id, candidate_id))
            .collect(),
        ..Default::default()
    }
}

/// Five contests electing seven people.
fn contests() -> Vec<Contest> {
    let mut contests: Vec<Contest> = SINGLE_SEAT_OFFICES
        .iter()
        .map(|office| {
            contest(
                office,
                1,
                &[
                    &format!("f-{office}"),
                    &format!("m-{office}"),
                    &format!("i-{office}"),
                ],
            )
        })
        .collect();
    contests.push(contest(
        TRUSTEES,
        3,
        &[
            "f-t1", "f-t2", "f-t3", "m-t1", "m-t2", "m-t3", "v-t1", "v-t2",
            "v-t3", "i-t1",
        ],
    ));
    contests
}

fn full_slate(id: &str, prefix: &str) -> serde_json::Value {
    json!({
        "id": id,
        "name": {"en": id},
        "members": {
            "president": [format!("{prefix}-president")],
            "vp": [format!("{prefix}-vp")],
            "treasurer": [format!("{prefix}-treasurer")],
            "secretary": [format!("{prefix}-secretary")],
            "trustees": [
                format!("{prefix}-t1"),
                format!("{prefix}-t2"),
                format!("{prefix}-t3")
            ]
        }
    })
}

fn config() -> SlatesConfig {
    let document = json!({
        "version": 1,
        "slates": [
            full_slate("forward", "f"),
            full_slate("members", "m"),
            {
                "id": "voices",
                "name": {"en": "Independent Voices"},
                "members": {"trustees": ["v-t1", "v-t2", "v-t3"]}
            }
        ]
    });
    parse(&document.to_string(), "slates").expect("a valid configuration")
}

fn slate(members: serde_json::Value) -> Slate {
    serde_json::from_value(json!({
        "id": "slate",
        "name": {"en": "Slate"},
        "members": members
    }))
    .expect("a slate")
}

#[test]
fn a_slate_filling_every_seat_is_complete() {
    let coverage = slates_coverage(&config(), &contests());

    assert_eq!(coverage.len(), 3);
    for full in &coverage[..2] {
        assert_eq!(full.kind, CoverageKind::Complete);
        assert_eq!(full.covered.len(), 5);
        assert!(full.uncovered_contest_ids.is_empty());
        assert_eq!(full.members, 7);
        assert_eq!(full.seats, 7);
    }
}

#[test]
fn a_trustee_only_slate_is_partial_and_names_the_offices_it_leaves() {
    let coverage = slates_coverage(&config(), &contests());
    let voices = &coverage[2];

    assert_eq!(voices.slate_id, "voices");
    assert_eq!(voices.kind, CoverageKind::Partial);
    assert_eq!(
        voices.covered,
        vec![ContestCoverage {
            contest_id: TRUSTEES.to_string(),
            candidate_ids: vec![
                "v-t1".to_string(),
                "v-t2".to_string(),
                "v-t3".to_string()
            ],
            seats: 3,
        }]
    );
    assert_eq!(voices.uncovered_contest_ids, SINGLE_SEAT_OFFICES);
    assert_eq!(voices.members, 3);
    assert_eq!(voices.seats, 7);
}

#[test]
fn a_slate_in_every_office_with_one_trustee_of_three_is_partial() {
    let slate = slate(json!({
        "president": ["f-president"],
        "vp": ["f-vp"],
        "treasurer": ["f-treasurer"],
        "secretary": ["f-secretary"],
        "trustees": ["f-t1"]
    }));

    let coverage =
        slate_coverage(&slate, &contests()).expect("a slate with members");

    assert_eq!(coverage.kind, CoverageKind::Partial);
    assert!(coverage.uncovered_contest_ids.is_empty());
    assert_eq!(coverage.members, 5);
    assert_eq!(coverage.seats, 7);
}

#[test]
fn covered_contests_follow_the_ballot_order() {
    let slate = slate(json!({
        "trustees": ["f-t1"],
        "president": ["f-president"]
    }));

    let coverage =
        slate_coverage(&slate, &contests()).expect("a slate with members");

    let covered: Vec<&str> = coverage
        .covered
        .iter()
        .map(|contest| contest.contest_id.as_str())
        .collect();
    assert_eq!(covered, ["president", TRUSTEES]);
    assert_eq!(
        coverage.uncovered_contest_ids,
        ["vp", "treasurer", "secretary"]
    );
}

#[test]
fn coverage_is_measured_against_the_contests_of_the_ballot_style() {
    let without_trustees: Vec<Contest> = contests()
        .into_iter()
        .filter(|contest| contest.id != TRUSTEES)
        .collect();

    let coverage = slates_coverage(&config(), &without_trustees);

    let slate_ids: Vec<&str> = coverage
        .iter()
        .map(|coverage| coverage.slate_id.as_str())
        .collect();
    assert_eq!(slate_ids, ["forward", "members"]);
    assert_eq!(coverage[0].kind, CoverageKind::Complete);
    assert_eq!(coverage[0].members, 4);
    assert_eq!(coverage[0].seats, 4);
}

#[test]
fn a_slate_complete_elsewhere_is_partial_where_an_office_is_added() {
    let only_trustees: Vec<Contest> = contests()
        .into_iter()
        .filter(|contest| contest.id == TRUSTEES)
        .collect();
    let voices = &config().slates[2];

    let narrow =
        slate_coverage(voices, &only_trustees).expect("a slate with members");
    let wide =
        slate_coverage(voices, &contests()).expect("a slate with members");

    assert_eq!(narrow.kind, CoverageKind::Complete);
    assert_eq!(wide.kind, CoverageKind::Partial);
}

#[test]
fn a_slate_without_candidates_in_the_ballot_style_has_no_coverage() {
    let voices = &config().slates[2];
    let without_trustees: Vec<Contest> = contests()
        .into_iter()
        .filter(|contest| contest.id != TRUSTEES)
        .collect();

    assert_eq!(slate_coverage(voices, &without_trustees), None);
    assert_eq!(slate_coverage(voices, &[]), None);
}

#[test]
fn an_acclaimed_contest_is_neither_covered_nor_missing() {
    let mut contests = contests();
    contests[0].is_acclaimed = Some(true);
    let forward = &config().slates[0];

    let coverage =
        slate_coverage(forward, &contests).expect("a slate with members");

    assert_eq!(coverage.kind, CoverageKind::Complete);
    assert_eq!(coverage.covered.len(), 4);
    assert!(coverage.uncovered_contest_ids.is_empty());
    assert_eq!(coverage.members, 6);
    assert_eq!(coverage.seats, 6);
}

#[test]
fn a_member_that_cannot_be_chosen_is_not_counted() {
    let mut contests = contests();
    contests[0].candidates[0].presentation = Some(CandidatePresentation {
        is_disabled: Some(true),
        ..Default::default()
    });
    let forward = &config().slates[0];

    let coverage =
        slate_coverage(forward, &contests).expect("a slate with members");

    assert_eq!(coverage.kind, CoverageKind::Partial);
    assert_eq!(coverage.uncovered_contest_ids, ["president"]);
    assert_eq!(coverage.members, 6);
}

#[test]
fn a_member_missing_from_its_contest_is_not_counted() {
    let slate = slate(json!({"trustees": ["v-t1", "nobody"]}));

    let coverage =
        slate_coverage(&slate, &contests()).expect("a slate with members");

    assert_eq!(coverage.covered[0].candidate_ids, ["v-t1"]);
    assert_eq!(coverage.members, 1);
}

#[test]
fn more_members_than_seats_is_never_complete() {
    let slate = slate(json!({"trustees": ["v-t1", "v-t2", "v-t3", "i-t1"]}));
    let only_trustees: Vec<Contest> = contests()
        .into_iter()
        .filter(|contest| contest.id == TRUSTEES)
        .collect();

    let coverage =
        slate_coverage(&slate, &only_trustees).expect("a slate with members");

    assert_eq!(coverage.kind, CoverageKind::Partial);
}

#[test]
fn coverage_serializes_with_lowercase_kinds() {
    let coverage = slates_coverage(&config(), &contests());

    let value = serde_json::to_value(&coverage[2]).expect("serializable");

    assert_eq!(value["kind"], "partial");
    assert_eq!(value["slate_id"], "voices");
    assert_eq!(value["covered"][0]["seats"], 3);
}
