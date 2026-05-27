// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::Result;
use crate::pipes::do_tally::{ExtendedMetricsContest, InvalidVotes};
use sequent_core::plaintext::{DecodedVoteContest, InvalidPlaintextErrorType};
use sequent_core::{
    ballot::{BallotStyle, Candidate, Contest, Weight},
    types::ceremonies::{CountingAlgType, TallyOperation},
};
use std::str::FromStr;
use tracing::{info, instrument};
use uuid::Uuid;

/// Calculates the number of undervotes in a ballot.
fn calculate_undervotes(vote: &DecodedVoteContest, contest: &Contest) -> u64 {
    // Count actual votes (selected > -1)
    let actual_votes: u64 = vote.choices.iter().fold(0u64, |acc, choice| {
        if choice.selected > -1 {
            acc.saturating_add(1)
        } else {
            acc
        }
    });

    // Calculate undervotes based on max_votes
    let max_votes = u64::try_from(contest.max_votes).expect("max_votes should be non-negative");
    max_votes.saturating_sub(actual_votes)
}

/// Calculates the number of valid votes in a ballot.
fn calculate_valid_votes(vote: &DecodedVoteContest, contest: &Contest) -> u64 {
    // Count actual votes (selected > -1)
    let actual_votes: u64 = vote.choices.iter().fold(0u64, |acc, choice| {
        if choice.selected > -1 {
            acc.saturating_add(1)
        } else {
            acc
        }
    });

    // Check if votes are within valid range
    let min_votes_u64 = contest.min_votes.cast_unsigned();
    let max_votes_u64 = contest.max_votes.cast_unsigned();
    if actual_votes >= min_votes_u64 && actual_votes <= max_votes_u64 {
        actual_votes
    } else {
        0
    }
}

/// Calculates the number of overvotes in a ballot.
fn calculate_overvotes(vote: &DecodedVoteContest, contest: &Contest) -> u64 {
    // Count actual votes (selected > -1)
    let actual_votes: u64 = vote.choices.iter().fold(0u64, |acc, choice| {
        if choice.selected > -1 {
            acc.saturating_add(1)
        } else {
            acc
        }
    });

    // Calculate overvotes if actual votes exceed max_votes
    let max_votes_u64 = contest.max_votes.cast_unsigned();
    actual_votes.saturating_sub(max_votes_u64)
}

/// Updates extended metrics for a contest based on a decoded vote.
///
/// Calculates valid votes, undervotes, and overvotes and updates the metrics accordingly.
#[instrument(skip_all)]
pub fn update_extended_metrics(
    vote: &DecodedVoteContest,
    current_metrics: &ExtendedMetricsContest,
    contest: &Contest,
) -> ExtendedMetricsContest {
    let metrics = *current_metrics;
    let mut result = metrics;

    // Calculate valid votes first
    let valid_votes = calculate_valid_votes(vote, contest);
    result.votes_actually = result.votes_actually.saturating_add(valid_votes);

    // Calculate undervotes
    let undervotes = calculate_undervotes(vote, contest);
    result.under_votes = result.under_votes.saturating_add(undervotes);

    // Calculate overvotes
    let overvotes = calculate_overvotes(vote, contest);
    result.over_votes = result.over_votes.saturating_add(overvotes);

    // Expected votes is always max_votes per ballot
    let max_votes_u64 = contest.max_votes.cast_unsigned();
    result.expected_votes = result.expected_votes.saturating_add(max_votes_u64);

    result
}

/// Gets the tally operation for a contest from its annotations.
#[instrument(skip_all)]
pub fn get_contest_tally_operation(contest: &Contest) -> TallyOperation {
    let default_tally_op = contest
        .get_counting_algorithm()
        .get_default_tally_operation_for_contest();
    let annotations = contest.annotations.clone().unwrap_or_default();
    let operation = annotations
        .get("tally_operation")
        .cloned()
        .unwrap_or_default();
    TallyOperation::from_str(&operation).unwrap_or(default_tally_op)
}

/// Gets the tally operation for an area based on ballot styles and counting algorithm.
#[instrument(skip_all)]
pub fn get_area_tally_operation(
    ballot_styles: &[BallotStyle],
    counting_alg: CountingAlgType,
    area_id: &Uuid,
) -> TallyOperation {
    let area_ballot_style: Option<&BallotStyle> = ballot_styles
        .iter()
        .find(|bs| bs.area_id == area_id.to_string());

    match area_ballot_style
        .and_then(|bs| bs.area_annotations.as_ref())
        .and_then(|area_annotations| area_annotations.tally_operation)
    {
        Some(tally_op) => tally_op,
        None => counting_alg.get_default_tally_operation_for_area(),
    }
}

/// Gets the weight for an area from ballot styles.
#[instrument(skip_all)]
pub fn get_area_weight(ballot_styles: &[BallotStyle], area_id: &Uuid) -> Weight {
    let area_ballot_style: Option<&BallotStyle> = ballot_styles
        .iter()
        .find(|bs| bs.area_id == area_id.to_string());

    area_ballot_style
        .and_then(|bs| {
            bs.area_annotations
                .as_ref()
                .map(sequent_core::ballot::AreaAnnotations::get_weight)
        })
        .unwrap_or_default()
}
