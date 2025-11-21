// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use std::collections::HashMap;

use crate::{
    ballot::{BallotStyle, Contest},
    plaintext::{DecodedVoteChoice, DecodedVoteContest},
    types::ceremonies::CountingAlgType,
};

pub fn normalize_vote_contest(
    input: &DecodedVoteContest,
    tally_type: CountingAlgType,
    remove_errors: bool,
    invalid_choice_ids: &Vec<String>,
) -> DecodedVoteContest {
    let mut original = input.clone();
    let filtered_choices: Vec<&DecodedVoteChoice> = original
        .choices
        .iter()
        .filter(|choice| !invalid_choice_ids.contains(&choice.id))
        .collect();
    let mut choices: Vec<DecodedVoteChoice> = filtered_choices
        .into_iter()
        .map(|choice| normalize_vote_choice(choice, tally_type))
        .collect();
    choices.sort_by_key(|q| q.id.clone());
    original.choices = choices;
    if remove_errors {
        original.invalid_errors = vec![];
        original.invalid_alerts = vec![];
    }
    original
}

pub fn normalize_election(
    input: &Vec<DecodedVoteContest>,
    ballot_style: &BallotStyle,
    remove_errors: bool,
) -> Result<Vec<DecodedVoteContest>> {
    let contest_map: HashMap<String, Contest> = ballot_style
        .contests
        .clone()
        .into_iter()
        .map(|contest| (contest.id.clone(), contest))
        .collect();
    let mut result: Vec<DecodedVoteContest> = input
        .clone()
        .into_iter()
        .map(|decoded_contest| -> Result<DecodedVoteContest> {
            let contest = contest_map
                .get(&decoded_contest.contest_id)
                .cloned()
                .ok_or(anyhow!(
                    "Can't find contest {}",
                    decoded_contest.contest_id
                ))?;
            let invalid_candidate_ids = contest.get_invalid_candidate_ids();
            Ok(normalize_vote_contest(
                &decoded_contest,
                contest.get_counting_algorithm(),
                remove_errors,
                &invalid_candidate_ids,
            ))
        })
        .collect::<Result<Vec<_>>>()?;

    result.sort_by(|a, b| a.contest_id.cmp(&b.contest_id));

    Ok(result)
}

pub fn normalize_vote_choice(
    input: &DecodedVoteChoice,
    tally_type: CountingAlgType,
) -> DecodedVoteChoice {
    let mut original = input.clone();
    if CountingAlgType::PluralityAtLarge == tally_type {
        original.selected = if original.selected < 0 { -1 } else { 0 };
    } else {
        original.selected = if original.selected < 0 {
            -1
        } else {
            original.selected
        };
    }

    original.write_in_text = match original.write_in_text {
        Some(text) => {
            if text.len() > 0 {
                Some(text)
            } else {
                None
            }
        }
        None => None,
    };
    original
}
