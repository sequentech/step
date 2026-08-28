// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Derivations from the wire types (`Contest`, `DecodedVoteContest`) to the
//! rationalized vote-validation spec's abstract shapes (`validation-spec`:
//! `Config`, `VoteState`) — the injection point through which production
//! call sites consult the query-provider instead of re-deriving validation
//! facts locally. The first injected site is the submission gates
//! (`util/voting_screen.rs`).
//!
//! Every derivation rule mirrors a named production behaviour:
//!
//! - policy resolution is the per-field `unwrap_or_default()` the checkers
//!   and gates already apply — an unset policy is the enum's default;
//! - `regulars` counts selected (`selected > -1`) choices excluding both
//!   marker kinds; `explicit_invalid` ORs the ballot flag with a selected
//!   explicit-invalid marker choice (the two routes to explicit invalidity
//!   converge at decode, and the old gates carried a double-count guard for
//!   exactly this — both are subsumed by deriving once here);
//! - `first_preferences` (preferential contests only) is the count of
//!   regular choices at rank 0 — the number the pre-injection gates counted;
//! - `duplicate_ranks` / `rank_gaps` call
//!   [`DecodedVoteContest::validate_preferencial_order`] — the same
//!   function `ballot_codec/raw_ballot.rs` uses — gated on
//!   `is_preferential()` exactly as decode gates it;
//! - unrepresentable `min_votes` / `max_votes` are a configuration
//!   rejection (`check_max_min_votes_policy`'s sanity class), not a
//!   vote-state effect: the caller keeps its record-driven handling for
//!   the encoding-error class (see the gates).
//!
//! `DecodedVoteContest::is_blank_ballot` (the multi-contest blank-ballots
//! feature) has no `VoteState` counterpart; the multi-contest codec lane is
//! outside the spec's certified domain.

use crate::ballot::{
    Contest, EBlankVotePolicy, EDuplicatedRankPolicy, EOverVotePolicy,
    EPreferenceGapsPolicy, EUnderVotePolicy, InvalidVotePolicy,
};
use crate::plaintext::{DecodedVoteContest, PreferencialOrderErrorType};
use validation_spec as spec;

/// The contest configuration itself is unusable — outside the validation
/// mapping's domain.
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

/// Derive the spec's `Config` from a `Contest`.
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

/// Derive the spec's `VoteState` from a `DecodedVoteContest` against its
/// `Contest`. See the module doc for the production rule each field mirrors.
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
