// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Vote validation. See docs/VALIDATION.md for what these rules
//! produce, where the rest of the system asks for them, and how to
//! analyse them.
//!
//! The rules themselves, evaluated from a contest's configuration and a
//! summary of the voter's selections.
//!
//! Defining them once is what keeps the places that evaluate them from
//! disagreeing about the same ballot:
//!
//! - ballot decoding evaluates the policy rules and records the resulting
//!   errors and alerts on the decoded contest
//!   ([`ContestValidator::messages`] — `ballot_codec/raw_ballot.rs`);
//! - the voting screen's Next gates evaluate the same rules, from the
//!   contest and the selections rather than from the record, to decide
//!   whether to block or ask for confirmation (`util/voting_screen.rs`);
//! - the booth evaluates the display rules over the recorded messages to
//!   decide which of them the voter sees on the screen being rendered
//!   ([`ContestValidator::filter_visible_messages`], reached from
//!   TypeScript through the
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
//! Reading the ballot types is part of the module:
//! [`ContestValidator::for_contest`] and
//! [`ContestValidator::vote_state`] are the only places validation facts
//! are read off a `Contest` / `DecodedVoteContest`:
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

use std::collections::{HashMap, HashSet};

use crate::ballot::{
    Contest, EBlankVotePolicy, EDuplicatedRankPolicy, EOverVotePolicy,
    EPreferenceGapsPolicy, EUnderVotePolicy, InvalidVotePolicy,
};
use crate::ballot_codec::multi_ballot::votable_contests;
use crate::ballot_codec::CheckerResult;
use crate::plaintext::{
    DecodedVoteChoice, DecodedVoteContest, InvalidPlaintextError,
    InvalidPlaintextErrorType, PreferencialOrderErrorType,
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
/// and alerts ballot decoding records (see [`ContestValidator::messages`]
/// for the full `InvalidPlaintextError` form each becomes).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Emissions {
    pub errors: Vec<String>,
    pub alerts: Vec<String>,
}

/// What a contest's candidates mean when reading a ballot: which ids are
/// the two marker kinds, and whether the selections carry ranks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ContestShape {
    blank_markers: HashSet<String>,
    invalid_markers: HashSet<String>,
    preferential: bool,
}

impl ContestShape {
    fn of(contest: &Contest) -> Self {
        let ids = |explicit_blank: bool| {
            contest
                .candidates
                .iter()
                .filter(|c| {
                    if explicit_blank {
                        c.is_explicit_blank()
                    } else {
                        c.is_explicit_invalid()
                    }
                })
                .map(|c| c.id.clone())
                .collect()
        };
        ContestShape {
            blank_markers: ids(true),
            invalid_markers: ids(false),
            preferential: contest.get_counting_algorithm().is_preferential(),
        }
    }
}

/// Reads a contest's policies, resolving each unset one to its default.
fn contest_policies(contest: &Contest) -> Policies {
    // A contest with no presentation is read as the default presentation,
    // which names its own value for four of these policies rather than
    // leaving them to the enums' defaults. The two disagree about the
    // over-vote policy, so going through `ContestPresentation` matters:
    // without it, a contest that configures no presentation would start
    // raising over-vote alerts it never raised.
    let p = contest.presentation.clone().unwrap_or_default();
    Policies {
        invalid: p.invalid_vote_policy.unwrap_or_default(),
        blank: p.blank_vote_policy.unwrap_or_default(),
        over: p.over_vote_policy.unwrap_or_default(),
        under: p.under_vote_policy.unwrap_or_default(),
        dup: p.duplicated_rank_policy.unwrap_or_default(),
        gap: p.preference_gaps_policy.unwrap_or_default(),
    }
}

/// Reads a contest's bounds as counts, or says why they cannot be used.
fn contest_bounds(contest: &Contest) -> Result<(u32, u32), ValidationError> {
    match (
        u32::try_from(contest.min_votes),
        u32::try_from(contest.max_votes),
    ) {
        (Ok(min), Ok(max)) => Ok((min, max)),
        _ => Err(ValidationError::UnrepresentableBounds {
            min_votes: contest.min_votes,
            max_votes: contest.max_votes,
        }),
    }
}

/// One edit a voter can make to a contest's selections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionEdit {
    /// Select, deselect or rank one candidate — including a marker, which
    /// is an ordinary choice as far as the wire format is concerned.
    Choice(DecodedVoteChoice),
    /// Mark the contest explicitly invalid, or stop doing so.
    ExplicitInvalid(bool),
}

/// What a ballot's selections look like to the tally: the explicit-blank
/// marker is the one candidate the classifier treats apart, so what
/// matters is whether it is selected, whether anything else is, or both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionClass {
    /// Nothing selected.
    None,
    /// Only ordinary candidates.
    Regular,
    /// Only the explicit-blank marker.
    Marker,
    /// The marker together with ordinary candidates.
    Mixed,
}

/// How a cast ballot counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BallotClass {
    ExplicitInvalid,
    ImplicitInvalid,
    ExplicitBlank,
    ImplicitBlank,
    Declined,
    Valid,
}

/// Classifies a cast ballot, by a strict precedence.
///
/// A decline is intentionally empty, so it outranks everything — but a
/// declined ballot carrying content is not a decline at all. Invalidity
/// comes next, and this is where the messages reach the count: a ballot is
/// invalid if it was marked so or if it carries ANY error, whatever the
/// error was about. Only then do the selections speak for themselves: the
/// blank marker alongside real selections contradicts itself, the marker
/// alone is a declared blank, nothing selected is an undeclared one, and
/// anything else is a valid vote.
///
/// Note what the second step means for the rules above: whether a ballot
/// counts turns on whether the rules emitted an error, not on which one.
/// A rule that stops emitting for a case — as the min-vote rule does for a
/// deliberate blank — moves that ballot from invalid to whatever its
/// selections say it is.
pub fn classify(
    decline: bool,
    explicit_invalid: bool,
    has_errors: bool,
    selection: SelectionClass,
) -> BallotClass {
    let invalid = explicit_invalid || has_errors;
    if decline {
        if !invalid && selection == SelectionClass::None {
            BallotClass::Declined
        } else {
            BallotClass::ImplicitInvalid
        }
    } else if invalid {
        if explicit_invalid {
            BallotClass::ExplicitInvalid
        } else {
            BallotClass::ImplicitInvalid
        }
    } else {
        match selection {
            SelectionClass::Mixed => BallotClass::ImplicitInvalid,
            SelectionClass::Marker => BallotClass::ExplicitBlank,
            SelectionClass::None => BallotClass::ImplicitBlank,
            SelectionClass::Regular => BallotClass::Valid,
        }
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

/// Reports whether the selection rules apply to this ballot. A decline
/// is intentionally empty in every contest — it says the voter is not
/// choosing, rather than choosing badly — so the over-, min-, under- and
/// blank-vote rules have nothing to judge. The explicit-invalid and
/// encoding rules still apply, and a decline carrying content is caught
/// by the tally classifier rather than by these.
fn selection_rules_apply(vs: &VoteState) -> bool {
    !vs.decline
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
    // The preferential rules below sit outside this guard because they
    // cannot fire on an empty ballot.
    if selection_rules_apply(vs) {
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
        if n == 0
            && !vs.explicit_invalid
            && p.blank != EBlankVotePolicy::ALLOWED
        {
            if p.blank == EBlankVotePolicy::NOT_ALLOWED {
                errors.push(BLANK_VOTE.into());
            } else {
                alerts.push(BLANK_VOTE.into());
            }
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
fn derive_hard_gate(
    config: &Config,
    vs: &VoteState,
    n: u32,
    em: &Emissions,
) -> bool {
    let p = &config.policies;
    let selections = selection_rules_apply(vs);
    em.errors
        .iter()
        .any(|m| EXPLICIT_OR_ENCODING.contains(&m.as_str()))
        || (!em.errors.is_empty()
            && p.invalid == InvalidVotePolicy::NOT_ALLOWED)
        || (selections && n == 0 && p.blank == EBlankVotePolicy::NOT_ALLOWED)
        || (selections
            && n > config.max
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
    let selections = selection_rules_apply(vs);
    (!em.errors.is_empty()
        && p.invalid != InvalidVotePolicy::ALLOWED
        && p.invalid != InvalidVotePolicy::ALLOWED_WITH_EXCLUSIVE_EXPLICIT)
        || (p.invalid == InvalidVotePolicy::WARN_INVALID_IMPLICIT_AND_EXPLICIT
            && vs.explicit_invalid)
        || (selections && p.blank == EBlankVotePolicy::WARN && n == 0)
        || (selections
            && n > config.max
            && p.over == EOverVotePolicy::ALLOWED_WITH_MSG_AND_ALERT)
        || (selections
            && is_undervote(config, n)
            && p.under == EUnderVotePolicy::WARN_AND_ALERT)
        || (p.dup == EDuplicatedRankPolicy::ALLOWED_WARN_AND_DIALOG
            && em.errors.iter().any(|m| m == DUPLICATED_POSITION))
        || (p.gap == EPreferenceGapsPolicy::ALLOWED_WARN_AND_DIALOG
            && em.errors.iter().any(|m| m == PREFERENCE_ORDER_WITH_GAPS))
}

/// Stage 0 — what is known before the voter has done anything: the
/// contest's policies, its bounds, and what its candidates mean.
///
/// Building one never fails. A contest whose `min_votes` / `max_votes`
/// cannot be read as counts can still say what the voter sees on screen;
/// it is the questions that compare a selection count against those
/// bounds — the messages and the gates — that cannot be answered, and
/// those return [`ValidationError::UnrepresentableBounds`].
pub struct ContestValidator {
    policies: Policies,
    bounds: Result<(u32, u32), ValidationError>,
    shape: ContestShape,
}

impl ContestValidator {
    /// Reads a contest.
    pub fn for_contest(contest: &Contest) -> Self {
        ContestValidator {
            policies: contest_policies(contest),
            bounds: contest_bounds(contest),
            shape: ContestShape::of(contest),
        }
    }

    /// Takes the policies and bounds directly, for callers that have them
    /// abstractly and supply the [`VoteState`] themselves. A validator
    /// built this way knows nothing of the contest's candidates, so it
    /// cannot summarize a decoded record — use [`Self::for_contest`] for
    /// that.
    pub fn from_config(config: Config) -> Self {
        ContestValidator {
            policies: config.policies,
            bounds: Ok((config.min, config.max)),
            shape: ContestShape::default(),
        }
    }

    /// Returns the contest's policies — what the display rules read
    /// ([`visible_messages`]) alongside a ballot's messages.
    pub fn policies(&self) -> &Policies {
        &self.policies
    }

    /// Returns the bounds and policies together, or says why the bounds
    /// cannot be used.
    pub fn config(&self) -> Result<Config, ValidationError> {
        let (min, max) = self.bounds.clone()?;
        Ok(Config {
            min,
            max,
            policies: self.policies.clone(),
        })
    }

    /// Summarizes the voter's selections on one decoded contest; see the
    /// module doc for the field-by-field rules.
    pub fn vote_state(&self, decoded: &DecodedVoteContest) -> VoteState {
        let selected = |sel: i64| sel > -1;
        let is_marker = |id: &String| {
            self.shape.blank_markers.contains(id)
                || self.shape.invalid_markers.contains(id)
        };
        let regulars = decoded
            .choices
            .iter()
            .filter(|ch| selected(ch.selected) && !is_marker(&ch.id))
            .count() as u32;
        let blank_marker = decoded.choices.iter().any(|ch| {
            selected(ch.selected) && self.shape.blank_markers.contains(&ch.id)
        });
        // A contest may present the explicit-invalid marker as a candidate.
        // Both codecs fold that candidate into the flag when encoding and
        // drop it when decoding, and the booth sets the flag directly, so a
        // record normally carries the flag alone; reading both forms keeps
        // the rules agreeing with whichever one reaches them.
        let explicit_invalid = decoded.is_explicit_invalid
            || decoded.choices.iter().any(|ch| {
                selected(ch.selected)
                    && self.shape.invalid_markers.contains(&ch.id)
            });

        let (duplicate_ranks, rank_gaps) = if self.shape.preferential {
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

    /// Fixes the vote-state facts once — the single selection count and the
    /// messages — and hands back the stage-1 validator. Everything
    /// downstream reads this one derivation.
    pub fn for_vote_state(
        &self,
        vs: VoteState,
    ) -> Result<VoteValidator, ValidationError> {
        let (min, max) = self.bounds.clone()?;
        let config = Config {
            min,
            max,
            policies: self.policies.clone(),
        };
        let n = selections(&vs);
        let em = derive_emissions(&config, &vs, n);
        Ok(VoteValidator { config, vs, n, em })
    }

    /// The same, starting from a decoded record rather than a summary.
    pub fn for_decoded(
        &self,
        decoded: &DecodedVoteContest,
    ) -> Result<VoteValidator, ValidationError> {
        self.for_vote_state(self.vote_state(decoded))
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
    pub fn messages(&self) -> &Emissions {
        &self.em
    }

    /// Returns the same messages in the form ballot decoding records on a
    /// contest: each key as an `InvalidPlaintextError` carrying the
    /// parameters its translation needs.
    pub fn recorded(&self) -> CheckerResult {
        let records = |keys: &[String]| {
            keys.iter()
                .map(|key| {
                    plaintext_error(
                        key,
                        self.n,
                        self.config.min,
                        self.config.max,
                    )
                })
                .collect()
        };
        CheckerResult {
            invalid_errors: records(&self.em.errors),
            invalid_alerts: records(&self.em.alerts),
        }
    }

    /// Reports whether this contest blocks Next.
    pub fn hard_gate(&self) -> bool {
        derive_hard_gate(&self.config, &self.vs, self.n, &self.em)
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
    messages: &Emissions,
    is_review: bool,
    is_touched: bool,
) -> Emissions {
    if !is_review && !is_touched {
        return Emissions::default();
    }
    let error_keys: Vec<&str> =
        messages.errors.iter().map(String::as_str).collect();
    let alert_keys: Vec<&str> =
        messages.alerts.iter().map(String::as_str).collect();
    Emissions {
        errors: messages.errors.clone(),
        alerts: visible_alert_keys(
            policies,
            &error_keys,
            &alert_keys,
            is_review,
        ),
    }
}

/// The composition axis: the gates OR across every contest on the ballot.
pub struct BallotValidator {
    gates: Vec<(bool, bool)>,
}

impl BallotValidator {
    /// Reads every contest of a ballot against its decoded record.
    ///
    /// Acclaimed contests are skipped: they have no selectable options, so
    /// a selection policy such as a minimum number of votes could never be
    /// satisfied and would block the voter for good. A contest with no
    /// decoded record blocks Next — an incomplete validation map is not
    /// proof that a contest is valid — but raises no dialog, since there is
    /// nothing to describe to the voter.
    pub fn for_ballot(
        contests: &[Contest],
        decoded: &HashMap<String, DecodedVoteContest>,
    ) -> Self {
        BallotValidator {
            gates: votable_contests(contests)
                .map(|contest| match decoded.get(&contest.id) {
                    Some(record) => {
                        ContestValidator::for_contest(contest).gates(record)
                    }
                    None => (true, false),
                })
                .collect(),
        }
    }

    /// Takes per-contest validators directly, for callers that built them
    /// from vote states rather than records.
    pub fn from_votes(votes: Vec<VoteValidator>) -> Self {
        BallotValidator {
            gates: votes
                .iter()
                .map(|vote| (vote.hard_gate(), vote.soft_gate()))
                .collect(),
        }
    }

    /// Reports whether ANY contest blocks Next.
    pub fn hard_gate(&self) -> bool {
        self.gates.iter().any(|(hard, _)| *hard)
    }

    /// Reports whether ANY contest asks for confirmation.
    pub fn soft_gate(&self) -> bool {
        self.gates.iter().any(|(_, soft)| *soft)
    }
}

/// Computes one message for the `invalid_errors` / `invalid_alerts` that
/// [`ContestValidator::messages`] produces, returned as an
/// [`InvalidPlaintextError`].
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

impl ContestValidator {
    /// Computes the policy-driven messages for one decoded contest — the
    /// errors and alerts the validation rules produce, in the order ballot
    /// decoding records them, as the `InvalidPlaintextError` entries it
    /// appends. Encoding and configuration errors are not produced here;
    /// decoding stamps those on the record itself.
    ///
    /// Fails when the contest's bounds cannot be read as counts: the
    /// min/over/under rules have nothing to compare against. The caller
    /// keeps the per-bound checks for that case (each runs with only the
    /// bounds it needs).
    pub fn messages(
        &self,
        decoded: &DecodedVoteContest,
    ) -> Result<CheckerResult, ValidationError> {
        Ok(self.for_decoded(decoded)?.recorded())
    }

    /// Classifies one decoded contest's ballot for the tally
    /// ([`classify`]), reading how it counts from the record: the decline
    /// bit, the explicit-invalid flag, whether decoding recorded any
    /// error, and which of this contest's candidates are selected.
    pub fn classify(&self, decoded: &DecodedVoteContest) -> BallotClass {
        let mut marker = false;
        let mut regular = false;
        for choice in &decoded.choices {
            if choice.selected > -1 {
                if self.shape.blank_markers.contains(&choice.id) {
                    marker = true;
                } else {
                    regular = true;
                }
                if marker && regular {
                    break;
                }
            }
        }
        classify(
            decoded.is_decline_to_vote,
            decoded.is_explicit_invalid,
            !decoded.invalid_errors.is_empty(),
            match (marker, regular) {
                (true, true) => SelectionClass::Mixed,
                (true, false) => SelectionClass::Marker,
                (false, true) => SelectionClass::Regular,
                (false, false) => SelectionClass::None,
            },
        )
    }

    /// Applies one edit to a contest's selections, enforcing the marker
    /// rules: a marker states something about the ballot as a whole, so it
    /// cannot stand beside the selections it contradicts.
    ///
    /// The explicit-blank marker means "I am leaving this contest blank",
    /// so selecting it clears everything else, and selecting anything else
    /// clears it. The explicit-invalid marker means "count this ballot as
    /// invalid", which the platform lets a voter combine with selections —
    /// they are recorded but not counted — EXCEPT under the invalid-vote
    /// policy that makes it exclusive, where it behaves like the blank
    /// marker in both directions.
    ///
    /// Returns the edited selections; nothing else about the ballot is
    /// touched, including the ballot-level blank and decline flags, which
    /// are the whole ballot's business rather than this contest's.
    pub fn apply(
        &self,
        selection: &DecodedVoteContest,
        edit: SelectionEdit,
    ) -> DecodedVoteContest {
        let mut edited = selection.clone();
        let exclusive_invalid = self.policies.invalid
            == InvalidVotePolicy::ALLOWED_WITH_EXCLUSIVE_EXPLICIT;

        match edit {
            SelectionEdit::Choice(choice) => {
                let selecting = choice.selected > -1;
                let is_blank_marker =
                    self.shape.blank_markers.contains(&choice.id);
                if let Some(existing) =
                    edited.choices.iter_mut().find(|c| c.id == choice.id)
                {
                    *existing = choice;
                } else {
                    return edited;
                }
                if selecting {
                    if is_blank_marker {
                        // The declared blank stands alone.
                        let marker = edited
                            .choices
                            .iter()
                            .find(|c| self.shape.blank_markers.contains(&c.id))
                            .map(|c| c.id.clone());
                        for other in edited.choices.iter_mut() {
                            if Some(&other.id) != marker.as_ref() {
                                other.selected = -1;
                            }
                        }
                        edited.is_explicit_invalid = false;
                    } else {
                        for other in edited.choices.iter_mut() {
                            if self.shape.blank_markers.contains(&other.id) {
                                other.selected = -1;
                            }
                        }
                        if exclusive_invalid {
                            edited.is_explicit_invalid = false;
                        }
                    }
                }
            }
            SelectionEdit::ExplicitInvalid(marked) => {
                edited.is_explicit_invalid = marked;
                if marked && exclusive_invalid {
                    for choice in edited.choices.iter_mut() {
                        choice.selected = -1;
                    }
                }
            }
        }
        edited
    }

    /// Reports whether this contest has taken all the selections it will
    /// accept, so the booth should stop offering more.
    ///
    /// Only the over-vote policy that disables inputs asks for this; every
    /// other policy lets the voter select past the maximum and says so
    /// afterwards, through a message or a gate. The count is the same one
    /// the rules use, markers included, so the controls close exactly when
    /// the rules consider the ballot full.
    ///
    /// A contest whose bounds cannot be read as counts has no maximum to
    /// reach, so nothing is capped and decoding reports the bounds instead.
    pub fn selection_capped(&self, decoded: &DecodedVoteContest) -> bool {
        let Ok((_, max)) = self.bounds else {
            return false;
        };
        self.policies.over == EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_DISABLE
            && selections(&self.vote_state(decoded)) >= max
    }

    /// Returns both gates for one decoded contest:
    /// `(blocks_next, needs_confirmation)`.
    ///
    /// Two things decide. The policy rules read the contest and the
    /// voter's selections. Errors that ballot decoding recorded are
    /// honoured directly, because the vote state cannot express them: an
    /// `Explicit` or `EncodingError` entry blocks Next, and an
    /// `EncodingError` entry also asks for confirmation unless the
    /// invalid-vote policy allows invalid ballots. When the contest's
    /// bounds cannot be read as counts the policy rules have nothing to
    /// compare against, and those recorded errors decide alone — which
    /// still blocks, since decoding records unusable bounds as encoding
    /// errors.
    pub fn gates(&self, decoded: &DecodedVoteContest) -> (bool, bool) {
        let recorded = &decoded.invalid_errors;
        let recorded_blocks = recorded.iter().any(|error| {
            matches!(
                error.error_type,
                InvalidPlaintextErrorType::Explicit
                    | InvalidPlaintextErrorType::EncodingError
            )
        });
        let encoding_error = recorded.iter().any(|error| {
            matches!(error.error_type, InvalidPlaintextErrorType::EncodingError)
        });
        let recorded_asks = encoding_error
            && self.policies.invalid != InvalidVotePolicy::ALLOWED
            && self.policies.invalid
                != InvalidVotePolicy::ALLOWED_WITH_EXCLUSIVE_EXPLICIT;

        match self.for_decoded(decoded) {
            Ok(validator) => (
                recorded_blocks || validator.hard_gate(),
                recorded_asks || validator.soft_gate(),
            ),
            Err(_) => (recorded_blocks, recorded_asks),
        }
    }

    /// Returns the decoded contest reduced to the messages the voter should
    /// see on one screen: the same record with `invalid_errors` and
    /// `invalid_alerts` filtered. `is_review` selects the review screen over
    /// the voting screen; `is_touched` is whether the voter has selected
    /// anything in this contest yet (an untouched contest shows nothing
    /// until they have).
    ///
    /// This needs no bounds and no selection count — only the recorded
    /// messages and two policies — so it answers even for a contest whose
    /// bounds are unusable, which is exactly when the voter most needs to
    /// see the encoding errors decoding recorded.
    pub fn filter_visible_messages(
        &self,
        decoded: &DecodedVoteContest,
        is_review: bool,
        is_touched: bool,
    ) -> DecodedVoteContest {
        let key_of =
            |e: &InvalidPlaintextError| e.message.clone().unwrap_or_default();
        let recorded = Emissions {
            errors: decoded.invalid_errors.iter().map(key_of).collect(),
            alerts: decoded.invalid_alerts.iter().map(key_of).collect(),
        };
        let kept =
            visible_messages(&self.policies, &recorded, is_review, is_touched);

        let mut visible = decoded.clone();
        visible
            .invalid_errors
            .retain(|error| kept.errors.contains(&key_of(error)));
        visible
            .invalid_alerts
            .retain(|alert| kept.alerts.contains(&key_of(alert)));
        visible
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ballot::{
        Candidate, CandidatePresentation, ContestPresentation,
    };
    use crate::plaintext::DecodedVoteChoice;

    fn config(min: u32, max: u32) -> Config {
        Config {
            min,
            max,
            policies: Policies::default(),
        }
    }

    fn vote(vs: VoteState, min: u32, max: u32) -> VoteValidator {
        ContestValidator::from_config(config(min, max))
            .for_vote_state(vs)
            .expect("representable bounds")
    }

    /// Returns what the voter sees on one screen: the display rules read
    /// this contest's policies and messages — never its selections.
    fn shown(
        v: &VoteValidator,
        is_review: bool,
        is_touched: bool,
    ) -> Vec<String> {
        let shown = visible_messages(
            &v.config().policies,
            v.messages(),
            is_review,
            is_touched,
        );
        shown.errors.into_iter().chain(shown.alerts).collect()
    }

    #[test]
    fn deliberate_blank_is_not_subject_to_the_min_vote_rule() {
        let blank = VoteState {
            blank_marker: true,
            ..VoteState::default()
        };
        let v = vote(blank, 2, 3);
        assert!(v.messages().errors.is_empty());
        assert!(!v.hard_gate() && !v.soft_gate());

        // A null vote (explicitly invalid) is not a blank: the min-vote
        // rule applies.
        let null = VoteState {
            explicit_invalid: true,
            ..VoteState::default()
        };
        let v = vote(null, 2, 3);
        assert!(v.messages().errors.iter().any(|m| m == SELECTED_MIN));
    }

    /// A decline says the voter is not choosing, so the selection rules
    /// have nothing to judge — at the count and at the Next button alike.
    /// Both halves need saying, because the gates read some of those rules
    /// from the vote state directly rather than through the messages.
    #[test]
    fn a_decline_is_not_held_to_the_selection_rules() {
        let mut c = config(2, 2);
        c.policies.blank = EBlankVotePolicy::NOT_ALLOWED;

        // The same empty ballot, declined and not.
        let declined = VoteState {
            decline: true,
            ..VoteState::default()
        };
        let v = ContestValidator::from_config(c.clone())
            .for_vote_state(declined)
            .expect("representable bounds");
        assert!(v.messages().errors.is_empty());
        assert!(v.messages().alerts.is_empty());
        assert!(!v.hard_gate(), "a decline is not blocked at Next");
        assert!(!v.soft_gate());

        // Without the decline the same ballot is stopped twice over, so
        // the assertions above are not passing for want of a rule to
        // break.
        let voting = ContestValidator::from_config(c)
            .for_vote_state(VoteState::default())
            .expect("representable bounds");
        assert!(voting.messages().errors.iter().any(|m| m == SELECTED_MIN));
        assert!(voting.messages().errors.iter().any(|m| m == BLANK_VOTE));
        assert!(voting.hard_gate());
    }

    #[test]
    fn the_empty_ballot_is_not_an_undervote() {
        let mut c = config(0, 2);
        c.policies.under = EUnderVotePolicy::WARN_AND_ALERT;
        let v = ContestValidator::from_config(c)
            .for_vote_state(VoteState::default())
            .expect("representable bounds");
        assert!(!v.messages().alerts.iter().any(|m| m == UNDER_VOTE));
        assert!(!v.soft_gate());
    }

    #[test]
    fn gate_and_checker_share_one_count() {
        // Two ranked selections with max 1: the checker emits the
        // over-vote error and the gate blocks from the same count.
        let mut c = config(0, 1);
        c.policies.over = EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_ALERT;
        let v = ContestValidator::from_config(c)
            .for_vote_state(VoteState {
                regulars: 2,
                ..VoteState::default()
            })
            .expect("representable bounds");
        assert!(v.messages().errors.iter().any(|m| m == SELECTED_MAX));
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
        let v = ContestValidator::from_config(c)
            .for_vote_state(VoteState {
                regulars: 1,
                ..VoteState::default()
            })
            .expect("representable bounds");
        assert!(v.messages().alerts.iter().any(|m| m == UNDER_VOTE));
        assert!(!shown(&v, false, true).iter().any(|m| m == UNDER_VOTE));
        assert!(shown(&v, true, true).iter().any(|m| m == UNDER_VOTE));
    }

    #[test]
    fn an_alert_that_already_renders_as_an_error_is_dropped() {
        // Over the maximum: the checker emits selectedMax as BOTH an error
        // and an alert; the voter sees it once.
        let mut c = config(0, 1);
        c.policies.over = EOverVotePolicy::ALLOWED_WITH_MSG;
        let v = ContestValidator::from_config(c)
            .for_vote_state(VoteState {
                regulars: 2,
                ..VoteState::default()
            })
            .expect("representable bounds");
        assert!(v.messages().errors.iter().any(|m| m == SELECTED_MAX));
        assert!(v.messages().alerts.iter().any(|m| m == SELECTED_MAX));
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
        let validator = ContestValidator::for_contest(&contest);
        let visible = validator.filter_visible_messages(&decoded, false, true);
        assert_eq!(visible.invalid_errors.len(), 1);
        // Untouched, the voting screen shows nothing at all.
        let untouched =
            validator.filter_visible_messages(&decoded, false, false);
        assert!(untouched.invalid_errors.is_empty());
    }

    /// A contest with one ordinary candidate and one explicit-blank marker.
    fn marker_contest() -> Contest {
        let candidate = |id: &str, blank: bool| Candidate {
            id: id.to_string(),
            presentation: Some(CandidatePresentation {
                is_explicit_blank: Some(blank),
                ..CandidatePresentation::default()
            }),
            ..Candidate::default()
        };
        Contest {
            candidates: vec![
                candidate("normal", false),
                candidate("blank", true),
            ],
            ..Contest::default()
        }
    }

    fn ballot(normal: bool, blank: bool) -> DecodedVoteContest {
        let choice = |id: &str, selected: bool| DecodedVoteChoice {
            id: id.to_string(),
            selected: if selected { 0 } else { -1 },
            write_in_text: None,
        };
        DecodedVoteContest {
            contest_id: "contest".to_string(),
            is_explicit_invalid: false,
            is_decline_to_vote: false,
            is_blank_ballot: false,
            invalid_errors: vec![],
            invalid_alerts: vec![],
            choices: vec![choice("normal", normal), choice("blank", blank)],
        }
    }

    fn class(decoded: &DecodedVoteContest) -> BallotClass {
        ContestValidator::for_contest(&marker_contest()).classify(decoded)
    }

    #[test]
    fn classifies_selections_that_speak_for_themselves() {
        assert_eq!(class(&ballot(true, false)), BallotClass::Valid);
        assert_eq!(class(&ballot(false, true)), BallotClass::ExplicitBlank);
        assert_eq!(class(&ballot(false, false)), BallotClass::ImplicitBlank);
        // The marker alongside a real selection contradicts itself.
        assert_eq!(class(&ballot(true, true)), BallotClass::ImplicitInvalid);
    }

    #[test]
    fn classifies_invalidity_by_how_it_arose() {
        let mut explicit = ballot(false, false);
        explicit.is_explicit_invalid = true;
        assert_eq!(class(&explicit), BallotClass::ExplicitInvalid);

        // ANY recorded error makes a ballot invalid, whatever it was about.
        let mut from_error = ballot(false, false);
        from_error.invalid_errors = vec![InvalidPlaintextError {
            error_type: InvalidPlaintextErrorType::Implicit,
            candidate_id: None,
            message: None,
            message_map: HashMap::new(),
        }];
        assert_eq!(class(&from_error), BallotClass::ImplicitInvalid);
    }

    #[test]
    fn a_decline_outranks_everything_but_must_be_empty() {
        let mut declined = ballot(false, false);
        declined.is_decline_to_vote = true;
        assert_eq!(class(&declined), BallotClass::Declined);

        // A declined ballot carrying content is not a decline at all.
        let mut with_selection = ballot(true, false);
        with_selection.is_decline_to_vote = true;
        assert_eq!(class(&with_selection), BallotClass::ImplicitInvalid);

        // Nor is one marked invalid — and the decline branch answers first,
        // so it is implicit, never explicit, invalid.
        let mut declined_invalid = ballot(false, false);
        declined_invalid.is_decline_to_vote = true;
        declined_invalid.is_explicit_invalid = true;
        assert_eq!(class(&declined_invalid), BallotClass::ImplicitInvalid);
    }

    /// A contest carrying both markers, so the two can be told apart.
    fn both_markers_contest(invalid: InvalidVotePolicy) -> Contest {
        let candidate = |id: &str, blank: bool, inv: bool| Candidate {
            id: id.to_string(),
            presentation: Some(CandidatePresentation {
                is_explicit_blank: Some(blank),
                is_explicit_invalid: Some(inv),
                ..CandidatePresentation::default()
            }),
            ..Candidate::default()
        };
        Contest {
            max_votes: 2,
            presentation: Some(ContestPresentation {
                invalid_vote_policy: Some(invalid),
                ..ContestPresentation::default()
            }),
            candidates: vec![
                candidate("normal", false, false),
                candidate("other", false, false),
                candidate("blank", true, false),
                candidate("null", false, true),
            ],
            ..Contest::default()
        }
    }

    fn selections(ids: &[&str]) -> DecodedVoteContest {
        DecodedVoteContest {
            contest_id: "contest".to_string(),
            is_explicit_invalid: false,
            is_decline_to_vote: false,
            is_blank_ballot: false,
            invalid_errors: vec![],
            invalid_alerts: vec![],
            choices: ["normal", "other", "blank", "null"]
                .iter()
                .map(|id| DecodedVoteChoice {
                    id: id.to_string(),
                    selected: if ids.contains(id) { 0 } else { -1 },
                    write_in_text: None,
                })
                .collect(),
        }
    }

    fn picked(selection: &DecodedVoteContest) -> Vec<&str> {
        selection
            .choices
            .iter()
            .filter(|c| c.selected > -1)
            .map(|c| c.id.as_str())
            .collect()
    }

    fn choose(id: &str, selected: bool) -> SelectionEdit {
        SelectionEdit::Choice(DecodedVoteChoice {
            id: id.to_string(),
            selected: if selected { 0 } else { -1 },
            write_in_text: None,
        })
    }

    #[test]
    fn the_declared_blank_stands_alone() {
        let v = ContestValidator::for_contest(&both_markers_contest(
            InvalidVotePolicy::ALLOWED,
        ));
        // Choosing the marker clears the selections it contradicts.
        let after =
            v.apply(&selections(&["normal", "other"]), choose("blank", true));
        assert_eq!(picked(&after), vec!["blank"]);
        // And choosing a candidate clears the marker.
        let after = v.apply(&selections(&["blank"]), choose("normal", true));
        assert_eq!(picked(&after), vec!["normal"]);
        // Deselecting clears nothing.
        let after =
            v.apply(&selections(&["normal", "other"]), choose("other", false));
        assert_eq!(picked(&after), vec!["normal"]);
    }

    #[test]
    fn the_invalid_marker_keeps_company_unless_the_policy_forbids_it() {
        // S5, kept per upstream #2949: under the ordinary policies a voter
        // may mark the ballot invalid AND leave selections standing.
        let permissive = ContestValidator::for_contest(&both_markers_contest(
            InvalidVotePolicy::ALLOWED,
        ));
        let after = permissive.apply(
            &selections(&["normal"]),
            SelectionEdit::ExplicitInvalid(true),
        );
        assert!(after.is_explicit_invalid);
        assert_eq!(picked(&after), vec!["normal"]);
        // And choosing a candidate afterwards does not unmark it.
        let after = permissive.apply(&after, choose("other", true));
        assert!(after.is_explicit_invalid);

        // Under the exclusive policy it behaves like the blank marker,
        // in both directions.
        let exclusive = ContestValidator::for_contest(&both_markers_contest(
            InvalidVotePolicy::ALLOWED_WITH_EXCLUSIVE_EXPLICIT,
        ));
        let after = exclusive.apply(
            &selections(&["normal", "other"]),
            SelectionEdit::ExplicitInvalid(true),
        );
        assert!(after.is_explicit_invalid);
        assert!(picked(&after).is_empty());

        let mut marked = selections(&[]);
        marked.is_explicit_invalid = true;
        let after = exclusive.apply(&marked, choose("normal", true));
        assert!(!after.is_explicit_invalid);
        assert_eq!(picked(&after), vec!["normal"]);

        // Marking an untouched contest invalid sets the flag and nothing
        // else, whichever policy is in force — there is nothing to clear.
        for validator in [&permissive, &exclusive] {
            let after = validator
                .apply(&selections(&[]), SelectionEdit::ExplicitInvalid(true));
            assert!(after.is_explicit_invalid);
            assert!(picked(&after).is_empty());
        }
    }

    #[test]
    fn an_edit_touches_only_this_contests_selections() {
        let v = ContestValidator::for_contest(&both_markers_contest(
            InvalidVotePolicy::ALLOWED,
        ));
        let mut before = selections(&["normal"]);
        before.is_blank_ballot = true;
        before.is_decline_to_vote = true;
        let after = v.apply(&before, choose("other", true));
        // The ballot-level flags are the whole ballot's business.
        assert!(after.is_blank_ballot);
        assert!(after.is_decline_to_vote);
        // An unknown candidate is not invented.
        let after = v.apply(&before, choose("nobody", true));
        assert_eq!(picked(&after), vec!["normal"]);
    }

    #[test]
    fn selections_are_capped_only_by_the_disabling_policy() {
        let contest = |over: EOverVotePolicy| {
            let mut c = marker_contest();
            c.max_votes = 1;
            c.presentation = Some(ContestPresentation {
                over_vote_policy: Some(over),
                ..ContestPresentation::default()
            });
            c
        };
        let disabling =
            contest(EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_DISABLE);
        let warning = contest(EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_ALERT);

        // At the maximum the disabling policy closes the controls; the
        // policy that only warns leaves them open.
        let full = ballot(true, false);
        assert!(
            ContestValidator::for_contest(&disabling).selection_capped(&full)
        );
        assert!(
            !ContestValidator::for_contest(&warning).selection_capped(&full)
        );

        // Below the maximum nothing is capped.
        let empty = ballot(false, false);
        assert!(
            !ContestValidator::for_contest(&disabling).selection_capped(&empty)
        );

        // The marker counts towards the maximum, as it does for the rules.
        let marker_only = ballot(false, true);
        assert!(ContestValidator::for_contest(&disabling)
            .selection_capped(&marker_only));
    }

    /// A worked analysis, and the property the display fix establishes: no
    /// ballot a voter can actually produce is excluded from the count
    /// while they were shown nothing about it.
    ///
    /// It is written to be copied. The rules are pure functions over small
    /// types, so a question about EVERY ballot is answered by enumerating,
    /// asking a validator for the effects, and asserting over the results
    /// — no reasoning about which cases matter, and no case left out.
    ///
    /// Note how the ballots are built: by applying the edits a voter makes
    /// ([`ContestValidator::apply`]), not by assembling vote states
    /// directly. That matters, because the marker rules make some states
    /// unreachable — a blank marker beside a real selection, for one — and
    /// those states DO classify as invalid without any message. They are
    /// not a defect, because no voter can produce them; enumerating states
    /// rather than edits would report them as if they were.
    #[test]
    fn no_ballot_is_discarded_without_telling_the_voter() {
        const REGULARS: [&str; 3] = ["a", "b", "c"];

        let contest = |invalid: InvalidVotePolicy,
                       blank: EBlankVotePolicy,
                       over: EOverVotePolicy,
                       under: EUnderVotePolicy,
                       min: i64,
                       max: i64| {
            let candidate =
                |id: &str, blank_marker: bool, invalid_marker: bool| {
                    Candidate {
                        id: id.to_string(),
                        presentation: Some(CandidatePresentation {
                            is_explicit_blank: Some(blank_marker),
                            is_explicit_invalid: Some(invalid_marker),
                            ..CandidatePresentation::default()
                        }),
                        ..Candidate::default()
                    }
                };
            let mut candidates: Vec<Candidate> = REGULARS
                .iter()
                .map(|id| candidate(id, false, false))
                .collect();
            candidates.push(candidate("blank", true, false));
            candidates.push(candidate("null", false, true));
            Contest {
                min_votes: min,
                max_votes: max,
                presentation: Some(ContestPresentation {
                    invalid_vote_policy: Some(invalid),
                    blank_vote_policy: Some(blank),
                    over_vote_policy: Some(over),
                    under_vote_policy: Some(under),
                    ..ContestPresentation::default()
                }),
                candidates,
                ..Contest::default()
            }
        };

        let untouched = |contest: &Contest| DecodedVoteContest {
            contest_id: contest.id.clone(),
            is_explicit_invalid: false,
            is_decline_to_vote: false,
            is_blank_ballot: false,
            invalid_errors: vec![],
            invalid_alerts: vec![],
            choices: contest
                .candidates
                .iter()
                .map(|c| DecodedVoteChoice {
                    id: c.id.clone(),
                    selected: -1,
                    write_in_text: None,
                })
                .collect(),
        };
        let select = |id: &str| {
            SelectionEdit::Choice(DecodedVoteChoice {
                id: id.to_string(),
                selected: 0,
                write_in_text: None,
            })
        };

        let mut examined = 0usize;
        for invalid in [
            InvalidVotePolicy::ALLOWED,
            InvalidVotePolicy::WARN,
            InvalidVotePolicy::WARN_INVALID_IMPLICIT_AND_EXPLICIT,
            InvalidVotePolicy::NOT_ALLOWED,
            InvalidVotePolicy::ALLOWED_WITH_EXCLUSIVE_EXPLICIT,
        ] {
            for blank in [
                EBlankVotePolicy::ALLOWED,
                EBlankVotePolicy::WARN,
                EBlankVotePolicy::WARN_ONLY_IN_REVIEW,
                EBlankVotePolicy::NOT_ALLOWED,
            ] {
                for over in [
                    EOverVotePolicy::ALLOWED,
                    EOverVotePolicy::ALLOWED_WITH_MSG,
                    EOverVotePolicy::ALLOWED_WITH_MSG_AND_ALERT,
                    EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_ALERT,
                    EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_DISABLE,
                ] {
                    for under in [
                        EUnderVotePolicy::ALLOWED,
                        EUnderVotePolicy::WARN,
                        EUnderVotePolicy::WARN_ONLY_IN_REVIEW,
                        EUnderVotePolicy::WARN_AND_ALERT,
                    ] {
                        for min in 0..=3i64 {
                            for max in 1..=3i64 {
                                if min > max {
                                    continue;
                                }
                                let contest = contest(
                                    invalid.clone(),
                                    blank.clone(),
                                    over.clone(),
                                    under.clone(),
                                    min,
                                    max,
                                );
                                let validator =
                                    ContestValidator::for_contest(&contest);

                                // Every ballot a voter can reach: some
                                // number of ordinary selections, then
                                // optionally the blank marker, then
                                // optionally marking the ballot invalid.
                                // Each step goes through the marker rules.
                                for regulars in 0..=REGULARS.len() {
                                    for marker in [false, true] {
                                        for mark_invalid in [false, true] {
                                            let mut ballot =
                                                untouched(&contest);
                                            for id in
                                                REGULARS.iter().take(regulars)
                                            {
                                                ballot = validator
                                                    .apply(&ballot, select(id));
                                            }
                                            if marker {
                                                ballot = validator.apply(
                                                    &ballot,
                                                    select("blank"),
                                                );
                                            }
                                            if mark_invalid {
                                                ballot = validator.apply(
                                                    &ballot,
                                                    SelectionEdit::ExplicitInvalid(true),
                                                );
                                            }
                                            // Decoding stamps the messages
                                            // onto the record, and the tally
                                            // reads the record — so classify
                                            // a decoded ballot, not a bare
                                            // selection.
                                            let recorded = validator
                                                .messages(&ballot)
                                                .expect("bounds are counts");
                                            ballot.update(recorded);
                                            examined += 1;

                                            let vote = validator
                                                .for_decoded(&ballot)
                                                .expect("bounds are counts");
                                            let messages = vote.messages();

                                            // Was the voter told anything,
                                            // at either point they can act?
                                            let shown = |is_review: bool| {
                                                let seen = visible_messages(
                                                    validator.policies(),
                                                    messages,
                                                    is_review,
                                                    true,
                                                );
                                                !seen.errors.is_empty()
                                                    || !seen.alerts.is_empty()
                                            };
                                            let told = shown(false)
                                                || shown(true)
                                                || vote.hard_gate()
                                                || vote.soft_gate();

                                            let discarded = matches!(
                                                validator.classify(&ballot),
                                                BallotClass::ImplicitInvalid
                                            );

                                            assert!(
                                                told || !discarded,
                                                "discarded in silence: {regulars} selected, blank marker {marker}, marked invalid {mark_invalid}, {:?}",
                                                validator.policies()
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // Guard against the enumeration silently collapsing to nothing.
        assert!(examined > 10_000, "examined only {examined} combinations");
    }

    /// The second worked analysis: a dependency map, saying which inputs
    /// can move which effects.
    ///
    /// Same three steps as the test above, asked of the rules rather than
    /// of a property. Evaluate every configuration in the domain, then
    /// walk each input in turn and record, per effect, whether varying
    /// that input alone ever changes it.
    ///
    /// The useful half of the answer is the absences. That a policy
    /// happens not to reach an effect on the cases someone thought to try
    /// says nothing; that it reaches it nowhere in an exhaustive domain
    /// is a fact about the rules. That is also why the map is asserted
    /// rather than printed — change what a rule reads and this test says
    /// so. Run it with `--nocapture` to see the table.
    ///
    /// The domain is every combination of the six policies, minimums
    /// `0..=2` against maximums `1..=3`, and eight ballots spanning both
    /// contest kinds — nothing selected through all three candidates
    /// selected, the blank marker alone, and three rankings (in order,
    /// two candidates sharing a rank, one rank skipped) — each with and
    /// without the ballot marked invalid. Ballots are built through
    /// [`ContestValidator::apply`], for the reason the test above gives.
    #[test]
    fn which_inputs_move_which_effects() {
        use crate::types::ceremonies::CountingAlgType;
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        /// Two effects are the same iff their digests are.
        fn digest(value: &impl std::fmt::Debug) -> u64 {
            let mut hasher = DefaultHasher::new();
            format!("{value:?}").hash(&mut hasher);
            hasher.finish()
        }

        const REGULARS: [&str; 3] = ["a", "b", "c"];
        const EFFECTS: [&str; 5] =
            ["messages", "dialog", "inline", "reachability", "tally"];

        #[derive(Clone, Copy)]
        enum Shape {
            /// Ordinary selections on a plurality contest, optionally
            /// with the blank marker.
            Plain { regulars: usize, marker: bool },
            /// A ranking on a preferential contest.
            Ranked(&'static [i64]),
        }

        let invalids = [
            InvalidVotePolicy::ALLOWED,
            InvalidVotePolicy::WARN,
            InvalidVotePolicy::WARN_INVALID_IMPLICIT_AND_EXPLICIT,
            InvalidVotePolicy::NOT_ALLOWED,
            InvalidVotePolicy::ALLOWED_WITH_EXCLUSIVE_EXPLICIT,
        ];
        let blanks = [
            EBlankVotePolicy::ALLOWED,
            EBlankVotePolicy::WARN,
            EBlankVotePolicy::WARN_ONLY_IN_REVIEW,
            EBlankVotePolicy::NOT_ALLOWED,
        ];
        let overs = [
            EOverVotePolicy::ALLOWED,
            EOverVotePolicy::ALLOWED_WITH_MSG,
            EOverVotePolicy::ALLOWED_WITH_MSG_AND_ALERT,
            EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_ALERT,
            EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_DISABLE,
        ];
        let unders = [
            EUnderVotePolicy::ALLOWED,
            EUnderVotePolicy::WARN,
            EUnderVotePolicy::WARN_ONLY_IN_REVIEW,
            EUnderVotePolicy::WARN_AND_ALERT,
        ];
        let dups = [
            EDuplicatedRankPolicy::ALLOWED_WARN_AND_DIALOG,
            EDuplicatedRankPolicy::NOT_ALLOWED_WARN_AND_DIALOG,
        ];
        let gaps = [
            EPreferenceGapsPolicy::ALLOWED_WARN_AND_DIALOG,
            EPreferenceGapsPolicy::NOT_ALLOWED_WARN_AND_DIALOG,
        ];
        let mins: [i64; 3] = [0, 1, 2];
        let maxes: [i64; 3] = [1, 2, 3];
        let shapes = [
            Shape::Plain {
                regulars: 0,
                marker: false,
            },
            Shape::Plain {
                regulars: 1,
                marker: false,
            },
            Shape::Plain {
                regulars: 2,
                marker: false,
            },
            Shape::Plain {
                regulars: 3,
                marker: false,
            },
            Shape::Plain {
                regulars: 0,
                marker: true,
            },
            Shape::Ranked(&[0, 1, 2]),
            Shape::Ranked(&[0, 0]),
            Shape::Ranked(&[0, 2]),
        ];

        // The domain as a grid: one axis per input, a cell per
        // combination, addressed in mixed radix so that varying one input
        // is arithmetic on one coordinate.
        let axes: [usize; 10] = [
            invalids.len(),
            blanks.len(),
            overs.len(),
            unders.len(),
            dups.len(),
            gaps.len(),
            mins.len(),
            maxes.len(),
            shapes.len(),
            2,
        ];
        let cells: usize = axes.iter().product();
        let address_of = |cell: &[usize; 10]| {
            cell.iter()
                .zip(axes)
                .fold(0usize, |address, (coord, len)| address * len + coord)
        };
        let cell_at = |address: usize| {
            let mut rest = address;
            let mut cell = [0usize; 10];
            for axis in (0..axes.len()).rev() {
                cell[axis] = rest % axes[axis];
                rest /= axes[axis];
            }
            cell
        };

        // The five effects of one cell, digested. `None` where a minimum
        // exceeds its maximum, which is not a contest anyone can build.
        let effects_of = |cell: &[usize; 10]| -> Option<[u64; 5]> {
            let (min, max) = (mins[cell[6]], maxes[cell[7]]);
            if min > max {
                return None;
            }
            let shape = shapes[cell[8]];

            let candidate =
                |id: &str, blank_marker: bool, invalid_marker: bool| {
                    Candidate {
                        id: id.to_string(),
                        presentation: Some(CandidatePresentation {
                            is_explicit_blank: Some(blank_marker),
                            is_explicit_invalid: Some(invalid_marker),
                            ..CandidatePresentation::default()
                        }),
                        ..Candidate::default()
                    }
                };
            let mut candidates: Vec<Candidate> = REGULARS
                .iter()
                .map(|id| candidate(id, false, false))
                .collect();
            candidates.push(candidate("blank", true, false));
            candidates.push(candidate("null", false, true));
            let contest = Contest {
                min_votes: min,
                max_votes: max,
                counting_algorithm: Some(match shape {
                    Shape::Ranked(_) => CountingAlgType::InstantRunoff,
                    Shape::Plain { .. } => CountingAlgType::PluralityAtLarge,
                }),
                presentation: Some(ContestPresentation {
                    invalid_vote_policy: Some(invalids[cell[0]].clone()),
                    blank_vote_policy: Some(blanks[cell[1]]),
                    over_vote_policy: Some(overs[cell[2]]),
                    under_vote_policy: Some(unders[cell[3]]),
                    duplicated_rank_policy: Some(dups[cell[4]]),
                    preference_gaps_policy: Some(gaps[cell[5]]),
                    ..ContestPresentation::default()
                }),
                candidates,
                ..Contest::default()
            };
            let validator = ContestValidator::for_contest(&contest);

            let mut ballot = DecodedVoteContest {
                contest_id: contest.id.clone(),
                is_explicit_invalid: false,
                is_decline_to_vote: false,
                is_blank_ballot: false,
                invalid_errors: vec![],
                invalid_alerts: vec![],
                choices: contest
                    .candidates
                    .iter()
                    .map(|c| DecodedVoteChoice {
                        id: c.id.clone(),
                        selected: -1,
                        write_in_text: None,
                    })
                    .collect(),
            };
            let select = |id: &str, rank: i64| {
                SelectionEdit::Choice(DecodedVoteChoice {
                    id: id.to_string(),
                    selected: rank,
                    write_in_text: None,
                })
            };
            match shape {
                Shape::Plain { regulars, marker } => {
                    for id in REGULARS.iter().take(regulars) {
                        ballot = validator.apply(&ballot, select(id, 0));
                    }
                    if marker {
                        ballot = validator.apply(&ballot, select("blank", 0));
                    }
                }
                Shape::Ranked(ranks) => {
                    for (id, rank) in REGULARS.iter().zip(ranks) {
                        ballot = validator.apply(&ballot, select(id, *rank));
                    }
                }
            }
            if cell[9] == 1 {
                ballot = validator
                    .apply(&ballot, SelectionEdit::ExplicitInvalid(true));
            }
            // Decoding stamps the messages onto the record, and the tally
            // reads the record.
            let recorded =
                validator.messages(&ballot).expect("bounds are counts");
            ballot.update(recorded);

            let vote =
                validator.for_decoded(&ballot).expect("bounds are counts");
            let seen = |is_review: bool| {
                visible_messages(
                    validator.policies(),
                    vote.messages(),
                    is_review,
                    true,
                )
            };
            Some([
                digest(vote.messages()),
                digest(&(vote.hard_gate(), vote.soft_gate())),
                digest(&(seen(false), seen(true))),
                digest(&validator.selection_capped(&ballot)),
                digest(&validator.classify(&ballot)),
            ])
        };

        let observed: Vec<Option<[u64; 5]>> = (0..cells)
            .map(|address| effects_of(&cell_at(address)))
            .collect();

        // Which pairs of values on an axis count as varying one input
        // alone. The ballot axis carries two inputs, so it needs both.
        type Varies = fn(usize, usize) -> bool;
        let freely: Varies = |_, _| true;
        // Selections: same contest kind, and the marker left as it was.
        let same_kind: Varies =
            |from, to| matches!((from, to), (0..=3, 0..=3) | (5..=7, 5..=7));
        // The marker: only against the empty ballot, because setting it
        // clears whatever stood beside it — so no other pair differs in
        // the marker alone.
        let marker_only: Varies =
            |from, to| matches!((from, to), (0, 4) | (4, 0));

        let inputs: [(&str, usize, Varies); 11] = [
            ("invalid_vote_policy", 0, freely),
            ("blank_vote_policy", 1, freely),
            ("over_vote_policy", 2, freely),
            ("under_vote_policy", 3, freely),
            ("duplicated_rank_policy", 4, freely),
            ("preference_gaps_policy", 5, freely),
            ("min_votes", 6, freely),
            ("max_votes", 7, freely),
            ("selections", 8, same_kind),
            ("blank marker", 8, marker_only),
            ("explicit invalid", 9, freely),
        ];

        let mut map: Vec<(&str, Vec<&str>)> = Vec::new();
        for (input, axis, varies) in inputs {
            let mut moves = [false; 5];
            for address in 0..cells {
                let Some(here) = observed[address] else {
                    continue;
                };
                let cell = cell_at(address);
                for value in 0..axes[axis] {
                    if value == cell[axis] || !varies(cell[axis], value) {
                        continue;
                    }
                    let mut elsewhere = cell;
                    elsewhere[axis] = value;
                    let Some(there) = observed[address_of(&elsewhere)] else {
                        continue;
                    };
                    for (moved, (a, b)) in
                        moves.iter_mut().zip(here.iter().zip(there.iter()))
                    {
                        *moved |= a != b;
                    }
                }
            }
            map.push((
                input,
                EFFECTS
                    .iter()
                    .zip(moves)
                    .filter(|(_, moved)| *moved)
                    .map(|(effect, _)| *effect)
                    .collect(),
            ));
        }

        // Printed as the markdown table docs/VALIDATION.md section 6
        // carries, so that section can be replaced with this output rather
        // than transcribed a cell at a time.
        println!("| input | {} |", EFFECTS.join(" | "));
        let mut rule = String::from("|---|");
        for _ in EFFECTS {
            rule.push_str(":-:|");
        }
        println!("{rule}");
        for (input, effects) in &map {
            let label = if input.contains('_') {
                format!("`{input}`")
            } else {
                input.to_string()
            };
            let cells: Vec<&str> = EFFECTS
                .iter()
                .map(|effect| if effects.contains(effect) { "•" } else { "" })
                .collect();
            println!("| {label} | {} |", cells.join(" | "));
        }

        // Read the absences. Under-vote and invalid-vote policy cannot
        // reach the tally: the first only ever raises an alert, and the
        // second only speaks when the ballot is already explicitly
        // invalid, which decides how it counts on its own. The rank
        // policies reach nothing but the dialog, because both of their
        // values emit the same error and differ only in which gate reacts.
        // Over-vote policy cannot reach the tally either, the over-vote
        // error being unconditional — while `max_votes`, which decides
        // whether that error is emitted at all, can.
        //
        // docs/VALIDATION.md section 6 reproduces this table; keep the
        // two in step.
        let expected: Vec<(&str, Vec<&str>)> = vec![
            (
                "invalid_vote_policy",
                vec!["messages", "dialog", "inline", "reachability"],
            ),
            (
                "blank_vote_policy",
                vec!["messages", "dialog", "inline", "tally"],
            ),
            (
                "over_vote_policy",
                vec!["messages", "dialog", "inline", "reachability"],
            ),
            ("under_vote_policy", vec!["messages", "dialog", "inline"]),
            ("duplicated_rank_policy", vec!["dialog"]),
            ("preference_gaps_policy", vec!["dialog"]),
            ("min_votes", vec!["messages", "dialog", "inline", "tally"]),
            (
                "max_votes",
                vec!["messages", "dialog", "inline", "reachability", "tally"],
            ),
            (
                "selections",
                vec!["messages", "dialog", "inline", "reachability", "tally"],
            ),
            (
                "blank marker",
                vec!["messages", "dialog", "inline", "reachability", "tally"],
            ),
            (
                "explicit invalid",
                vec!["messages", "dialog", "inline", "reachability", "tally"],
            ),
        ];
        assert_eq!(map, expected, "a rule started or stopped reading an input");
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
            .for_vote_state(VoteState::default())
            .expect("representable bounds");
        assert!(!clean.hard_gate());
        assert!(blocking.hard_gate());
        assert!(BallotValidator::from_votes(vec![clean, blocking]).hard_gate());
    }
}
