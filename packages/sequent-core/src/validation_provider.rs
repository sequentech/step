// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Conversion from the ballot types (`Contest`, `DecodedVoteContest`) to
//! the inputs of the `validation-spec` crate — `Config` (bounds and
//! policies) and `VoteState` (a summary of the voter's selections) — from
//! which that crate evaluates the vote-validation rules.
//!
//! - An unset policy resolves to the enum's default, matching how the
//!   ballot checkers resolve unset policies.
//! - `regulars` counts selected choices (`selected > -1`), excluding the
//!   explicit-blank and explicit-invalid marker candidates.
//! - `explicit_invalid` is set by the decoded contest's
//!   `is_explicit_invalid` flag or by a selected explicit-invalid marker
//!   candidate — the two ways a ballot is marked invalid.
//! - On preferential contests, `first_preferences` counts the regular
//!   choices at rank 0, and `duplicate_ranks` / `rank_gaps` come from
//!   [`DecodedVoteContest::validate_preferencial_order`]; on
//!   non-preferential contests all three are absent.
//! - Invalid (negative or out-of-range) `min_votes` / `max_votes` are
//!   rejected as [`ValidationProviderError::UnrepresentableBounds`].
//!   Ballot decoding reports such bounds as encoding errors
//!   (`errors.encoding.invalidMinVotes` / `invalidMaxVotes`), which
//!   callers handle from `invalid_errors` (see `util/voting_screen.rs`).
//!
//! `DecodedVoteContest::is_blank_ballot` is not consulted: the rules
//! derive blankness from the selections themselves.

use crate::ballot::{
    Contest, EBlankVotePolicy, EDuplicatedRankPolicy, EOverVotePolicy,
    EPreferenceGapsPolicy, EUnderVotePolicy, InvalidVotePolicy,
};
use crate::plaintext::{DecodedVoteContest, PreferencialOrderErrorType};
use validation_spec as spec;

/// A contest configuration that cannot be converted for validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationProviderError {
    /// `min_votes` / `max_votes` not representable as a count (negative or
    /// out of range) — the class `check_max_min_votes_policy` reports as
    /// `errors.encoding.invalidMinVotes` / `invalidMaxVotes`.
    UnrepresentableBounds { min_votes: i64, max_votes: i64 },
}

impl std::fmt::Display for ValidationProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationProviderError::UnrepresentableBounds {
                min_votes,
                max_votes,
            } => write!(
                f,
                "contest bounds not representable: min_votes={min_votes}, max_votes={max_votes}"
            ),
        }
    }
}

impl std::error::Error for ValidationProviderError {}

fn map_invalid(p: InvalidVotePolicy) -> spec::InvalidVotePolicy {
    match p {
        InvalidVotePolicy::ALLOWED => spec::InvalidVotePolicy::Allowed,
        InvalidVotePolicy::WARN => spec::InvalidVotePolicy::Warn,
        InvalidVotePolicy::WARN_INVALID_IMPLICIT_AND_EXPLICIT => {
            spec::InvalidVotePolicy::WarnInvalidImplicitAndExplicit
        }
        InvalidVotePolicy::NOT_ALLOWED => spec::InvalidVotePolicy::NotAllowed,
        InvalidVotePolicy::ALLOWED_WITH_EXCLUSIVE_EXPLICIT => {
            spec::InvalidVotePolicy::AllowedWithExclusiveExplicit
        }
    }
}

fn map_blank(p: EBlankVotePolicy) -> spec::BlankVotePolicy {
    match p {
        EBlankVotePolicy::ALLOWED => spec::BlankVotePolicy::Allowed,
        EBlankVotePolicy::WARN => spec::BlankVotePolicy::Warn,
        EBlankVotePolicy::WARN_ONLY_IN_REVIEW => {
            spec::BlankVotePolicy::WarnOnlyInReview
        }
        EBlankVotePolicy::NOT_ALLOWED => spec::BlankVotePolicy::NotAllowed,
    }
}

fn map_over(p: EOverVotePolicy) -> spec::OverVotePolicy {
    match p {
        EOverVotePolicy::ALLOWED => spec::OverVotePolicy::Allowed,
        EOverVotePolicy::ALLOWED_WITH_MSG => {
            spec::OverVotePolicy::AllowedWithMsg
        }
        EOverVotePolicy::ALLOWED_WITH_MSG_AND_ALERT => {
            spec::OverVotePolicy::AllowedWithMsgAndAlert
        }
        EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_ALERT => {
            spec::OverVotePolicy::NotAllowedWithMsgAndAlert
        }
        EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_DISABLE => {
            spec::OverVotePolicy::NotAllowedWithMsgAndDisable
        }
    }
}

fn map_under(p: EUnderVotePolicy) -> spec::UnderVotePolicy {
    match p {
        EUnderVotePolicy::ALLOWED => spec::UnderVotePolicy::Allowed,
        EUnderVotePolicy::WARN => spec::UnderVotePolicy::Warn,
        EUnderVotePolicy::WARN_ONLY_IN_REVIEW => {
            spec::UnderVotePolicy::WarnOnlyInReview
        }
        EUnderVotePolicy::WARN_AND_ALERT => spec::UnderVotePolicy::WarnAndAlert,
    }
}

fn map_dup(p: EDuplicatedRankPolicy) -> spec::RankPolicy {
    match p {
        EDuplicatedRankPolicy::ALLOWED_WARN_AND_DIALOG => {
            spec::RankPolicy::AllowedWarnAndDialog
        }
        EDuplicatedRankPolicy::NOT_ALLOWED_WARN_AND_DIALOG => {
            spec::RankPolicy::NotAllowedWarnAndDialog
        }
    }
}

fn map_gap(p: EPreferenceGapsPolicy) -> spec::RankPolicy {
    match p {
        EPreferenceGapsPolicy::ALLOWED_WARN_AND_DIALOG => {
            spec::RankPolicy::AllowedWarnAndDialog
        }
        EPreferenceGapsPolicy::NOT_ALLOWED_WARN_AND_DIALOG => {
            spec::RankPolicy::NotAllowedWarnAndDialog
        }
    }
}

/// Convert a `Contest`'s bounds and policies to a [`spec::Config`].
pub fn contest_config(
    contest: &Contest,
) -> Result<spec::Config, ValidationProviderError> {
    let (Ok(min), Ok(max)) = (
        u32::try_from(contest.min_votes),
        u32::try_from(contest.max_votes),
    ) else {
        return Err(ValidationProviderError::UnrepresentableBounds {
            min_votes: contest.min_votes,
            max_votes: contest.max_votes,
        });
    };
    let p = contest.presentation.as_ref();
    Ok(spec::Config {
        min,
        max,
        policies: spec::Policies {
            invalid: map_invalid(
                p.and_then(|p| p.invalid_vote_policy.clone())
                    .unwrap_or_default(),
            ),
            blank: map_blank(
                p.and_then(|p| p.blank_vote_policy.clone())
                    .unwrap_or_default(),
            ),
            over: map_over(
                p.and_then(|p| p.over_vote_policy).unwrap_or_default(),
            ),
            under: map_under(
                p.and_then(|p| p.under_vote_policy).unwrap_or_default(),
            ),
            dup: map_dup(
                p.and_then(|p| p.duplicated_rank_policy.clone())
                    .unwrap_or_default(),
            ),
            gap: map_gap(
                p.and_then(|p| p.preference_gaps_policy.clone())
                    .unwrap_or_default(),
            ),
        },
    })
}

/// Summarize the voter's selections on one decoded contest as a
/// [`spec::VoteState`]; see the module doc for the field-by-field rules.
pub fn vote_state(
    contest: &Contest,
    decoded: &DecodedVoteContest,
) -> spec::VoteState {
    let is_blank_marker = |id: &str| {
        contest
            .candidates
            .iter()
            .any(|c| c.id == id && c.is_explicit_blank())
    };
    let is_invalid_marker = |id: &str| {
        contest
            .candidates
            .iter()
            .any(|c| c.id == id && c.is_explicit_invalid())
    };

    let selected = |sel: i64| sel > -1;
    let regular_choices = decoded.choices.iter().filter(|ch| {
        selected(ch.selected)
            && !is_blank_marker(&ch.id)
            && !is_invalid_marker(&ch.id)
    });

    let regulars = regular_choices.clone().count() as u32;
    let blank_marker = decoded
        .choices
        .iter()
        .any(|ch| selected(ch.selected) && is_blank_marker(&ch.id));
    let explicit_invalid = decoded.is_explicit_invalid
        || decoded
            .choices
            .iter()
            .any(|ch| selected(ch.selected) && is_invalid_marker(&ch.id));

    let preferential = contest.get_counting_algorithm().is_preferential();
    let (first_preferences, duplicate_ranks, rank_gaps) = if preferential {
        let fp = regular_choices
            .clone()
            .filter(|ch| ch.selected == 0)
            .count() as u32;
        let (dup, gap) = match decoded.validate_preferencial_order() {
            Ok(()) => (false, false),
            Err(errors) => (
                errors
                    .contains(&PreferencialOrderErrorType::DuplicatedPosition),
                errors.contains(
                    &PreferencialOrderErrorType::PreferenceOrderWithGaps,
                ),
            ),
        };
        (Some(fp), dup, gap)
    } else {
        (None, false, false)
    };

    spec::VoteState {
        regulars,
        blank_marker,
        explicit_invalid,
        decline: decoded.is_decline_to_vote,
        duplicate_ranks,
        rank_gaps,
        first_preferences,
    }
}
