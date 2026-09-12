// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Small election fixtures shared by the boundary tests. Defaults describe a
//! two-seat plurality contest; each test states the marks and weights it changes.

#![allow(dead_code)] // Each integration binary uses a different subset.

use sequent_core::ballot::{Candidate, CandidatePresentation, Contest, Weight};
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
use sequent_core::types::ceremonies::{CountingAlgType, ScopeOperation, TallyOperation};
use velvet::pipes::do_tally::tally::Tally;
use velvet::pipes::do_tally::ContestResult;

pub fn candidate(id: &str, name: &str) -> Candidate {
    Candidate {
        id: id.into(),
        name: Some(name.into()),
        contest_id: "council".into(),
        ..Default::default()
    }
}

pub fn contest() -> Contest {
    let mut blank = candidate("blank", "Explicit blank");
    blank.presentation = Some(CandidatePresentation {
        is_explicit_blank: Some(true),
        ..Default::default()
    });
    let mut invalid = candidate("invalid", "Explicit invalid");
    invalid.presentation = Some(CandidatePresentation {
        is_explicit_invalid: Some(true),
        ..Default::default()
    });

    Contest {
        id: "council".into(),
        min_votes: 0,
        max_votes: 2,
        winning_candidates_num: 2,
        counting_algorithm: Some(CountingAlgType::PluralityAtLarge),
        candidates: vec![
            candidate("ada", "Ada"),
            candidate("bea", "Bea"),
            candidate("cam", "Cam"),
            blank,
            invalid,
        ],
        ..Default::default()
    }
}

pub fn ballot(selected: &[&str]) -> DecodedVoteContest {
    DecodedVoteContest {
        contest_id: "council".into(),
        is_explicit_invalid: false,
        is_decline_to_vote: false,
        is_blank_ballot: false,
        invalid_errors: vec![],
        invalid_alerts: vec![],
        choices: selected
            .iter()
            .map(|id| DecodedVoteChoice {
                id: (*id).into(),
                selected: 0,
                write_in_text: None,
            })
            .collect(),
    }
}

pub fn weight(value: u64) -> Weight {
    serde_json::from_value(serde_json::json!(value)).expect("a nonnegative area weight")
}

pub fn tally(ballots: Vec<(DecodedVoteContest, Weight)>) -> Tally {
    Tally {
        id: CountingAlgType::PluralityAtLarge,
        scope_operation: ScopeOperation::Area(TallyOperation::ProcessBallotsAll),
        contest: contest(),
        ballots,
        census: 10,
        auditable_votes: 1,
        tally_sheet_results: vec![],
        tally_results: vec![],
    }
}

/// Compare by candidate identity; aggregation is allowed to change vector order.
pub fn candidate_counts(result: &ContestResult) -> std::collections::BTreeMap<String, u64> {
    result
        .candidate_result
        .iter()
        .map(|entry| (entry.candidate.id.clone(), entry.total_count))
        .collect()
}

pub fn candidate_percentage(result: &ContestResult, id: &str) -> f64 {
    result
        .candidate_result
        .iter()
        .find(|entry| entry.candidate.id == id)
        .expect("candidate is retained even when its count is zero")
        .percentage_votes
}

pub fn assert_percentage(actual: f64, expected: f64) {
    assert!(
        actual.is_finite() && (actual - expected).abs() < 1e-10,
        "expected {expected}%, got {actual}%"
    );
}
