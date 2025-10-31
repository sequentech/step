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

#[instrument]
pub fn get_contest_tally_operation(contest: &Contest) -> TallyOperation {
    let default_tally_op = match contest.get_counting_algorithm() {
        CountingAlgType::InstantRunoff => TallyOperation::ProcessBallots,
        _ => TallyOperation::AggregateResults,
    };
    let annotations = contest.annotations.clone().unwrap_or_default();
    let operation = annotations
        .get("tally_operation")
        .map(|val| val.clone())
        .unwrap_or_default();
    TallyOperation::from_str(&operation).unwrap_or(default_tally_op)
}

#[instrument]
pub fn get_area_tally_operation(
    ballot_styles: &Vec<BallotStyle>,
    counting_alg: CountingAlgType,
    area_id: &Uuid,
) -> TallyOperation {
    let default_tally_op = match counting_alg {
        CountingAlgType::InstantRunoff => TallyOperation::ParticipationSummary,
        _ => TallyOperation::ProcessBallots,
    };
    let area_ballot_style: Option<&BallotStyle> = ballot_styles
        .iter()
        .find(|bs| bs.area_id == area_id.to_string());

    area_ballot_style
        .map(|bs| {
            bs.area_annotations
                .as_ref()
                .map(|area_annotations| area_annotations.get_tally_operation())
        })
        .flatten()
        .unwrap_or(TallyOperation::ProcessBallots)
}

#[instrument]
pub fn get_area_weight(ballot_styles: &Vec<BallotStyle>, area_id: &Uuid) -> Weight {
    let area_ballot_style: Option<&BallotStyle> = ballot_styles
        .iter()
        .find(|bs| bs.area_id == area_id.to_string());

    area_ballot_style
        .map(|bs| {
            bs.area_annotations
                .as_ref()
                .map(|area_annotations| area_annotations.get_weight())
        })
        .flatten()
        .unwrap_or_default()
}
