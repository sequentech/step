// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The injection layer: adapters from production's wire types to the
//! rationalized query-provider (`validation-spec`).
//!
//! `validation-spec` is deliberately abstract — its `Config` / `VoteState`
//! never mention `Contest` / `DecodedVoteContest`, which is what keeps that
//! crate independent of the code it specifies (the property that makes the
//! headless sweep meaningful). This crate is the other half: it owns the
//! sequent-core dependency and derives the abstract shapes from the wire
//! types, so a production call site can be answered by the provider:
//!
//! ```text
//!   &Contest                        → contest_config → Config
//!   &Contest, &DecodedVoteContest   → vote_state     → VoteState
//!   for_contest / for_vote / for_ballot → the provider's three validators
//! ```
//!
//! Every derivation rule below names the production line it mirrors; the
//! conformance test at the bottom is the acceptance: production's own decode
//! and gate functions, run natively over the bundled fixtures' contests and a
//! policy × vote-state matrix, must agree with the FROZEN ORACLE (`f`) fed
//! through these adapters — the native analogue of the wasm sweep, with the
//! adapters in the loop. (`f_fixed` is deliberately NOT the comparison
//! target: it diverges from production by the ledger's fixes.)
//!
//! Out of scope, named: `DecodedVoteContest::is_blank_ballot` (the
//! multi-ballot blank-ballots feature) has no `VoteState` counterpart — the
//! multi-contest codec lane is a standing scope boundary
//! (characterization/README.md, "Scope boundaries").

use sequent_core::ballot::{
    Contest, EBlankVotePolicy, EDuplicatedRankPolicy, EOverVotePolicy, EPreferenceGapsPolicy,
    EUnderVotePolicy, InvalidVotePolicy,
};
use sequent_core::plaintext::{DecodedVoteContest, PreferencialOrderErrorType};
use validation_spec as spec;
use validation_spec::{BallotValidator, ContestValidator, VoteValidator};

/// The configuration itself is unusable — production's
/// `check_max_min_votes_policy` sanity class ("config rejected"), which is
/// outside the mapping's domain: `f` is total over vote states, not over
/// malformed bounds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterError {
    /// `min_votes` / `max_votes` not representable as a count (negative or
    /// out of range) — mirrors `errors.encoding.invalidMinVotes` /
    /// `invalidMaxVotes`.
    UnrepresentableBounds { min_votes: i64, max_votes: i64 },
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdapterError::UnrepresentableBounds {
                min_votes,
                max_votes,
            } => write!(
                f,
                "contest bounds not representable: min_votes={min_votes}, max_votes={max_votes}"
            ),
        }
    }
}

impl std::error::Error for AdapterError {}

// ---------------------------------------------------------------------------
// Policy mapping — explicit matches, so a new upstream variant is a compile
// error here (the loud failure this layer wants), mirroring the wire-string
// parse failure the sweep gets.
// ---------------------------------------------------------------------------

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
        EBlankVotePolicy::WARN_ONLY_IN_REVIEW => spec::BlankVotePolicy::WarnOnlyInReview,
        EBlankVotePolicy::NOT_ALLOWED => spec::BlankVotePolicy::NotAllowed,
    }
}

fn map_over(p: EOverVotePolicy) -> spec::OverVotePolicy {
    match p {
        EOverVotePolicy::ALLOWED => spec::OverVotePolicy::Allowed,
        EOverVotePolicy::ALLOWED_WITH_MSG => spec::OverVotePolicy::AllowedWithMsg,
        EOverVotePolicy::ALLOWED_WITH_MSG_AND_ALERT => spec::OverVotePolicy::AllowedWithMsgAndAlert,
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
        EUnderVotePolicy::WARN_ONLY_IN_REVIEW => spec::UnderVotePolicy::WarnOnlyInReview,
        EUnderVotePolicy::WARN_AND_ALERT => spec::UnderVotePolicy::WarnAndAlert,
    }
}

fn map_dup(p: EDuplicatedRankPolicy) -> spec::RankPolicy {
    match p {
        EDuplicatedRankPolicy::ALLOWED_WARN_AND_DIALOG => spec::RankPolicy::AllowedWarnAndDialog,
        EDuplicatedRankPolicy::NOT_ALLOWED_WARN_AND_DIALOG => {
            spec::RankPolicy::NotAllowedWarnAndDialog
        }
    }
}

fn map_gap(p: EPreferenceGapsPolicy) -> spec::RankPolicy {
    match p {
        EPreferenceGapsPolicy::ALLOWED_WARN_AND_DIALOG => spec::RankPolicy::AllowedWarnAndDialog,
        EPreferenceGapsPolicy::NOT_ALLOWED_WARN_AND_DIALOG => {
            spec::RankPolicy::NotAllowedWarnAndDialog
        }
    }
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Derive the abstract `Config` from a production `Contest`.
///
/// Policy resolution mirrors the per-field `unwrap_or_default()` production
/// applies wherever the checkers and gates read a policy (`checker.rs`,
/// `voting_screen.rs`, `Contest::get_invalid_vote_policy`): an unset field is
/// the enum's platform default. (The booth's TypeScript message filter reads
/// the field raw — the unset-vs-allowed candidate noted in the workbench
/// README — but the Rust surfaces this crate injects into all resolve.)
///
/// Bounds mirror `check_max_min_votes_policy`'s convertibility sanity:
/// unrepresentable bounds are a config-rejection, not a vote-state effect.
pub fn contest_config(contest: &Contest) -> Result<spec::Config, AdapterError> {
    let (Ok(min), Ok(max)) = (
        u32::try_from(contest.min_votes),
        u32::try_from(contest.max_votes),
    ) else {
        return Err(AdapterError::UnrepresentableBounds {
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
                p.and_then(|p| p.over_vote_policy.clone())
                    .unwrap_or_default(),
            ),
            under: map_under(
                p.and_then(|p| p.under_vote_policy.clone())
                    .unwrap_or_default(),
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

// ---------------------------------------------------------------------------
// VoteState
// ---------------------------------------------------------------------------

/// Derive the abstract `VoteState` from a production `DecodedVoteContest`
/// against its `Contest`. Each field names the production rule it mirrors:
///
/// - `regulars` — selected (`selected > -1`) choices excluding BOTH marker
///   kinds. Production's checker count (`num_selected_candidates`,
///   raw_ballot.rs) excludes explicit-blank markers only, because on a
///   canonical decoded record the invalid marker never appears as a selected
///   choice (decode drops it into the flag — the route convergence recorded
///   in invalid-rule.md); excluding it here too makes the derivation correct
///   on pre-decode records as well, where the marker choice and the flag
///   coexist (the gates' double-count guard mirrors the same concern).
/// - `blank_marker` / `explicit_invalid` — the marker booleans;
///   `explicit_invalid` ORs the flag with a selected invalid-marker choice
///   (the convergence again).
/// - `first_preferences` — preferential contests only: the count of regular
///   choices at rank 0 (`selected == 0`) — the number the production GATES
///   count (S6). `None` on plurality, where every selection is at rank 0 and
///   the spec's `unwrap_or(regulars)` is exact.
/// - `duplicate_ranks` / `rank_gaps` — preferential contests only, computed
///   by production's own `validate_preferencial_order` (called, not
///   transcribed), exactly as `raw_ballot.rs::decode` gates it.
/// - `decline` — `is_decline_to_vote`. (`is_blank_ballot` has no
///   counterpart: multi-ballot lane, out of scope.)
pub fn vote_state(contest: &Contest, decoded: &DecodedVoteContest) -> spec::VoteState {
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
        selected(ch.selected) && !is_blank_marker(&ch.id) && !is_invalid_marker(&ch.id)
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
                errors.contains(&PreferencialOrderErrorType::DuplicatedPosition),
                errors.contains(&PreferencialOrderErrorType::PreferenceOrderWithGaps),
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

// ---------------------------------------------------------------------------
// The three validators, production-typed
// ---------------------------------------------------------------------------

/// Stage 0 from a production contest (config known).
pub fn for_contest(contest: &Contest) -> Result<ContestValidator, AdapterError> {
    Ok(ContestValidator::from_config(contest_config(contest)?))
}

/// Stage 1 from a production contest + decoded record (per edit / per decode).
pub fn for_vote(
    contest: &Contest,
    decoded: &DecodedVoteContest,
) -> Result<VoteValidator, AdapterError> {
    Ok(for_contest(contest)?.for_vote_state(vote_state(contest, decoded)))
}

/// The whole-ballot composition (the Next/review transition): one
/// (contest, decoded) pair per contest on the ballot, all pages.
pub fn for_ballot<'a>(
    pairs: impl IntoIterator<Item = (&'a Contest, &'a DecodedVoteContest)>,
) -> Result<BallotValidator, AdapterError> {
    let votes = pairs
        .into_iter()
        .map(|(c, d)| for_vote(c, d))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(BallotValidator::from_votes(votes))
}
