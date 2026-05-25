// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;

use crate::{
    ballot::Contest,
    plaintext::{
        DecodedVoteContest, InvalidPlaintextError, InvalidPlaintextErrorType,
    },
};

/// Checks the maximum selections per candidate type in a contest.
///
/// # Panics
/// Panics if the count overflows when incrementing selections per type.
#[must_use]
pub fn check_max_selections_per_type(
    contest: &Contest,
    decoded_vote: &DecodedVoteContest,
) -> Vec<InvalidPlaintextError> {
    let presentation = contest.presentation.clone().unwrap_or_default();
    let Some(max_selections_per_type) = presentation.max_selections_per_type
    else {
        return vec![];
    };

    let mut candidates_type_map = HashMap::<String, String>::new();

    for candidate in &contest.candidates {
        if let Some(candidate_type) = candidate.candidate_type.clone() {
            candidates_type_map.insert(candidate.id.clone(), candidate_type);
        }
    }

    let mut type_count = HashMap::<String, u64>::new();

    for selection in &decoded_vote.choices {
        if selection.selected < 0 {
            continue;
        }
        let Some(candidate_type) = candidates_type_map.get(&selection.id)
        else {
            continue;
        };
        let current_count = type_count.get(candidate_type).unwrap_or(&0);
        let count_to_insert = current_count
            .checked_add(1)
            .expect("Overflow when counting selections per type");
        type_count.insert(candidate_type.clone(), count_to_insert);
    }

    let mut invalid_errors = vec![];

    for (key, value) in &type_count {
        if *value > max_selections_per_type {
            invalid_errors.push(InvalidPlaintextError {
                error_type: InvalidPlaintextErrorType::Implicit,
                candidate_id: None,
                message: Some(
                    "errors.implicit.maxSelectionsPerType".to_string(),
                ),
                message_map: HashMap::from([
                    ("type".to_string(), key.clone()),
                    ("numSelected".to_string(), value.to_string()),
                    ("max".to_string(), max_selections_per_type.to_string()),
                ]),
            });
        }
    }

    invalid_errors
}

#[must_use]
pub fn check_contest(
    contest: &Contest,
    decoded_vote: &DecodedVoteContest,
) -> DecodedVoteContest {
    let mut with_errors = decoded_vote.clone();

    let mut invalid_errors = decoded_vote.invalid_errors.clone();

    // check max selections per type
    invalid_errors.extend(check_max_selections_per_type(contest, decoded_vote));

    with_errors.invalid_errors = invalid_errors;
    with_errors
}
