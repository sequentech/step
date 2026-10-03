// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::ballot::{Candidate, CandidatePresentation};
use crate::plaintext::{InvalidPlaintextError, InvalidPlaintextErrorType};
use std::collections::{BTreeMap, HashMap};

const PRESIDENT: &str = "president";
const SECRETARY: &str = "secretary";
const TRUSTEES: &str = "trustees";

fn contest(id: &str, max_votes: i64, candidates: &[&str]) -> Contest {
    Contest {
        id: id.to_string(),
        max_votes,
        candidates: candidates
            .iter()
            .map(|candidate_id| Candidate {
                id: candidate_id.to_string(),
                contest_id: id.to_string(),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}

fn contests() -> Vec<Contest> {
    vec![
        contest(PRESIDENT, 1, &["p-forward", "p-members", "p-independent"]),
        contest(SECRETARY, 1, &["s-forward", "s-members"]),
        contest(
            TRUSTEES,
            3,
            &[
                "t-forward-1",
                "t-forward-2",
                "t-forward-3",
                "t-members-1",
                "t-voices-1",
                "t-voices-2",
                "t-independent",
            ],
        ),
    ]
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

fn forward() -> Slate {
    slate(
        "forward",
        &[
            (PRESIDENT, &["p-forward"]),
            (SECRETARY, &["s-forward"]),
            (TRUSTEES, &["t-forward-1", "t-forward-2", "t-forward-3"]),
        ],
    )
}

fn voices() -> Slate {
    slate("voices", &[(TRUSTEES, &["t-voices-1", "t-voices-2"])])
}

/// The selection of a voter who has marked nothing yet.
fn empty_selection() -> Vec<DecodedVoteContest> {
    contests()
        .iter()
        .map(|contest| DecodedVoteContest {
            contest_id: contest.id.clone(),
            is_explicit_invalid: false,
            is_decline_to_vote: false,
            is_blank_ballot: false,
            invalid_errors: vec![],
            invalid_alerts: vec![],
            choices: contest
                .candidates
                .iter()
                .map(|candidate| DecodedVoteChoice {
                    id: candidate.id.clone(),
                    selected: UNSELECTED,
                    write_in_text: None,
                })
                .collect(),
        })
        .collect()
}

/// Mark one candidate, as the voting screen does for an ordinary contest.
fn mark(selection: &mut [DecodedVoteContest], contest_id: &str, id: &str) {
    let choice = selection
        .iter_mut()
        .find(|entry| entry.contest_id == contest_id)
        .and_then(|entry| {
            entry.choices.iter_mut().find(|choice| choice.id == id)
        })
        .expect("a choice of the ballot");
    choice.selected = SELECTED;
}

fn selected(selection: &[DecodedVoteContest], contest_id: &str) -> Vec<String> {
    selection
        .iter()
        .find(|entry| entry.contest_id == contest_id)
        .expect("a contest of the ballot")
        .choices
        .iter()
        .filter(|choice| choice.is_selected())
        .map(|choice| choice.id.clone())
        .collect()
}

fn change(
    contest_id: &str,
    added: &[&str],
    removed: &[&str],
) -> SlateContestChange {
    SlateContestChange {
        contest_id: contest_id.to_string(),
        added: added.iter().map(|id| (*id).to_string()).collect(),
        removed: removed.iter().map(|id| (*id).to_string()).collect(),
    }
}

fn codes(problems: &[Problem]) -> Vec<Code> {
    problems.iter().map(|problem| problem.code).collect()
}

fn an_error() -> InvalidPlaintextError {
    InvalidPlaintextError {
        error_type: InvalidPlaintextErrorType::Implicit,
        candidate_id: None,
        message: Some("errors.implicit.underVote".to_string()),
        message_map: HashMap::new(),
    }
}

#[test]
fn a_full_slate_selects_every_member_and_nothing_else() {
    let choices = apply_slate(&forward(), &contests(), &empty_selection())
        .expect("the slate applies");

    assert_eq!(selected(&choices.selection, PRESIDENT), ["p-forward"]);
    assert_eq!(selected(&choices.selection, SECRETARY), ["s-forward"]);
    assert_eq!(
        selected(&choices.selection, TRUSTEES),
        ["t-forward-1", "t-forward-2", "t-forward-3"]
    );
    assert_eq!(
        choices.changes,
        [
            change(PRESIDENT, &["p-forward"], &[]),
            change(SECRETARY, &["s-forward"], &[]),
            change(
                TRUSTEES,
                &["t-forward-1", "t-forward-2", "t-forward-3"],
                &[]
            ),
        ]
    );
    assert!(!choices.removes_choices());
}

#[test]
fn choosing_a_slate_equals_marking_its_members_by_hand() {
    let by_slate = apply_slate(&forward(), &contests(), &empty_selection())
        .expect("the slate applies")
        .selection;

    let mut by_hand = empty_selection();
    mark(&mut by_hand, TRUSTEES, "t-forward-3");
    mark(&mut by_hand, PRESIDENT, "p-forward");
    mark(&mut by_hand, TRUSTEES, "t-forward-1");
    mark(&mut by_hand, SECRETARY, "s-forward");
    mark(&mut by_hand, TRUSTEES, "t-forward-2");

    assert_eq!(by_slate, by_hand);
}

#[test]
fn a_partial_slate_leaves_the_contests_it_does_not_cover() {
    let mut current = empty_selection();
    mark(&mut current, PRESIDENT, "p-independent");

    let choices = apply_slate(&voices(), &contests(), &current)
        .expect("the slate applies");

    assert_eq!(selected(&choices.selection, PRESIDENT), ["p-independent"]);
    assert!(selected(&choices.selection, SECRETARY).is_empty());
    assert_eq!(
        selected(&choices.selection, TRUSTEES),
        ["t-voices-1", "t-voices-2"]
    );
    assert_eq!(
        choices.changes,
        [change(TRUSTEES, &["t-voices-1", "t-voices-2"], &[])]
    );
}

#[test]
fn members_replace_the_marks_of_covered_contests() {
    let mut current = empty_selection();
    mark(&mut current, PRESIDENT, "p-independent");
    for id in ["t-independent", "t-members-1", "t-forward-1"] {
        mark(&mut current, TRUSTEES, id);
    }

    let choices = apply_slate(&forward(), &contests(), &current)
        .expect("the slate applies");

    for contest in contests() {
        let count = selected(&choices.selection, &contest.id).len();
        assert!(i64::try_from(count).expect("a count") <= contest.max_votes);
    }
    assert_eq!(
        choices.changes,
        [
            change(PRESIDENT, &["p-forward"], &["p-independent"]),
            change(SECRETARY, &["s-forward"], &[]),
            change(
                TRUSTEES,
                &["t-forward-2", "t-forward-3"],
                &["t-members-1", "t-independent"]
            ),
        ]
    );
    assert!(choices.removes_choices());
}

#[test]
fn a_slate_already_selected_changes_nothing() {
    let applied = apply_slate(&forward(), &contests(), &empty_selection())
        .expect("the slate applies");
    let again = apply_slate(&forward(), &contests(), &applied.selection)
        .expect("the slate applies");

    assert!(again.changes.is_empty());
    assert_eq!(again.selection, applied.selection);
}

#[test]
fn a_deselected_write_in_loses_its_text() {
    let mut current = empty_selection();
    current[0].choices[2] = DecodedVoteChoice {
        id: "p-independent".to_string(),
        selected: SELECTED,
        write_in_text: Some("Someone".to_string()),
    };

    let choices = apply_slate(&forward(), &contests(), &current)
        .expect("the slate applies");

    assert!(choices.selection[0]
        .choices
        .iter()
        .all(|choice| choice.write_in_text.is_none()));
}

#[test]
fn invalid_decline_and_blank_markers_are_cleared() {
    let mut current = empty_selection();
    for entry in &mut current {
        entry.is_explicit_invalid = entry.contest_id == TRUSTEES;
        entry.is_decline_to_vote = true;
        entry.is_blank_ballot = true;
        entry.invalid_errors = vec![an_error()];
    }

    let choices = apply_slate(&voices(), &contests(), &current)
        .expect("the slate applies");

    let trustees = &choices.selection[2];
    assert!(!trustees.is_explicit_invalid);
    assert!(trustees.invalid_errors.is_empty());
    assert!(choices
        .selection
        .iter()
        .all(|entry| !entry.is_blank_ballot && !entry.is_decline_to_vote));
    // A contest the slate does not cover keeps its own validation state.
    assert_eq!(choices.selection[0].invalid_errors, [an_error()]);
}

#[test]
fn a_cleared_explicit_invalid_mark_counts_as_removed() {
    let mut contests = contests();
    contests[0].candidates.push(Candidate {
        id: "p-invalid".to_string(),
        contest_id: PRESIDENT.to_string(),
        presentation: Some(CandidatePresentation {
            is_explicit_invalid: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    });
    let mut current = empty_selection();
    current[0].is_explicit_invalid = true;

    let choices = apply_slate(&forward(), &contests, &current)
        .expect("the slate applies");

    assert_eq!(
        choices.changes[0],
        change(PRESIDENT, &["p-forward"], &["p-invalid"])
    );
}

#[test]
fn contests_outside_the_ballot_are_skipped() {
    let ballot = &contests()[2..];
    let current = &empty_selection()[2..];

    let choices =
        apply_slate(&forward(), ballot, current).expect("the slate applies");

    assert_eq!(choices.selection.len(), 1);
    assert_eq!(
        selected(&choices.selection, TRUSTEES),
        ["t-forward-1", "t-forward-2", "t-forward-3"]
    );
}

#[test]
fn a_slate_with_nothing_on_the_ballot_is_refused() {
    let problems =
        apply_slate(&voices(), &contests()[..2], &empty_selection()[..2])
            .expect_err("nothing to select");

    assert_eq!(codes(&problems), [Code::MissingField]);
}

#[test]
fn a_selection_without_the_contest_or_candidate_is_refused() {
    let without_contest = &empty_selection()[..2];
    let problems = apply_slate(&forward(), &contests(), without_contest)
        .expect_err("the trustees are missing");
    assert_eq!(codes(&problems), [Code::DanglingReference]);

    let mut without_choice = empty_selection();
    without_choice[0].choices.remove(0);
    let problems = apply_slate(&forward(), &contests(), &without_choice)
        .expect_err("a member is missing");
    assert_eq!(codes(&problems), [Code::DanglingReference]);

    let mut repeated = empty_selection();
    repeated.push(repeated[2].clone());
    let problems = apply_slate(&forward(), &contests(), &repeated)
        .expect_err("the trustees are repeated");
    assert_eq!(codes(&problems), [Code::DanglingReference]);
}

#[test]
fn a_member_that_is_not_a_candidate_of_the_contest_is_refused() {
    let stranger = slate("stranger", &[(PRESIDENT, &["s-forward"])]);

    let problems = apply_slate(&stranger, &contests(), &empty_selection())
        .expect_err("not a candidate for president");

    assert_eq!(codes(&problems), [Code::DanglingReference]);
}

#[test]
fn a_slate_above_a_contest_maximum_is_refused() {
    let mut contests = contests();
    contests[2].max_votes = 2;

    let problems = apply_slate(&forward(), &contests, &empty_selection())
        .expect_err("three trustees where two are allowed");

    assert_eq!(codes(&problems), [Code::ContestArithmetic]);
}

#[test]
fn empty_and_repeated_members_are_refused() {
    let empty = slate("empty", &[(TRUSTEES, &[])]);
    let problems = apply_slate(&empty, &contests(), &empty_selection())
        .expect_err("no candidates");
    assert_eq!(codes(&problems), [Code::MissingField]);

    let repeated =
        slate("repeated", &[(TRUSTEES, &["t-voices-1", "t-voices-1"])]);
    let problems = apply_slate(&repeated, &contests(), &empty_selection())
        .expect_err("the same candidate twice");
    assert_eq!(codes(&problems), [Code::DuplicateId]);
}

#[test]
fn a_refused_slate_reports_every_problem_and_changes_nothing() {
    let mut contests = contests();
    contests[2].max_votes = 2;
    let broken = slate(
        "broken",
        &[
            (PRESIDENT, &["nobody"]),
            (TRUSTEES, &["t-forward-1", "t-forward-2", "t-forward-3"]),
        ],
    );
    let current = empty_selection();

    let problems = apply_slate(&broken, &contests, &current)
        .expect_err("two contests are wrong");

    assert_eq!(
        codes(&problems),
        [Code::DanglingReference, Code::ContestArithmetic]
    );
    assert_eq!(current, empty_selection());
}
