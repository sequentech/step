// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::pipes::do_tally::{ExtendedMetricsContest, InvalidVotes};
use sequent_core::ballot::Contest;
use sequent_core::plaintext::{DecodedVoteContest, InvalidPlaintextErrorType};

use tracing::{info, instrument};

use super::Result;

fn calculate_undervotes(vote: &DecodedVoteContest, contest: &Contest) -> u64 {
    // Count actual votes (selected > -1)
    let actual_votes: u64 =
        vote.choices.iter().fold(
            0u64,
            |acc, choice| {
                if choice.selected > -1 {
                    acc + 1
                } else {
                    acc
                }
            },
        );

    // Calculate undervotes based on max_votes
    let max_votes = contest.max_votes as u64;
    if actual_votes < max_votes {
        max_votes - actual_votes
    } else {
        0
    }
}

fn calculate_valid_votes(vote: &DecodedVoteContest, contest: &Contest) -> u64 {
    // Count actual votes (selected > -1)
    let actual_votes: u64 =
        vote.choices.iter().fold(
            0u64,
            |acc, choice| {
                if choice.selected > -1 {
                    acc + 1
                } else {
                    acc
                }
            },
        );

    // Check if votes are within valid range
    if actual_votes >= (contest.min_votes as u64) && actual_votes <= (contest.max_votes as u64) {
        actual_votes
    } else {
        0
    }
}

fn calculate_overvotes(vote: &DecodedVoteContest, contest: &Contest) -> u64 {
    // Count actual votes (selected > -1)
    let actual_votes: u64 =
        vote.choices.iter().fold(
            0u64,
            |acc, choice| {
                if choice.selected > -1 {
                    acc + 1
                } else {
                    acc
                }
            },
        );

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
) -> ExtendedMetricsContest {
    let mut metrics = current_metrics.clone();

    // Calculate valid votes first
    let valid_votes = calculate_valid_votes(vote, contest);
    metrics.votes_actually += valid_votes;

    // Calculate undervotes
    let undervotes = calculate_undervotes(vote, contest);
    metrics.under_votes += undervotes;

    // Calculate overvotes
    let overvotes = calculate_overvotes(vote, contest);
    metrics.over_votes += overvotes;

    // Expected votes is always max_votes per ballot
    metrics.expected_votes += contest.max_votes as u64;

    metrics
}
