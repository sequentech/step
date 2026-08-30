// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Vote-validation rules, evaluated from a contest's configuration and a
//! summary of the voter's selections.
//!
//! Defining them once is what keeps the places that evaluate them from
//! disagreeing about the same ballot:
//!
//! - ballot decoding evaluates the policy rules and records the resulting
//!   errors and alerts on the decoded contest ([`policy_emissions`] —
//!   `ballot_codec/raw_ballot.rs`);
//! - the voting screen's Next gates evaluate the same rules, from the
//!   contest and the selections rather than from the record, to decide
//!   whether to block or ask for confirmation (`util/voting_screen.rs`);
//! - the booth evaluates the display rules over the recorded messages to
//!   decide which of them the voter sees on the screen being rendered
//!   ([`filter_visible_messages`], reached from TypeScript through the
//!   `filter_visible_messages_js` wasm export);
//! - the tally evaluates nothing of its own: it classifies from the
//!   decoded record's fields, so it follows decoding.
//!
//! Within one evaluation the vote-state facts are derived once and every
//! answer is a projection of that derivation, so a site cannot contradict
//! itself; across sites the rules are these, so two sites cannot reach
//! different conclusions about one ballot.
//!
//! Two stages, keyed on how much is known:
//!
//! - [`ContestValidator`] — configuration only (bounds and policies).
//! - [`VoteValidator`] — configuration plus one contest's [`VoteState`],
//!   its single selection count, and the messages derived from it.
//!
//! [`BallotValidator`] is the composition axis: the gates OR across the
//! ballot's contests. The display rules are not part of either stage:
//! they read a contest's policies and its messages but never its
//! selections ([`visible_messages`]), which is why the booth can
//! apply them to a decoded record it did not derive.
//!
//! Conversion from the ballot types is part of the module:
//! [`contest_config`] and [`vote_state`] are the only places validation
//! facts are read off a `Contest` / `DecodedVoteContest`:
//!
//! - an unset policy resolves to the enum's default;
//! - `regulars` counts selected choices (`selected > -1`), excluding the
//!   explicit-blank and explicit-invalid marker candidates;
//! - `explicit_invalid` is set by the decoded contest's
//!   `is_explicit_invalid` flag or by a selected explicit-invalid marker
//!   candidate — the two ways a ballot is marked invalid;
//! - on preferential contests `duplicate_ranks` / `rank_gaps` come from
//!   [`DecodedVoteContest::validate_preferencial_order`];
//! - invalid (negative or out-of-range) `min_votes` / `max_votes` are
//!   rejected as [`ValidationError::UnrepresentableBounds`]. Ballot
//!   decoding reports such bounds as encoding errors
//!   (`errors.encoding.invalidMinVotes` / `invalidMaxVotes`), which
//!   callers handle from `invalid_errors` (see the call sites).
//!
//! `DecodedVoteContest::is_blank_ballot` is not consulted: the rules
//! derive blankness from the selections themselves.

use std::collections::HashMap;

use crate::ballot::{
    Contest, EBlankVotePolicy, EDuplicatedRankPolicy, EOverVotePolicy,
    EPreferenceGapsPolicy, EUnderVotePolicy, InvalidVotePolicy,
};
use crate::ballot_codec::CheckerResult;
use crate::plaintext::{
    DecodedVoteContest, InvalidPlaintextError, InvalidPlaintextErrorType,
    PreferencialOrderErrorType,
};

pub const SELECTED_MAX: &str = "errors.implicit.selectedMax";
pub const SELECTED_MIN: &str = "errors.implicit.selectedMin";
pub const BLANK_VOTE: &str = "errors.implicit.blankVote";
pub const UNDER_VOTE: &str = "errors.implicit.underVote";
pub const OVER_VOTE_DISABLED: &str = "errors.implicit.overVoteDisabled";
pub const DUPLICATED_POSITION: &str = "errors.implicit.duplicatedPosition";
pub const PREFERENCE_ORDER_WITH_GAPS: &str =
    "errors.implicit.preferenceOrderWithGaps";
pub const EXPLICIT_NOT_ALLOWED: &str = "errors.explicit.notAllowed";
pub const EXPLICIT_ALERT: &str = "errors.explicit.alert";

/// Messages whose `error_type` is `Explicit` or `EncodingError` — the hard
/// gate blocks on any of them, whatever the policies say.
const EXPLICIT_OR_ENCODING: [&str; 5] = [
    EXPLICIT_NOT_ALLOWED,
    "errors.configuration.multipleExplicitInvalidCandidates",
    "errors.configuration.multipleExplicitBlankCandidates",
    "errors.encoding.invalidMinVotes",
    "errors.encoding.invalidMaxVotes",
];

/// A contest's validation policies, unset values resolved to the enum
/// defaults.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Policies {
    pub invalid: InvalidVotePolicy,
    pub blank: EBlankVotePolicy,
    pub over: EOverVotePolicy,
    pub under: EUnderVotePolicy,
    pub dup: EDuplicatedRankPolicy,
    pub gap: EPreferenceGapsPolicy,
}

/// The contest knobs the rules read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub min: u32,
    pub max: u32,
    pub policies: Policies,
}

/// What the voter did on one contest, independent of wire encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VoteState {
    /// Selected candidates, excluding the marker candidates.
    pub regulars: u32,
    /// The explicit-blank marker candidate is selected.
    pub blank_marker: bool,
    /// The ballot is explicitly invalid — via the flag or a selected
    /// explicit-invalid marker candidate.
    pub explicit_invalid: bool,
    /// The ballot-level decline-to-vote bit (multi-contest encodings only).
    pub decline: bool,
    /// Two candidates share a rank (preferential contests only).
    pub duplicate_ranks: bool,
    /// The ranking skips a rank (preferential contests only).
    pub rank_gaps: bool,
}

/// A contest configuration that cannot be converted for validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// `min_votes` / `max_votes` not representable as a count (negative or
    /// out of range) — the class `check_max_min_votes_policy` reports as
    /// `errors.encoding.invalidMinVotes` / `invalidMaxVotes`.
    UnrepresentableBounds { min_votes: i64, max_votes: i64 },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::UnrepresentableBounds {
                min_votes,
                max_votes,
            } => write!(
                f,
                "contest bounds not representable: min_votes={min_votes}, max_votes={max_votes}"
            ),
        }
    }
}

impl std::error::Error for ValidationError {}

/// The messages one contest's rules produce, as message keys: the errors
/// and alerts ballot decoding records (see [`policy_emissions`] for the
/// full `InvalidPlaintextError` form each becomes).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Emissions {
    pub errors: Vec<String>,
    pub alerts: Vec<String>,
}

/// Converts a `Contest`'s bounds and policies to a [`Config`].
pub fn contest_config(contest: &Contest) -> Result<Config, ValidationError> {
    let (Ok(min), Ok(max)) = (
        u32::try_from(contest.min_votes),
        u32::try_from(contest.max_votes),
    ) else {
        return Err(ValidationError::UnrepresentableBounds {
            min_votes: contest.min_votes,
            max_votes: contest.max_votes,
        });
    };
    let p = contest.presentation.as_ref();
    Ok(Config {
        min,
        max,
        policies: Policies {
            invalid: p
                .and_then(|p| p.invalid_vote_policy.clone())
                .unwrap_or_default(),
            blank: p
                .and_then(|p| p.blank_vote_policy.clone())
                .unwrap_or_default(),
            over: p.and_then(|p| p.over_vote_policy).unwrap_or_default(),
            under: p.and_then(|p| p.under_vote_policy).unwrap_or_default(),
            dup: p
                .and_then(|p| p.duplicated_rank_policy.clone())
                .unwrap_or_default(),
            gap: p
                .and_then(|p| p.preference_gaps_policy.clone())
                .unwrap_or_default(),
        },
    })
}

/// Summarizes the voter's selections on one decoded contest as a
/// [`VoteState`]; see the module doc for the field-by-field rules.
pub fn vote_state(
    contest: &Contest,
    decoded: &DecodedVoteContest,
) -> VoteState {
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
    let regulars = decoded
        .choices
        .iter()
        .filter(|ch| {
            selected(ch.selected)
                && !is_blank_marker(&ch.id)
                && !is_invalid_marker(&ch.id)
        })
        .count() as u32;
    let blank_marker = decoded
        .choices
        .iter()
        .any(|ch| selected(ch.selected) && is_blank_marker(&ch.id));
    let explicit_invalid = decoded.is_explicit_invalid
        || decoded
            .choices
            .iter()
            .any(|ch| selected(ch.selected) && is_invalid_marker(&ch.id));

    let (duplicate_ranks, rank_gaps) =
        if contest.get_counting_algorithm().is_preferential() {
            match decoded.validate_preferencial_order() {
                Ok(()) => (false, false),
                Err(errors) => (
                    errors.contains(
                        &PreferencialOrderErrorType::DuplicatedPosition,
                    ),
                    errors.contains(
                        &PreferencialOrderErrorType::PreferenceOrderWithGaps,
                    ),
                ),
            }
        } else {
            (false, false)
        };

    VoteState {
        regulars,
        blank_marker,
        explicit_invalid,
        decline: decoded.is_decline_to_vote,
        duplicate_ranks,
        rank_gaps,
    }
}

/// Counts the selections every count-based rule reads — regulars plus
/// each marker: a selected blank marker and a set invalid flag each count as
/// one selection. There is exactly one count, shared by the message rules
/// and the gates.
fn selections(vs: &VoteState) -> u32 {
    vs.regulars + u32::from(vs.blank_marker) + u32::from(vs.explicit_invalid)
}

/// Reports whether the ballot is an under-vote: non-empty, short of the
/// maximum but at least the minimum. One predicate, shared by the alert
/// and the gate. The
/// empty ballot is the blank rule's domain, not the under-vote rule's.
fn is_undervote(config: &Config, n: u32) -> bool {
    n > 0 && n >= config.min && n < config.max
}

/// Reports whether the ballot is a deliberate blank — its content is the
/// explicit-blank marker and nothing else. Explicit blank votes are not subject to the min-vote
/// rule — the voter's declared blank stands at any `min_votes`. The marker
/// still counts as a selection in the blank/over/under rules, and a ballot
/// that is also explicitly invalid is a null vote, not a blank.
fn is_deliberate_blank(vs: &VoteState) -> bool {
    vs.blank_marker && vs.regulars == 0 && !vs.explicit_invalid
}

/// Computes the messages for one ballot — invalid → over → min → under →
/// blank →
/// duplicated-rank → preference-gaps, the order decoding records them.
fn derive_emissions(config: &Config, vs: &VoteState, n: u32) -> Emissions {
    let p = &config.policies;
    let mut errors: Vec<String> = Vec::new();
    let mut alerts: Vec<String> = Vec::new();

    if vs.explicit_invalid {
        if p.invalid == InvalidVotePolicy::NOT_ALLOWED {
            errors.push(EXPLICIT_NOT_ALLOWED.into());
        }
        if p.invalid == InvalidVotePolicy::WARN_INVALID_IMPLICIT_AND_EXPLICIT {
            alerts.push(EXPLICIT_ALERT.into());
        }
    }
    // Over-vote: the error is unconditional; the policy governs the alert
    // and, at exactly max under the disable policy, the "maximum reached"
    // hint.
    if n > config.max {
        errors.push(SELECTED_MAX.into());
        if p.over != EOverVotePolicy::ALLOWED {
            alerts.push(SELECTED_MAX.into());
        }
    } else if n == config.max
        && p.over == EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_DISABLE
    {
        alerts.push(OVER_VOTE_DISABLED.into());
    }
    // Min-vote: a fixed rule with no policy of its own — always an error,
    // except that a deliberate blank is not subject to it.
    if n < config.min && !is_deliberate_blank(vs) {
        errors.push(SELECTED_MIN.into());
    }
    if is_undervote(config, n) && p.under != EUnderVotePolicy::ALLOWED {
        alerts.push(UNDER_VOTE.into());
    }
    // Blank: skipped entirely for an explicitly-invalid ballot.
    if n == 0 && !vs.explicit_invalid && p.blank != EBlankVotePolicy::ALLOWED {
        if p.blank == EBlankVotePolicy::NOT_ALLOWED {
            errors.push(BLANK_VOTE.into());
        } else {
            alerts.push(BLANK_VOTE.into());
        }
    }
    // Preferential rules — both policy variants emit identically; the
    // policy decides only which gate reacts. Duplicates before gaps, the
    // order `validate_preferencial_order` reports them.
    if vs.duplicate_ranks {
        errors.push(DUPLICATED_POSITION.into());
    }
    if vs.rank_gaps {
        errors.push(PREFERENCE_ORDER_WITH_GAPS.into());
    }
    Emissions { errors, alerts }
}

/// Reports whether the ballot must change before Next proceeds.
fn derive_hard_gate(config: &Config, n: u32, em: &Emissions) -> bool {
    let p = &config.policies;
    em.errors
        .iter()
        .any(|m| EXPLICIT_OR_ENCODING.contains(&m.as_str()))
        || (!em.errors.is_empty()
            && p.invalid == InvalidVotePolicy::NOT_ALLOWED)
        || (n == 0 && p.blank == EBlankVotePolicy::NOT_ALLOWED)
        || (n > config.max
            && p.over == EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_ALERT)
        || (p.dup == EDuplicatedRankPolicy::NOT_ALLOWED_WARN_AND_DIALOG
            && em.errors.iter().any(|m| m == DUPLICATED_POSITION))
        || (p.gap == EPreferenceGapsPolicy::NOT_ALLOWED_WARN_AND_DIALOG
            && em.errors.iter().any(|m| m == PREFERENCE_ORDER_WITH_GAPS))
}

/// Reports whether Next requires a confirmation dialog.
fn derive_soft_gate(
    config: &Config,
    vs: &VoteState,
    n: u32,
    em: &Emissions,
) -> bool {
    let p = &config.policies;
    (!em.errors.is_empty()
        && p.invalid != InvalidVotePolicy::ALLOWED
        && p.invalid != InvalidVotePolicy::ALLOWED_WITH_EXCLUSIVE_EXPLICIT)
        || (p.invalid == InvalidVotePolicy::WARN_INVALID_IMPLICIT_AND_EXPLICIT
            && vs.explicit_invalid)
        || (p.blank == EBlankVotePolicy::WARN && n == 0)
        || (n > config.max
            && p.over == EOverVotePolicy::ALLOWED_WITH_MSG_AND_ALERT)
        || (is_undervote(config, n)
            && p.under == EUnderVotePolicy::WARN_AND_ALERT)
        || (p.dup == EDuplicatedRankPolicy::ALLOWED_WARN_AND_DIALOG
            && em.errors.iter().any(|m| m == DUPLICATED_POSITION))
        || (p.gap == EPreferenceGapsPolicy::ALLOWED_WARN_AND_DIALOG
            && em.errors.iter().any(|m| m == PREFERENCE_ORDER_WITH_GAPS))
}

/// Stage 0 — the configuration is known.
pub struct ContestValidator {
    config: Config,
}

impl ContestValidator {
    pub fn from_config(config: Config) -> Self {
        ContestValidator { config }
    }

    /// Fixes the vote-state facts once — the single selection count and the
    /// messages — and hand back the stage-1 validator. Everything
    /// downstream reads this one derivation.
    pub fn for_vote_state(&self, vs: VoteState) -> VoteValidator {
        let n = selections(&vs);
        let em = derive_emissions(&self.config, &vs, n);
        VoteValidator {
            config: self.config.clone(),
            vs,
            n,
            em,
        }
    }
}

/// Stage 1 — configuration plus one contest's derived [`VoteState`], its
/// selection count, and the messages derived from them.
pub struct VoteValidator {
    config: Config,
    vs: VoteState,
    n: u32,
    em: Emissions,
}

impl VoteValidator {
    /// Returns the messages this ballot produces, as message keys.
    pub fn emissions(&self) -> &Emissions {
        &self.em
    }

    /// Reports whether this contest blocks Next.
    pub fn hard_gate(&self) -> bool {
        derive_hard_gate(&self.config, self.n, &self.em)
    }

    /// Reports whether this contest asks for confirmation at Next.
    pub fn soft_gate(&self) -> bool {
        derive_soft_gate(&self.config, &self.vs, self.n, &self.em)
    }

    /// Returns this contest's configuration — the policies the display
    /// rules read ([`visible_messages`]) alongside these messages.
    pub fn config(&self) -> &Config {
        &self.config
    }
}

/// Selects the alerts the voter should see on one screen, keeping the
/// record's order.
///
/// Two kinds of rule apply. **Visibility** depends on which screen is
/// showing: the warn-only-in-review policies hold their message back until
/// review, and the "maximum reached" hint is a voting-screen aid that
/// review does not repeat. **Deduplication** then drops what would be
/// redundant: an empty ballot shows the blank message rather than the
/// under-vote hint, and an alert whose message already appears as an error
/// is dropped because errors are shown first.
///
/// Errors are not filtered. Whatever affects how the ballot will be
/// counted is shown to the voter; the invalid-vote policy's role is the
/// confirmation/blocking ladder at Next, not what the voter is told.
fn visible_alert_keys(
    policies: &Policies,
    error_keys: &[&str],
    alert_keys: &[&str],
    is_review: bool,
) -> Vec<String> {
    let visible: Vec<&str> = alert_keys
        .iter()
        .copied()
        .filter(|key| {
            !((*key == UNDER_VOTE
                && !is_review
                && policies.under == EUnderVotePolicy::WARN_ONLY_IN_REVIEW)
                || (*key == BLANK_VOTE
                    && !is_review
                    && policies.blank == EBlankVotePolicy::WARN_ONLY_IN_REVIEW)
                || (*key == OVER_VOTE_DISABLED && is_review))
        })
        .collect();
    let blank_vote_present =
        visible.contains(&BLANK_VOTE) || error_keys.contains(&BLANK_VOTE);
    visible
        .into_iter()
        .filter(|key| {
            !((*key == UNDER_VOTE && blank_vote_present)
                || error_keys.contains(key))
        })
        .map(str::to_string)
        .collect()
}

/// Returns the messages that reach the voter on one screen, as the errors
/// and the alerts they will be shown as: every error (whatever affects how
/// the ballot will be counted is shown — the invalid-vote policy governs
/// the confirmation/blocking ladder at Next, not what the voter is told)
/// and the surviving alerts (see [`visible_alert_keys`]). An untouched
/// contest shows nothing until the voter has selected something; the
/// review screen always shows.
///
/// Note what this reads: the messages, two of the contest's policies, and
/// which screen is showing. It does NOT read the voter's selections — the
/// messages already carry everything the display rules need — so two
/// ballots producing the same messages under the same policies always show
/// the same thing.
pub fn visible_messages(
    policies: &Policies,
    errors: &[String],
    alerts: &[String],
    is_review: bool,
    is_touched: bool,
) -> (Vec<String>, Vec<String>) {
    if !is_review && !is_touched {
        return (Vec::new(), Vec::new());
    }
    let error_keys: Vec<&str> = errors.iter().map(String::as_str).collect();
    let alert_keys: Vec<&str> = alerts.iter().map(String::as_str).collect();
    (
        errors.to_vec(),
        visible_alert_keys(policies, &error_keys, &alert_keys, is_review),
    )
}

/// The composition axis: the gates OR across every contest on the ballot.
pub struct BallotValidator {
    contests: Vec<VoteValidator>,
}

impl BallotValidator {
    pub fn from_votes(contests: Vec<VoteValidator>) -> Self {
        BallotValidator { contests }
    }

    /// Reports whether ANY contest blocks Next.
    pub fn hard_gate(&self) -> bool {
        self.contests.iter().any(VoteValidator::hard_gate)
    }

    /// Reports whether ANY contest asks for confirmation.
    pub fn soft_gate(&self) -> bool {
        self.contests.iter().any(VoteValidator::soft_gate)
    }
}

/// Computes one message for the `invalid_errors` / `invalid_alerts` that
/// [`policy_emissions`] produces, returned as an [`InvalidPlaintextError`].
/// Besides the message key it carries the parameters the message's
/// translation interpolates — `numSelected` is the marker-inclusive
/// selection count, `min` / `max` the contest bounds — and `type: "alert"`
/// on the alert-style messages. Explicit invalidity is typed `Explicit`,
/// everything else `Implicit`.
///
/// These shapes are also what `ballot_codec/checker.rs` produces for the
/// multi-contest codec, so a voter meets the same message either way.
fn plaintext_error(
    key: &str,
    n: u32,
    min: u32,
    max: u32,
) -> InvalidPlaintextError {
    let num_selected = || ("numSelected".to_string(), n.to_string());
    let alert_type = || ("type".to_string(), "alert".to_string());
    let (error_type, message_map) = match key {
        EXPLICIT_NOT_ALLOWED | EXPLICIT_ALERT => {
            (InvalidPlaintextErrorType::Explicit, HashMap::new())
        }
        SELECTED_MAX => (
            InvalidPlaintextErrorType::Implicit,
            HashMap::from([
                num_selected(),
                ("max".to_string(), max.to_string()),
            ]),
        ),
        OVER_VOTE_DISABLED => (
            InvalidPlaintextErrorType::Implicit,
            HashMap::from([
                alert_type(),
                num_selected(),
                ("max".to_string(), max.to_string()),
            ]),
        ),
        SELECTED_MIN => (
            InvalidPlaintextErrorType::Implicit,
            HashMap::from([
                num_selected(),
                ("min".to_string(), min.to_string()),
            ]),
        ),
        UNDER_VOTE => (
            InvalidPlaintextErrorType::Implicit,
            HashMap::from([
                alert_type(),
                num_selected(),
                ("min".to_string(), min.to_string()),
                ("max".to_string(), max.to_string()),
            ]),
        ),
        BLANK_VOTE => (
            InvalidPlaintextErrorType::Implicit,
            HashMap::from([alert_type(), num_selected()]),
        ),
        // duplicatedPosition and preferenceOrderWithGaps carry no
        // parameters; the arm also covers any future key conservatively
        // (an Implicit record with the key and no parameters).
        _ => (InvalidPlaintextErrorType::Implicit, HashMap::new()),
    };
    InvalidPlaintextError {
        error_type,
        candidate_id: None,
        message: Some(key.to_string()),
        message_map,
    }
}

/// Computes the policy-driven messages for one contest — the errors and
/// alerts the validation rules produce from the contest configuration and the
/// decoded selections, in the order ballot decoding appends them. Encoding
/// and configuration errors are not produced here; decoding stamps those on
/// the record itself. Fails when `min_votes`/`max_votes` cannot be
/// interpreted as counts — the caller keeps the per-bound checks for that
/// case (each check runs with only the bounds it needs).
pub fn policy_emissions(
    contest: &Contest,
    decoded: &DecodedVoteContest,
) -> Result<CheckerResult, ValidationError> {
    let config = contest_config(contest)?;
    let (min, max) = (config.min, config.max);
    let vs = vote_state(contest, decoded);
    let n = selections(&vs);
    let validator = ContestValidator::from_config(config).for_vote_state(vs);
    let emissions = validator.emissions();
    Ok(CheckerResult {
        invalid_errors: emissions
            .errors
            .iter()
            .map(|key| plaintext_error(key, n, min, max))
            .collect(),
        invalid_alerts: emissions
            .alerts
            .iter()
            .map(|key| plaintext_error(key, n, min, max))
            .collect(),
    })
}

/// Returns the decoded contest reduced to the messages the voter should
/// see on one screen: the same record with `invalid_errors` and
/// `invalid_alerts` filtered. `is_review` selects the review screen over the voting screen;
/// `is_touched` is whether the voter has selected anything in this contest
/// yet (an untouched contest shows nothing until they have).
///
/// This reads only the record and the contest's blank and under-vote
/// policies — no bounds, no selection count — so it cannot fail on a
/// misconfigured contest, and encoding errors recorded by decoding are
/// shown like any other error.
pub fn filter_visible_messages(
    contest: &Contest,
    decoded: &DecodedVoteContest,
    is_review: bool,
    is_touched: bool,
) -> DecodedVoteContest {
    let p = contest.presentation.as_ref();
    let policies = Policies {
        blank: p
            .and_then(|p| p.blank_vote_policy.clone())
            .unwrap_or_default(),
        under: p.and_then(|p| p.under_vote_policy).unwrap_or_default(),
        ..Policies::default()
    };
    let key_of =
        |e: &InvalidPlaintextError| e.message.clone().unwrap_or_default();
    let error_keys: Vec<String> =
        decoded.invalid_errors.iter().map(key_of).collect();
    let alert_keys: Vec<String> =
        decoded.invalid_alerts.iter().map(key_of).collect();
    let (kept_errors, kept_alerts) = visible_messages(
        &policies,
        &error_keys,
        &alert_keys,
        is_review,
        is_touched,
    );

    let mut visible = decoded.clone();
    visible
        .invalid_errors
        .retain(|error| kept_errors.contains(&key_of(error)));
    visible
        .invalid_alerts
        .retain(|alert| kept_alerts.contains(&key_of(alert)));
    visible
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(min: u32, max: u32) -> Config {
        Config {
            min,
            max,
            policies: Policies::default(),
        }
    }

    fn vote(vs: VoteState, min: u32, max: u32) -> VoteValidator {
        ContestValidator::from_config(config(min, max)).for_vote_state(vs)
    }

    /// Returns what the voter sees on one screen: the display rules read
    /// this contest's policies and messages — never its selections.
    fn shown(
        v: &VoteValidator,
        is_review: bool,
        is_touched: bool,
    ) -> Vec<String> {
        let (errors, alerts) = visible_messages(
            &v.config().policies,
            &v.emissions().errors,
            &v.emissions().alerts,
            is_review,
            is_touched,
        );
        errors.into_iter().chain(alerts).collect()
    }

    #[test]
    fn deliberate_blank_is_not_subject_to_the_min_vote_rule() {
        let blank = VoteState {
            blank_marker: true,
            ..VoteState::default()
        };
        let v = vote(blank, 2, 3);
        assert!(v.emissions().errors.is_empty());
        assert!(!v.hard_gate() && !v.soft_gate());

        // A null vote (explicitly invalid) is not a blank: the min-vote
        // rule applies.
        let null = VoteState {
            explicit_invalid: true,
            ..VoteState::default()
        };
        let v = vote(null, 2, 3);
        assert!(v.emissions().errors.iter().any(|m| m == SELECTED_MIN));
    }

    #[test]
    fn the_empty_ballot_is_not_an_undervote() {
        let mut c = config(0, 2);
        c.policies.under = EUnderVotePolicy::WARN_AND_ALERT;
        let v = ContestValidator::from_config(c)
            .for_vote_state(VoteState::default());
        assert!(!v.emissions().alerts.iter().any(|m| m == UNDER_VOTE));
        assert!(!v.soft_gate());
    }

    #[test]
    fn gate_and_checker_share_one_count() {
        // Two ranked selections with max 1: the checker emits the
        // over-vote error and the gate blocks from the same count.
        let mut c = config(0, 1);
        c.policies.over = EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_ALERT;
        let v = ContestValidator::from_config(c).for_vote_state(VoteState {
            regulars: 2,
            ..VoteState::default()
        });
        assert!(v.emissions().errors.iter().any(|m| m == SELECTED_MAX));
        assert!(v.hard_gate());
    }

    #[test]
    fn every_error_is_shown_to_the_voter() {
        // Below the minimum with the invalid-vote policy at its permissive
        // default: the voter is told, on both screens.
        let v = vote(
            VoteState {
                regulars: 1,
                ..VoteState::default()
            },
            2,
            3,
        );
        assert!(shown(&v, false, true).iter().any(|m| m == SELECTED_MIN));
        assert!(shown(&v, true, true).iter().any(|m| m == SELECTED_MIN));
        // But not before the voter has touched the contest.
        assert!(shown(&v, false, false).is_empty());
    }

    #[test]
    fn warn_only_in_review_holds_the_alert_back_until_review() {
        let mut c = config(0, 2);
        c.policies.under = EUnderVotePolicy::WARN_ONLY_IN_REVIEW;
        let v = ContestValidator::from_config(c).for_vote_state(VoteState {
            regulars: 1,
            ..VoteState::default()
        });
        assert!(v.emissions().alerts.iter().any(|m| m == UNDER_VOTE));
        assert!(!shown(&v, false, true).iter().any(|m| m == UNDER_VOTE));
        assert!(shown(&v, true, true).iter().any(|m| m == UNDER_VOTE));
    }

    #[test]
    fn an_alert_that_already_renders_as_an_error_is_dropped() {
        // Over the maximum: the checker emits selectedMax as BOTH an error
        // and an alert; the voter sees it once.
        let mut c = config(0, 1);
        c.policies.over = EOverVotePolicy::ALLOWED_WITH_MSG;
        let v = ContestValidator::from_config(c).for_vote_state(VoteState {
            regulars: 2,
            ..VoteState::default()
        });
        assert!(v.emissions().errors.iter().any(|m| m == SELECTED_MAX));
        assert!(v.emissions().alerts.iter().any(|m| m == SELECTED_MAX));
        let visible = shown(&v, false, true);
        assert_eq!(visible.iter().filter(|m| *m == SELECTED_MAX).count(), 1);
    }

    #[test]
    fn filtering_a_record_keeps_every_error_including_encoding_errors() {
        let contest = Contest::default();
        let decoded = DecodedVoteContest {
            contest_id: contest.id.clone(),
            is_explicit_invalid: false,
            is_decline_to_vote: false,
            is_blank_ballot: false,
            invalid_errors: vec![InvalidPlaintextError {
                error_type: InvalidPlaintextErrorType::EncodingError,
                candidate_id: None,
                message: Some("errors.encoding.ballotTooLarge".to_string()),
                message_map: HashMap::new(),
            }],
            invalid_alerts: vec![],
            choices: vec![],
        };
        let visible = filter_visible_messages(&contest, &decoded, false, true);
        assert_eq!(visible.invalid_errors.len(), 1);
        // Untouched, the voting screen shows nothing at all.
        let untouched =
            filter_visible_messages(&contest, &decoded, false, false);
        assert!(untouched.invalid_errors.is_empty());
    }

    #[test]
    fn ballot_gate_is_the_or_across_contests() {
        let clean = vote(
            VoteState {
                regulars: 1,
                ..VoteState::default()
            },
            1,
            2,
        );
        let mut blocking_config = config(0, 2);
        blocking_config.policies.blank = EBlankVotePolicy::NOT_ALLOWED;
        let blocking = ContestValidator::from_config(blocking_config)
            .for_vote_state(VoteState::default());
        assert!(!clean.hard_gate());
        assert!(blocking.hard_gate());
        assert!(BallotValidator::from_votes(vec![clean, blocking]).hard_gate());
    }
}
