// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Ballot classification and per-ballot extended-metrics accumulators.
//!
//! Accumulates over-votes, under-votes, actual marks and expected marks
//! into `ExtendedMetricsContest`. How a ballot COUNTS is not decided here:
//! that is a validation rule and lives with the others, in
//! `sequent_core::validation` (`ContestValidator::classify`).
//!
//! Ported from velvet's `do_tally::counting_algorithm::utils`. The
//! area / ballot-style helpers (`get_contest_tally_operation`,
//! `get_area_tally_operation`, `get_area_weight`) stay in velvet: they
//! are pipeline concerns, not per-ballot counting.

use std::collections::HashSet;

use sequent_core::ballot::Contest;
use sequent_core::plaintext::DecodedVoteContest;
use tracing::instrument;

use crate::result::ExtendedMetricsContest;

pub fn get_explicit_blank_candidate_ids(contest: &Contest) -> HashSet<String> {
    contest
        .candidates
        .iter()
        .filter(|candidate| candidate.is_explicit_blank())
        .map(|candidate| candidate.id.clone())
        .collect()
}

fn is_explicit_blank_choice(
    choice_id: &str,
    explicit_blank_candidate_ids: &HashSet<String>,
) -> bool {
    explicit_blank_candidate_ids.contains(choice_id)
}

fn count_actual_votes(
    vote: &DecodedVoteContest,
    explicit_blank_candidate_ids: &HashSet<String>,
) -> u64 {
    vote.choices.iter().fold(0u64, |acc, choice| {
        if choice.selected > -1
            && !is_explicit_blank_choice(&choice.id, explicit_blank_candidate_ids)
        {
            acc + 1
        } else {
            acc
        }
    })
}

fn calculate_undervotes(actual_votes: u64, contest: &Contest) -> u64 {
    // Calculate undervotes based on max_votes
    let max_votes = contest.max_votes as u64;
    if actual_votes < max_votes {
        max_votes - actual_votes
    } else {
        0
    }
}

fn calculate_valid_votes(actual_votes: u64, contest: &Contest) -> u64 {
    // Check if votes are within valid range
    if actual_votes >= (contest.min_votes as u64) && actual_votes <= (contest.max_votes as u64) {
        actual_votes
    } else {
        0
    }
}

fn calculate_overvotes(actual_votes: u64, contest: &Contest) -> u64 {
    // Calculate overvotes if actual votes exceed max_votes
    if actual_votes > (contest.max_votes as u64) {
        actual_votes - (contest.max_votes as u64)
    } else {
        0
    }
}

#[instrument(skip_all)]
pub fn update_extended_metrics(
    vote: &DecodedVoteContest,
    current_metrics: &ExtendedMetricsContest,
    contest: &Contest,
    explicit_blank_candidate_ids: &HashSet<String>,
) -> ExtendedMetricsContest {
    let mut metrics = current_metrics.clone();

    // Count the actual (non marker) votes once; all derived metrics below
    // are computed from this count.
    let actual_votes = count_actual_votes(vote, explicit_blank_candidate_ids);

    // Calculate valid votes first
    let valid_votes = calculate_valid_votes(actual_votes, contest);
    metrics.votes_actually += valid_votes;

    // Calculate undervotes if not a decline to vote
    if !vote.is_decline_to_vote() {
        let undervotes = calculate_undervotes(actual_votes, contest);
        metrics.under_votes += undervotes;
    }

    // Calculate overvotes
    let overvotes = calculate_overvotes(actual_votes, contest);
    metrics.over_votes += overvotes;

    // Expected votes is always max_votes per ballot
    metrics.expected_votes += contest.max_votes as u64;

    metrics
}
