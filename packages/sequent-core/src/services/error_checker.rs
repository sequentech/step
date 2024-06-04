// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;

use crate::{
    ballot::Contest,
    plaintext::{
        DecodedVoteContest, InvalidPlaintextError, InvalidPlaintextErrorType,
    },
};

pub fn check_max_selections_per_type(
    contest: &Contest,
    decoded_vote: &DecodedVoteContest,
) -> Vec<InvalidPlaintextError> {
    let presentation =
        contest.presentation.clone().unwrap_or(Default::default());
    let Some(max_selections_per_type) =
        presentation.max_selections_per_type.clone()
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
        let Some(candidate_type) =
            candidates_type_map.get(&selection.id).clone()
        else {
            continue;
        };
        let current_count =
            type_count.get(candidate_type).clone().unwrap_or(&0);
        type_count.insert(candidate_type.clone(), current_count + 1);
    }

    let mut invalid_errors = vec![];

    for (key, value) in type_count.iter() {
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

pub fn check_under_vote_selections(contest: &Contest,
    decoded_vote: &DecodedVoteContest) -> Vec<InvalidPlaintextError>{
        let min_vote_count = &contest.min_votes;
        let max_vote_count = &contest.max_votes;
        let under_vote_alert = &contest.under_vote_alert;
        match under_vote_alert{
            Some(should_show_under_vote_alert)=>{
                if !should_show_under_vote_alert{
                    return vec![];
                }
                else{
                    let selection_count = &decoded_vote.choices.iter().filter(|choice| choice.selected > -1).count();
                    let mut invalid_alerts = vec![];
                    let under_vote_err = check_selection_count(selection_count, min_vote_count, max_vote_count);
                    match under_vote_err{
                        Some(alert) =>{
                            invalid_alerts.push(alert);
                            return invalid_alerts;
                        },
                        None=> {
                            return invalid_alerts;
                        }
                    }
                    
                }
            },
            None=>{
                return vec![];
            }
        }
    }

fn check_selection_count(selection_count: &usize, min_vote_count: &i64, max_vote_count: &i64) -> Option<InvalidPlaintextError> {
        if *selection_count > *min_vote_count as usize && *selection_count < *max_vote_count as usize  {
            Some(InvalidPlaintextError { 
                error_type: InvalidPlaintextErrorType::Implicit,
                candidate_id: None,
                message: Some(
                    "errors.implicit.underVote".to_string(),
                ),
                message_map: HashMap::from([
                    ("type".to_string(), "alert".to_string()),
                    ("numSelected".to_string(), selection_count.to_string()),
                    ("underVote".to_string(), max_vote_count.to_string()),
                ]),
             })
        } else {
            None
        }
}

pub fn check_contest(
    contest: &Contest,
    decoded_vote: &DecodedVoteContest,
) -> DecodedVoteContest {
    let mut with_errors = decoded_vote.clone();

    let mut invalid_errors = decoded_vote.invalid_errors.clone();
    let mut invalid_alerts = decoded_vote.invalid_alerts.clone();

    // check max selections per type
    invalid_errors.extend(check_max_selections_per_type(contest, decoded_vote));

    // check range selection per type
    invalid_alerts.extend(check_under_vote_selections(contest, decoded_vote));

    with_errors.invalid_errors = invalid_errors;
    with_errors.invalid_alerts = invalid_alerts;
    with_errors
}
