// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::plaintext::{DecodedVoteChoice, DecodedVoteContest};
use chrono::{DateTime, Local};

pub fn get_current_date() -> String {
    let local: DateTime<Local> = Local::now();
    local.format("%-d/%-m/%Y").to_string()
}

pub fn normalize_vote_contest(
    input: &DecodedVoteContest,
    tally_type: &str,
    remove_errors: bool,
) -> DecodedVoteContest {
    let mut original = input.clone();
    let mut choices: Vec<DecodedVoteChoice> = original
        .choices
        .iter()
        .map(|choice| normalize_vote_choice(choice, tally_type))
        .collect();
    choices.sort_by_key(|q| q.id.clone());
    original.choices = choices;
    if remove_errors {
        original.invalid_errors = vec![];
    }
    original
}

pub fn normalize_vote_choice(
    input: &DecodedVoteChoice,
    tally_type: &str,
) -> DecodedVoteChoice {
    let mut original = input.clone();
    if "plurality-at-large" == tally_type {
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
