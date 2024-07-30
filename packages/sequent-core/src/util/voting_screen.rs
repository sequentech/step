// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ballot::*;
use crate::plaintext::*;
use std::collections::HashMap;

pub fn check_voting_not_allowed_next(
    contests: Vec<Contest>,
    decoded_contests: HashMap<String, DecodedVoteContest>,
) -> Result<bool, bool> {

    let voting_not_allowed = contests.iter().any(|contest| {
        let default_vote_policy = InvalidVotePolicy::default();
        let vote_policy = contest
            .presentation
            .as_ref()
            .and_then(|p| p.invalid_vote_policy.as_ref())
            .unwrap_or(&default_vote_policy);

        let default_blank_policy = EBlankVotePolicy::default();
        let blank_policy = contest
            .presentation
            .as_ref()
            .and_then(|p| p.blank_vote_policy.as_ref())
            .unwrap_or(&default_blank_policy);

        if let Some(decoded_contest) = decoded_contests.get(&contest.id) {
            let choices_selected = decoded_contest
                .choices
                .iter()
                .any(|choice| choice.selected == 0);
            let invalid_errors: Vec<InvalidPlaintextError> =
                decoded_contest.invalid_errors.clone();
            invalid_errors.iter().any(|error| {
                matches!(
                    error.error_type,
                    InvalidPlaintextErrorType::Explicit
                        | InvalidPlaintextErrorType::EncodingError
                )
            }) || (invalid_errors.len() > 0
                && *vote_policy == InvalidVotePolicy::NOT_ALLOWED)
                || (!choices_selected
                    && *blank_policy == EBlankVotePolicy::NOT_ALLOWED)
        } else {
            false
        }
    });

    Ok(voting_not_allowed)
}

pub fn check_voting_error_dialog(
    contests: Vec<Contest>,
    decoded_contests: HashMap<String, DecodedVoteContest>,
) -> Result<bool, bool> {
    let show_voting_alert = contests.iter().any(|contest| {
        let default_vote_policy = InvalidVotePolicy::default();
        let vote_policy = contest
            .presentation
            .as_ref()
            .and_then(|p| p.invalid_vote_policy.as_ref())
            .unwrap_or(&default_vote_policy);

        let default_blank_policy = EBlankVotePolicy::default();
        let blank_policy = contest
            .presentation
            .as_ref()
            .and_then(|p| p.blank_vote_policy.as_ref())
            .unwrap_or(&default_blank_policy);

        if let Some(decoded_contest) = decoded_contests.get(&contest.id) {
            let choices_selected = decoded_contest
                .choices
                .iter()
                .any(|choice| choice.selected == 0);
            let invalid_errors: Vec<InvalidPlaintextError> =
                decoded_contest.invalid_errors.clone();
            let explicit_invalid = decoded_contest.is_explicit_invalid;
            (invalid_errors.len() > 0
                && *vote_policy != InvalidVotePolicy::ALLOWED)
                || (*vote_policy
                    == InvalidVotePolicy::WARN_INVALID_IMPLICIT_AND_EXPLICIT
                    && explicit_invalid)
                || (*blank_policy == EBlankVotePolicy::WARN
                    && !choices_selected)
        } else {
            false
        }
    });

    Ok(show_voting_alert)
}