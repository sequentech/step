// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ballot_codec::multi_ballot::DecodedContestChoices;
use crate::plaintext::DecodedVoteContest;
use crate::{
    ballot::{
        ContestPresentation, EBlankVotePolicy, EDuplicatedRankPolicy,
        EOverVotePolicy, EPreferenceGapsPolicy, EUnderVotePolicy,
        InvalidVotePolicy,
    },
    plaintext::{InvalidPlaintextError, InvalidPlaintextErrorType},
};
use std::collections::HashMap;

/// Result of a ballot checker operation, containing errors and alerts.
#[derive(Default, PartialEq, Eq, Debug, Clone)]
pub struct CheckerResult {
    /// List of invalid errors found during checking.
    pub invalid_errors: Vec<InvalidPlaintextError>,
    /// List of invalid alerts found during checking.
    pub invalid_alerts: Vec<InvalidPlaintextError>,
}

impl DecodedVoteContest {
    /// Update this contest with errors and alerts from a checker result.
    pub fn update(&mut self, data: CheckerResult) {
        self.invalid_errors.extend(data.invalid_errors);
        self.invalid_alerts.extend(data.invalid_alerts);
    }
}

impl DecodedContestChoices {
    /// Update this contest with errors and alerts from a checker result.
    pub fn update(&mut self, data: CheckerResult) {
        self.invalid_errors.extend(data.invalid_errors);
        self.invalid_alerts.extend(data.invalid_alerts);
    }
}

/// Checks the validity of max and min votes policy.
///
/// # Returns
/// Tuple of (`max_votes_opt`, `min_votes_opt`, `checker_result`)
#[must_use]
pub fn check_max_min_votes_policy(
    max_votes: i64,
    min_votes: i64,
) -> (Option<usize>, Option<usize>, CheckerResult) {
    let mut checker_result = CheckerResult::default();

    let max_votes_opt = if let Ok(val) = usize::try_from(max_votes) {
        Some(val)
    } else {
        checker_result.invalid_errors.push(InvalidPlaintextError {
            error_type: InvalidPlaintextErrorType::EncodingError,
            candidate_id: None,
            message: Some("errors.encoding.invalidMaxVotes".to_string()),
            message_map: HashMap::from([(
                "max".to_string(),
                max_votes.to_string(),
            )]),
        });
        None
    };

    let min_votes_opt = if let Ok(val) = usize::try_from(min_votes) {
        Some(val)
    } else {
        checker_result.invalid_errors.push(InvalidPlaintextError {
            error_type: InvalidPlaintextErrorType::EncodingError,
            candidate_id: None,
            message: Some("errors.encoding.invalidMinVotes".to_string()),
            message_map: HashMap::from([(
                "min".to_string(),
                min_votes.to_string(),
            )]),
        });
        None
    };

    (max_votes_opt, min_votes_opt, checker_result)
}

/// Checks if the number of selected candidates meets the minimum votes policy.
///
/// # Returns
/// `CheckerResult` with errors if the policy is violated.
#[must_use]
pub fn check_min_vote_policy(
    num_selected_candidates: usize,
    min_votes: usize,
) -> CheckerResult {
    let mut checker_result = CheckerResult::default();
    if num_selected_candidates < min_votes {
        checker_result.invalid_errors.push(InvalidPlaintextError {
            error_type: InvalidPlaintextErrorType::Implicit,
            candidate_id: None,
            message: Some("errors.implicit.selectedMin".to_string()),
            message_map: HashMap::from([
                (
                    "numSelected".to_string(),
                    num_selected_candidates.to_string(),
                ),
                ("min".to_string(), min_votes.to_string()),
            ]),
        });
    }
    checker_result
}

/// Checks the blank vote policy for a contest.
///
/// # Returns
/// `CheckerResult` with errors or alerts if the policy is violated.
#[must_use]
pub fn check_blank_vote_policy(
    presentation: &ContestPresentation,
    num_selected_candidates: usize,
    is_explicit_invalid: bool,
) -> CheckerResult {
    let mut checker_result = CheckerResult::default();
    let blank_vote_policy = presentation.blank_vote_policy.unwrap_or_default();
    if num_selected_candidates == 0
        && !is_explicit_invalid
        && EBlankVotePolicy::ALLOWED != blank_vote_policy
    {
        (match blank_vote_policy {
            EBlankVotePolicy::NOT_ALLOWED => &mut checker_result.invalid_errors,
            _ => &mut checker_result.invalid_alerts,
        })
        .push(InvalidPlaintextError {
            error_type: InvalidPlaintextErrorType::Implicit,
            candidate_id: None,
            message: Some("errors.implicit.blankVote".to_string()),
            message_map: HashMap::from([
                ("type".to_string(), "alert".to_string()),
                (
                    "numSelected".to_string(),
                    num_selected_candidates.to_string(),
                ),
            ]),
        });
    }
    checker_result
}

/// Checks the over-vote policy for a contest.
///
/// # Returns
/// `CheckerResult` with errors or alerts if the policy is violated.
#[must_use]
pub fn check_over_vote_policy(
    presentation: &ContestPresentation,
    num_selected_candidates: usize,
    max_votes: usize,
) -> CheckerResult {
    let mut checker_result = CheckerResult::default();
    if num_selected_candidates == max_votes
        && presentation.over_vote_policy
            == Some(EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_DISABLE)
    {
        checker_result.invalid_alerts.push(InvalidPlaintextError {
            error_type: InvalidPlaintextErrorType::Implicit,
            candidate_id: None,
            message: Some("errors.implicit.overVoteDisabled".to_string()),
            message_map: HashMap::from([
                ("type".to_string(), "alert".to_string()),
                (
                    "numSelected".to_string(),
                    num_selected_candidates.to_string(),
                ),
                ("max".to_string(), max_votes.to_string()),
            ]),
        });
    } else if num_selected_candidates > max_votes {
        let text_error = || InvalidPlaintextError {
            error_type: InvalidPlaintextErrorType::Implicit,
            candidate_id: None,
            message: Some("errors.implicit.selectedMax".to_string()),
            message_map: HashMap::from([
                (
                    "numSelected".to_string(),
                    num_selected_candidates.to_string(),
                ),
                ("max".to_string(), max_votes.to_string()),
            ]),
        };

        // for errors, we use only invalid_vote_policy. Overvote policy is going
        // to be used only for alerts
        checker_result.invalid_errors.push(text_error());

        match presentation.over_vote_policy.unwrap_or_default() {
            EOverVotePolicy::ALLOWED => {}
            EOverVotePolicy::ALLOWED_WITH_MSG
            | EOverVotePolicy::ALLOWED_WITH_MSG_AND_ALERT
            | EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_ALERT
            | EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_DISABLE => {
                checker_result.invalid_alerts.push(text_error());
            }
        }
    }
    checker_result
}

/// Checks the under-vote policy for a contest.
///
/// # Returns
/// `CheckerResult` with alerts if the policy is violated.
#[must_use]
pub fn check_under_vote_policy(
    presentation: &ContestPresentation,
    num_selected_candidates: usize,
    max_votes: Option<usize>,
    min_votes: Option<usize>,
) -> CheckerResult {
    let mut checker_result = CheckerResult::default();
    // Handle undervote alerts. Please note that the case of
    // `num_selected_candidates < min_votes` is handled in prev step and
    // is independent of `under_vote_policy`, it's an invalid vote no
    // matter what
    let under_vote_policy = presentation.under_vote_policy.unwrap_or_default();
    let min_votes = min_votes.unwrap_or(0);
    if let Some(max_votes) = max_votes {
        if under_vote_policy != EUnderVotePolicy::ALLOWED
            && num_selected_candidates < max_votes
            && num_selected_candidates >= min_votes
        {
            checker_result.invalid_alerts.push(InvalidPlaintextError {
                error_type: InvalidPlaintextErrorType::Implicit,
                candidate_id: None,
                message: Some("errors.implicit.underVote".to_string()),
                message_map: HashMap::from([
                    ("type".to_string(), "alert".to_string()),
                    (
                        "numSelected".to_string(),
                        num_selected_candidates.to_string(),
                    ),
                    ("min".to_string(), min_votes.to_string()),
                    ("max".to_string(), max_votes.to_string()),
                ]),
            });
        }
    }
    checker_result
}

/// Checks the duplicated rank policy for a contest.
///
/// # Returns
/// `CheckerResult` with errors if the policy is violated.
#[must_use]
pub fn check_duplicated_rank_policy(
    presentation: &ContestPresentation,
) -> CheckerResult {
    let mut checker_result = CheckerResult::default();
    let policy = presentation.duplicated_rank_policy.unwrap_or_default();
    let error = InvalidPlaintextError {
        error_type: InvalidPlaintextErrorType::Implicit,
        candidate_id: None,
        message: Some("errors.implicit.duplicatedPosition".to_string()),
        message_map: HashMap::new(),
    };
    match policy {
        EDuplicatedRankPolicy::ALLOWED_WARN_AND_DIALOG
        | EDuplicatedRankPolicy::NOT_ALLOWED_WARN_AND_DIALOG => {
            checker_result.invalid_errors.push(error);
        }
    }
    checker_result
}

/// Checks the preference gaps policy for a contest.
///
/// # Returns
/// `CheckerResult` with errors if the policy is violated.
#[must_use]
pub fn check_preference_gaps_policy(
    presentation: &ContestPresentation,
) -> CheckerResult {
    let mut checker_result = CheckerResult::default();
    let policy = presentation.preference_gaps_policy.unwrap_or_default();
    let error = InvalidPlaintextError {
        error_type: InvalidPlaintextErrorType::Implicit,
        candidate_id: None,
        message: Some("errors.implicit.preferenceOrderWithGaps".to_string()),
        message_map: HashMap::new(),
    };
    match policy {
        EPreferenceGapsPolicy::ALLOWED_WARN_AND_DIALOG
        | EPreferenceGapsPolicy::NOT_ALLOWED_WARN_AND_DIALOG => {
            checker_result.invalid_errors.push(error);
        }
    }
    checker_result
}

/// Checks the invalid vote policy for a contest.
///
/// # Returns
/// `CheckerResult` with errors or alerts if the policy is violated.
#[must_use]
pub fn check_invalid_vote_policy(
    presentation: &ContestPresentation,
    is_explicit_invalid: bool,
) -> CheckerResult {
    let mut checker_result = CheckerResult::default();
    let invalid_vote_policy =
        presentation.invalid_vote_policy.clone().unwrap_or_default();
    // explicit invalid error
    if is_explicit_invalid {
        match invalid_vote_policy {
            InvalidVotePolicy::NOT_ALLOWED => {
                checker_result.invalid_errors.push(InvalidPlaintextError {
                    error_type: InvalidPlaintextErrorType::Explicit,
                    candidate_id: None,
                    message: Some("errors.explicit.notAllowed".to_string()),
                    message_map: HashMap::new(),
                });
            }
            InvalidVotePolicy::WARN_INVALID_IMPLICIT_AND_EXPLICIT => {
                checker_result.invalid_alerts.push(InvalidPlaintextError {
                    error_type: InvalidPlaintextErrorType::Explicit,
                    candidate_id: None,
                    message: Some("errors.explicit.alert".to_string()),
                    message_map: HashMap::new(),
                });
            }
            _ => {}
        }
    }
    checker_result
}
