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

pub fn is_explicit_blank_vote(vote: &DecodedVoteContest, contest: &Contest) -> bool {
    vote.choices.iter().any(|choice| {
        choice.selected > -1
            && contest
                .candidates
                .iter()
                .find(|candidate| candidate.id == choice.id)
                .map(|candidate| candidate.is_explicit_blank())
                .unwrap_or(false)
    })
}

fn count_actual_votes(vote: &DecodedVoteContest, contest: &Contest) -> u64 {
    vote.choices.iter().fold(0u64, |acc, choice| {
        let is_explicit_blank = contest
            .candidates
            .iter()
            .find(|candidate| candidate.id == choice.id)
            .map(|candidate| candidate.is_explicit_blank())
            .unwrap_or(false);

        if choice.selected > -1 && !is_explicit_blank {
            acc + 1
        } else {
            acc
        }
    })
}

fn calculate_undervotes(vote: &DecodedVoteContest, contest: &Contest) -> u64 {
    let actual_votes = count_actual_votes(vote, contest);

    // Calculate undervotes based on max_votes
    let max_votes = contest.max_votes as u64;
    if actual_votes < max_votes {
        max_votes - actual_votes
    } else {
        0
    }
}

fn calculate_valid_votes(vote: &DecodedVoteContest, contest: &Contest) -> u64 {
    let actual_votes = count_actual_votes(vote, contest);

    // Check if votes are within valid range
    if actual_votes >= (contest.min_votes as u64) && actual_votes <= (contest.max_votes as u64) {
        actual_votes
    } else {
        0
    }
}

fn calculate_overvotes(vote: &DecodedVoteContest, contest: &Contest) -> u64 {
    let actual_votes = count_actual_votes(vote, contest);

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

    // Calculate undervotes if not a decline to vote
    if !vote.is_decline_to_vote() {
        let undervotes = calculate_undervotes(vote, contest);
        metrics.under_votes += undervotes;
    }

    // Calculate overvotes
    let overvotes = calculate_overvotes(vote, contest);
    metrics.over_votes += overvotes;

    // Expected votes is always max_votes per ballot
    metrics.expected_votes += contest.max_votes as u64;

    metrics
}

#[instrument(skip_all)]
pub fn get_contest_tally_operation(contest: &Contest) -> TallyOperation {
    let default_tally_op = contest
        .get_counting_algorithm()
        .get_default_tally_operation_for_contest();
    let annotations = contest.annotations.clone().unwrap_or_default();
    let operation = annotations
        .get("tally_operation")
        .map(|val| val.clone())
        .unwrap_or_default();
    TallyOperation::from_str(&operation).unwrap_or(default_tally_op)
}

#[instrument(skip_all)]
pub fn get_area_tally_operation(
    ballot_styles: &Vec<BallotStyle>,
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

#[instrument(skip_all)]
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
