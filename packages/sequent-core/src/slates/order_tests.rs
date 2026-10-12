// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::ballot::{CandidatePresentation, ContestPresentation};
use crate::slates::problem::Severity;
use crate::slates::{MobileCandidateLists, Slate, SLATES_VERSION};
use std::collections::BTreeMap;

const PRESIDENT: &str = "president";
const TRUSTEES: &str = "trustees";

/// A candidate listed at `sort_order` in a contest with a custom order.
fn candidate(id: &str, contest_id: &str, sort_order: i64) -> Candidate {
    Candidate {
        id: id.to_string(),
        contest_id: contest_id.to_string(),
        name: Some(id.to_string()),
        presentation: Some(CandidatePresentation {
            sort_order: Some(sort_order),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn contest(id: &str, max_votes: i64, candidate_ids: &[&str]) -> Contest {
    Contest {
        id: id.to_string(),
        name: Some(id.to_string()),
        max_votes,
        candidates: candidate_ids
            .iter()
            .enumerate()
            .map(|(index, candidate_id)| {
                candidate(candidate_id, id, i64::try_from(index).unwrap())
            })
            .collect(),
        presentation: Some(ContestPresentation {
            candidates_order: Some(CandidatesOrder::Custom),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn slate(id: &str, members: &[(&str, &[&str])]) -> Slate {
    Slate {
        id: id.to_string(),
        name: BTreeMap::from([("en".to_string(), id.to_string())]),
        members: members
            .iter()
            .map(|(contest_id, candidates)| {
                (
                    (*contest_id).to_string(),
                    candidates.iter().map(|id| (*id).to_string()).collect(),
                )
            })
            .collect(),
    }
}

fn config(slates: Vec<Slate>) -> SlatesConfig {
    SlatesConfig {
        version: SLATES_VERSION,
        mobile_candidate_lists: MobileCandidateLists::default(),
        slates,
    }
}

fn united() -> Slate {
    slate(
        "united",
        &[
            (PRESIDENT, &["p-united"]),
            (TRUSTEES, &["t-united-1", "t-united-2"]),
        ],
    )
}

fn reform() -> Slate {
    slate(
        "reform",
        &[
            (PRESIDENT, &["p-reform"]),
            (TRUSTEES, &["t-reform-1", "t-reform-2"]),
        ],
    )
}

fn voices() -> Slate {
    slate("voices", &[(TRUSTEES, &["t-voices-1"])])
}

/// Full slates, then the partial one, then independents, in each contest.
fn contests() -> Vec<Contest> {
    vec![
        contest(PRESIDENT, 1, &["p-united", "p-reform", "p-independent"]),
        contest(
            TRUSTEES,
            2,
            &[
                "t-united-1",
                "t-united-2",
                "t-reform-1",
                "t-reform-2",
                "t-voices-1",
                "t-independent",
            ],
        ),
    ]
}

fn paths(problems: &[Problem]) -> Vec<&str> {
    problems
        .iter()
        .map(|problem| problem.path.as_str())
        .collect()
}

#[test]
fn the_mandated_listing_order_raises_nothing() {
    let problems = check_listing_order(
        &config(vec![united(), reform(), voices()]),
        &contests(),
        "slates",
    );

    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn a_partial_slate_before_a_full_one_is_a_warning() {
    let mut contests = contests();
    // Keep the candidates in slate order, so only the slate order is wrong.
    contests[1].candidates = contest(
        TRUSTEES,
        2,
        &[
            "t-voices-1",
            "t-united-1",
            "t-united-2",
            "t-reform-1",
            "t-reform-2",
            "t-independent",
        ],
    )
    .candidates;

    let problems = check_listing_order(
        &config(vec![voices(), united(), reform()]),
        &contests,
        "slates",
    );

    assert_eq!(paths(&problems), ["slates.slates[1]", "slates.slates[2]"]);
    assert!(problems.iter().all(|problem| {
        problem.severity == Severity::Warning
            && problem.code == Code::ListingOrder
    }));
    assert!(problems[0].message.contains("'united'"));
    assert!(problems[0].message.contains("'voices'"));
}

#[test]
fn an_independent_before_slate_members_is_a_warning() {
    let mut contests = contests();
    contests[0].candidates =
        contest(PRESIDENT, 1, &["p-independent", "p-united", "p-reform"])
            .candidates;

    let problems = check_listing_order(
        &config(vec![united(), reform(), voices()]),
        &contests,
        "slates",
    );

    assert_eq!(paths(&problems), ["slates.contests[\"president\"]"]);
    assert_eq!(problems[0].severity, Severity::Warning);
    assert!(problems[0]
        .message
        .contains("independent candidate 'p-independent'"));
}

#[test]
fn candidates_out_of_slate_order_are_a_warning() {
    let mut contests = contests();
    contests[0].candidates =
        contest(PRESIDENT, 1, &["p-reform", "p-united", "p-independent"])
            .candidates;

    let problems = check_listing_order(
        &config(vec![united(), reform(), voices()]),
        &contests,
        "slates",
    );

    assert_eq!(paths(&problems), ["slates.contests[\"president\"]"]);
    assert!(problems[0].message.contains("'p-reform' of slate 'reform'"));
}

#[test]
fn the_check_follows_the_order_the_voter_sees() {
    let mut contests = contests();
    // Listed in the right order, but sorted by name on the ballot, which puts
    // "p-independent" first.
    contests[0].presentation = Some(ContestPresentation {
        candidates_order: Some(CandidatesOrder::Alphabetical),
        ..Default::default()
    });

    let problems = check_listing_order(
        &config(vec![united(), reform(), voices()]),
        &contests,
        "slates",
    );

    assert_eq!(paths(&problems), ["slates.contests[\"president\"]"]);
}

#[test]
fn a_random_candidate_order_is_a_warning() {
    let mut contests = contests();
    contests[1].presentation = Some(ContestPresentation {
        candidates_order: Some(CandidatesOrder::Random),
        ..Default::default()
    });

    let problems = check_listing_order(
        &config(vec![united(), reform(), voices()]),
        &contests,
        "slates",
    );

    assert_eq!(paths(&problems), ["slates.contests[\"trustees\"]"]);
    assert!(problems[0].message.contains("random order"));
}

#[test]
fn contests_without_slate_members_are_not_checked() {
    let mut contests = contests();
    contests.push(contest("auditor", 1, &["a-two", "a-one"]));
    contests[2].presentation = Some(ContestPresentation {
        candidates_order: Some(CandidatesOrder::Random),
        ..Default::default()
    });

    let problems = check_listing_order(
        &config(vec![united(), reform(), voices()]),
        &contests,
        "slates",
    );

    // "auditor" has no slate member: it is a partial-slate gap for every
    // slate, but not a listing problem.
    assert!(paths(&problems)
        .iter()
        .all(|path| !path.contains("auditor")));
}
