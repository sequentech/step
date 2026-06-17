// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Vote-policy validation during ballot encoding and decoding.

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

/// Errors and alerts collected while validating a decoded vote against contest policies.
#[derive(Default, PartialEq, Eq, Debug, Clone)]
pub struct CheckerResult {
    /// Hard validation failures that invalidate the ballot.
    pub invalid_errors: Vec<InvalidPlaintextError>,
    /// Soft warnings shown to the voter but allowed to proceed.
    pub invalid_alerts: Vec<InvalidPlaintextError>,
}

impl DecodedVoteContest {
    /// Merges additional checker results into this decoded contest.
    pub fn update(&mut self, data: CheckerResult) -> () {
        self.invalid_errors.extend(data.invalid_errors);
        self.invalid_alerts.extend(data.invalid_alerts);
    }
}

impl DecodedContestChoices {
    /// Merges additional checker results into this decoded contest.
    pub fn update(&mut self, data: CheckerResult) -> () {
        self.invalid_errors.extend(data.invalid_errors);
        self.invalid_alerts.extend(data.invalid_alerts);
    }
}

/// Validates `max_votes` and `min_votes` and returns usable `usize` bounds.
///
/// Returns encoding errors when either bound cannot be represented as `usize`.
pub fn check_max_min_votes_policy(
    max_votes: i64,
    min_votes: i64,
) -> (Option<usize>, Option<usize>, CheckerResult) {
    let mut checker_result: CheckerResult = Default::default();

    let max_votes_opt: Option<usize> = match usize::try_from(max_votes) {
        Ok(val) => Some(val),
        Err(_) => {
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
        }
    };

    let min_votes_opt: Option<usize> = match usize::try_from(min_votes) {
        Ok(val) => Some(val),
        Err(_) => {
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
        }
    };

    (max_votes_opt, min_votes_opt, checker_result)
}

/// Returns an error when fewer candidates are selected than `min_votes` requires.
pub fn check_min_vote_policy(
    num_selected_candidates: usize,
    min_votes: usize,
) -> CheckerResult {
    let mut checker_result: CheckerResult = Default::default();

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

/// Applies the contest blank-vote policy to a selection with no candidates chosen.
pub fn check_blank_vote_policy(
    presentation: &ContestPresentation,
    num_selected_candidates: usize,
    is_explicit_invalid: bool,
) -> CheckerResult {
    let mut checker_result: CheckerResult = Default::default();

    let blank_vote_policy =
        presentation.blank_vote_policy.clone().unwrap_or_default();

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

/// Applies over-vote policy when too many candidates are selected.
pub fn check_over_vote_policy(
    presentation: &ContestPresentation,
    num_selected_candidates: usize,
    max_votes: usize,
) -> CheckerResult {
    let mut checker_result: CheckerResult = Default::default();
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
            EOverVotePolicy::ALLOWED => (),
            EOverVotePolicy::ALLOWED_WITH_MSG => {
                checker_result.invalid_alerts.push(text_error())
            }
            EOverVotePolicy::ALLOWED_WITH_MSG_AND_ALERT => {
                checker_result.invalid_alerts.push(text_error())
            }
            EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_ALERT => {
                checker_result.invalid_alerts.push(text_error());
            }
            EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_DISABLE => {
                checker_result.invalid_alerts.push(text_error());
            }
        };
    }
    checker_result
}

/// Applies under-vote policy when fewer than the maximum allowed selections are made.
pub fn check_under_vote_policy(
    presentation: &ContestPresentation,
    num_selected_candidates: usize,
    max_votes: Option<usize>,
    min_votes: Option<usize>,
) -> CheckerResult {
    let mut checker_result: CheckerResult = Default::default();
    // Handle undervote alerts. Please note that the case of
    // `num_selected_candidates < min_votes` is handle in prev step and
    // is independent of `under_vote_policy`, it's an invalid vote no
    // matter what
    let under_vote_policy =
        presentation.under_vote_policy.clone().unwrap_or_default();
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

/// Applies duplicated-rank policy for preferential voting contests.
pub fn check_duplicated_rank_policy(
    presentation: &ContestPresentation,
) -> CheckerResult {
    let mut checker_result: CheckerResult = Default::default();
    let policy = presentation
        .duplicated_rank_policy
        .clone()
        .unwrap_or_default();
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

/// Applies preference-gap policy when ranked choices skip positions.
pub fn check_preference_gaps_policy(
    presentation: &ContestPresentation,
) -> CheckerResult {
    let mut checker_result: CheckerResult = Default::default();
    let policy = presentation
        .preference_gaps_policy
        .clone()
        .unwrap_or_default();
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

/// Applies explicit-invalid vote policy when the voter spoils the ballot intentionally.
pub fn check_invalid_vote_policy(
    presentation: &ContestPresentation,
    is_explicit_invalid: bool,
) -> CheckerResult {
    let mut checker_result: CheckerResult = Default::default();
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
