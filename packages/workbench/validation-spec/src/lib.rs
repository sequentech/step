// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The vote-validation specification as a typed, pure Rust function —
//! the Rust port of `packages/workbench/characterization/spec.mjs`
//! (VALIDATION_LOGIC_DISTILLATION.md §5.3 step 3), with the shape the
//! type system can now enforce (§1/§3 of that document):
//!
//! ```text
//! f(config, vote_state)
//!     = ( emissions,      the checker record (not a surface — the record
//!                         the surfaces consume; WASM-checkable)
//!         inline: observation point → keys,   surface {votingUntouched, voting, review}
//!         gate: (hard, soft),                 surface (the mechanism pair)
//!         dialog,          the gate's voter-facing projection
//!         reachability,    domain property, not an effect
//!         tally: BallotClass )                surface
//! ```
//!
//! Three of the six components are the effect surfaces
//! (VALIDATION_LOGIC_DISTILLATION.md §1, "The surfaces, enumerated"); the
//! others are a surface's projection, the upstream record, and a domain
//! property — the §1 correspondence note states each role.
//!
//! There is no observation-context input: the observation point indexes
//! the OUTPUT of the one surface it parameterizes (inline).
//!
//! This crate is a **characterization artifact**, not production code: it
//! transcribes the production rules bug-compatibly, with every surprising
//! behaviour carried as a named quirk (see [`quirks`]) tied to its
//! suspect/defect entry in `docs/UPSTREAM_FINDINGS.md`. Removing or
//! toggling a quirk is an adjudication decision, not a refactor — until
//! consultation blesses a change, this crate must match the live system.
//! Equivalence is checked two ways (see
//! `characterization/rust-conformance.mjs`): against the recorded
//! wasm-observed ground truth of every characterization cell, and against
//! `spec.mjs` over a large seeded-random input sample.
//!
//! Enum wire strings match the production JSON values exactly (the same
//! strings the recorded tables carry); the *type names* are
//! workbench-local. The equivalence harness maps by wire string, so a new
//! upstream variant fails loudly at parse rather than silently.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Message keys (single source; identical to spec.mjs MSG)
// ---------------------------------------------------------------------------

pub const SELECTED_MAX: &str = "errors.implicit.selectedMax";
pub const SELECTED_MIN: &str = "errors.implicit.selectedMin";
pub const BLANK_VOTE: &str = "errors.implicit.blankVote";
pub const UNDER_VOTE: &str = "errors.implicit.underVote";
pub const OVER_VOTE_DISABLED: &str = "errors.implicit.overVoteDisabled";
pub const DUPLICATED_POSITION: &str = "errors.implicit.duplicatedPosition";
pub const PREFERENCE_ORDER_WITH_GAPS: &str = "errors.implicit.preferenceOrderWithGaps";
pub const EXPLICIT_NOT_ALLOWED: &str = "errors.explicit.notAllowed";
pub const EXPLICIT_ALERT: &str = "errors.explicit.alert";

/// Messages whose `error_type` is Explicit or EncodingError — the hard
/// gate's fast path fires on any of them. The encoding/configuration keys
/// are listed for gate faithfulness only; no grid exercises them
/// (characterization/README.md, "Scope boundaries").
const EXPLICIT_OR_ENCODING: [&str; 5] = [
    EXPLICIT_NOT_ALLOWED,
    "errors.configuration.multipleExplicitInvalidCandidates",
    "errors.configuration.multipleExplicitBlankCandidates",
    "errors.encoding.invalidMinVotes",
    "errors.encoding.invalidMaxVotes",
];

// ---------------------------------------------------------------------------
// Policy enums — wire strings exactly as production serializes them
// (production types: sequent-core's InvalidVotePolicy, EBlankVotePolicy,
// EOverVotePolicy, EUnderVotePolicy, EDuplicatedRankPolicy /
// EPreferenceGapsPolicy)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum InvalidVotePolicy {
    #[default]
    #[serde(rename = "allowed")]
    Allowed,
    #[serde(rename = "warn")]
    Warn,
    #[serde(rename = "warn-invalid-implicit-and-explicit")]
    WarnInvalidImplicitAndExplicit,
    #[serde(rename = "not-allowed")]
    NotAllowed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BlankVotePolicy {
    #[default]
    #[serde(rename = "allowed")]
    Allowed,
    #[serde(rename = "warn")]
    Warn,
    #[serde(rename = "warn-only-in-review")]
    WarnOnlyInReview,
    #[serde(rename = "not-allowed")]
    NotAllowed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OverVotePolicy {
    #[serde(rename = "allowed")]
    Allowed,
    #[serde(rename = "allowed-with-msg")]
    AllowedWithMsg,
    #[default]
    #[serde(rename = "allowed-with-msg-and-alert")]
    AllowedWithMsgAndAlert,
    #[serde(rename = "not-allowed-with-msg-and-alert")]
    NotAllowedWithMsgAndAlert,
    #[serde(rename = "not-allowed-with-msg-and-disable")]
    NotAllowedWithMsgAndDisable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UnderVotePolicy {
    #[default]
    #[serde(rename = "allowed")]
    Allowed,
    #[serde(rename = "warn")]
    Warn,
    #[serde(rename = "warn-only-in-review")]
    WarnOnlyInReview,
    #[serde(rename = "warn-and-alert")]
    WarnAndAlert,
}

/// Shared shape of `EDuplicatedRankPolicy` and `EPreferenceGapsPolicy`:
/// both enums have ONLY `*_WARN_AND_DIALOG` variants — no silent value —
/// which is exactly why the preferential rules are immune to silent
/// discounting (VALIDATION_LOGIC_DISTILLATION.md §4.5, condition 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RankPolicy {
    #[default]
    #[serde(rename = "allowed-warn-and-dialog")]
    AllowedWarnAndDialog,
    #[serde(rename = "not-allowed-warn-and-dialog")]
    NotAllowedWarnAndDialog,
}

/// A contest's six validation policies, defaulting to the platform
/// baselines (FIXTURE_VARIANCE.md §10.A) — callers set only what they vary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Policies {
    pub invalid: InvalidVotePolicy,
    pub blank: BlankVotePolicy,
    pub over: OverVotePolicy,
    pub under: UnderVotePolicy,
    pub dup: RankPolicy,
    pub gap: RankPolicy,
}

/// The contest knobs the mapping reads. Serde defaults mirror spec.mjs's
/// `?? 0` / `?? 1` fallbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub min: u32,
    pub max: u32,
    pub policies: Policies,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            min: 0,
            max: 1,
            policies: Policies::default(),
        }
    }
}

/// What the voter did, independent of wire encoding (spec.mjs VoteState).
/// For preferential contests `regulars` is the number of candidates at
/// rank 1 (`selected == 0`) — the count the gates use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct VoteState {
    pub regulars: u32,
    /// The explicit-blank marker candidate is selected.
    pub blank_marker: bool,
    /// The ballot is explicitly invalid — the `is_explicit_invalid` flag,
    /// whether set via the null-vote marker or directly (the two routes
    /// converge; recorded in invalid-rule.md, "Route convergence").
    pub explicit_invalid: bool,
    /// The ballot-level decline-to-vote bit (multi-contest encodings only).
    pub decline: bool,
    /// Two candidates share a rank (preferential only).
    pub duplicate_ranks: bool,
    /// The ranking skips a rank (preferential only).
    pub rank_gaps: bool,
}

// ---------------------------------------------------------------------------
// Derived vote-state facts
// ---------------------------------------------------------------------------

/// The marker-inclusive selection count (`num_selected_with_markers` in
/// raw_ballot.rs, `selections_with_markers` in voting_screen.rs): a
/// selected blank marker and a set explicit-invalid flag each count as one
/// selection.
///
/// QUIRK(S3_MARKER_COUNTS_AS_SELECTION): the min/over/under/blank rules all
/// compare against this count, so a deliberate blank marker is inside the
/// count rules' domain and counts as 1 — UPSTREAM_FINDINGS.md S3, and the
/// domain half of S2.
pub fn selections_with_markers(vs: &VoteState) -> u32 {
    vs.regulars + u32::from(vs.blank_marker) + u32::from(vs.explicit_invalid)
}

/// The ballot-shape class the tally classifier reads. `Marker`/`Mixed`
/// concern the explicit-BLANK marker only (`classify_ballot` collects
/// explicit-blank candidate ids; the invalid marker is represented by the
/// flag, which short-circuits earlier in the precedence).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionClass {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "regular")]
    Regular,
    #[serde(rename = "marker")]
    Marker,
    #[serde(rename = "mixed")]
    Mixed,
}

pub fn selection_class(vs: &VoteState) -> SelectionClass {
    match (vs.blank_marker, vs.regulars > 0) {
        (true, true) => SelectionClass::Mixed,
        (true, false) => SelectionClass::Marker,
        (false, true) => SelectionClass::Regular,
        (false, false) => SelectionClass::None,
    }
}

// ---------------------------------------------------------------------------
// The checker stage
// ---------------------------------------------------------------------------

/// The checker record: which `invalid_errors` / `invalid_alerts`
/// checker.rs produces. Internal — never voter-perceived — but exposed
/// because it is independently checkable against the real WASM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Emissions {
    pub errors: Vec<String>,
    pub alerts: Vec<String>,
}

/// Transcription of the checker calls in `raw_ballot.rs::decode`, in call
/// order: invalid → over → min → under → blank → preference-gaps →
/// duplicated-rank. The config-sanity and encoding-error emissions are not
/// modelled (no grid exercises them — characterization/README.md, "Scope
/// boundaries").
pub fn emissions(config: &Config, vs: &VoteState) -> Emissions {
    let p = &config.policies;
    let n = selections_with_markers(vs);
    let mut errors: Vec<String> = Vec::new();
    let mut alerts: Vec<String> = Vec::new();

    // invalid rule — fires only on an explicitly-invalid ballot
    if vs.explicit_invalid {
        if p.invalid == InvalidVotePolicy::NotAllowed {
            errors.push(EXPLICIT_NOT_ALLOWED.into());
        }
        if p.invalid == InvalidVotePolicy::WarnInvalidImplicitAndExplicit {
            alerts.push(EXPLICIT_ALERT.into());
        }
    }
    // over-vote rule — the error is unconditional; the policy governs only
    // the alert (and, at exactly max under DISABLE, the "maximum reached"
    // hint). The unconditional error dates from 7b0a1c71e8 (before it,
    // emission was guarded by invalid != allowed — see
    // INVALID_VOTE_POLICY_INTENT.md §5).
    if n > config.max {
        errors.push(SELECTED_MAX.into());
        if p.over != OverVotePolicy::Allowed {
            alerts.push(SELECTED_MAX.into());
        }
    } else if n == config.max && p.over == OverVotePolicy::NotAllowedWithMsgAndDisable {
        alerts.push(OVER_VOTE_DISABLED.into());
    }
    // min-vote rule — a fixed rule with no policy of its own: always an
    // error. Its lack of a policy is what pins it at the silent-prone
    // configuration (UPSTREAM_FINDINGS.md S1, min-vote family).
    if n < config.min {
        errors.push(SELECTED_MIN.into());
    }
    // under-vote rule — alerts only, never errors.
    // QUIRK(S4_UNDERVOTE_ZONE_INCLUDES_ZERO): the zone `min ≤ n < max`
    // includes n = 0 when min is 0, overlapping the blank rule.
    if n >= config.min && n < config.max && p.under != UnderVotePolicy::Allowed {
        alerts.push(UNDER_VOTE.into());
    }
    // blank rule — skipped entirely for an explicitly-invalid ballot
    if n == 0 && !vs.explicit_invalid && p.blank != BlankVotePolicy::Allowed {
        let key = BLANK_VOTE.into();
        if p.blank == BlankVotePolicy::NotAllowed {
            errors.push(key);
        } else {
            alerts.push(key);
        }
    }
    // preferential rules — both policy variants emit identically; the
    // policy decides only which gate reacts
    if vs.rank_gaps {
        errors.push(PREFERENCE_ORDER_WITH_GAPS.into());
    }
    if vs.duplicate_ranks {
        errors.push(DUPLICATED_POSITION.into());
    }
    Emissions { errors, alerts }
}

// ---------------------------------------------------------------------------
// Gates
// ---------------------------------------------------------------------------

/// Both review-transition gates. The surface's value is the PAIR — the two
/// production functions are independent and can both fire (recorded:
/// invalid-rule.md, not-allowed rows). The dialog the voter meets is a
/// projection (see [`Effects::dialog`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gate {
    pub hard: bool,
    pub soft: bool,
}

/// `voting_screen.rs::check_voting_not_allowed_next_util`. True ⇒ a
/// blocking dialog: the voter cannot reach review until the ballot is
/// fixed.
pub fn hard_gate(config: &Config, vs: &VoteState, em: &Emissions) -> bool {
    let p = &config.policies;
    let n = selections_with_markers(vs);
    em.errors
        .iter()
        .any(|m| EXPLICIT_OR_ENCODING.contains(&m.as_str()))
        || (!em.errors.is_empty() && p.invalid == InvalidVotePolicy::NotAllowed)
        || (n == 0 && p.blank == BlankVotePolicy::NotAllowed)
        || (n > config.max && p.over == OverVotePolicy::NotAllowedWithMsgAndAlert)
        || (p.dup == RankPolicy::NotAllowedWarnAndDialog
            && em.errors.iter().any(|m| m == DUPLICATED_POSITION))
        || (p.gap == RankPolicy::NotAllowedWarnAndDialog
            && em.errors.iter().any(|m| m == PREFERENCE_ORDER_WITH_GAPS))
}

/// `voting_screen.rs::check_voting_error_dialog_util`. True ⇒ a
/// dismissible dialog: the voter is warned but may continue.
pub fn soft_gate(config: &Config, vs: &VoteState, em: &Emissions) -> bool {
    let p = &config.policies;
    let n = selections_with_markers(vs);
    (!em.errors.is_empty() && p.invalid != InvalidVotePolicy::Allowed)
        || (p.invalid == InvalidVotePolicy::WarnInvalidImplicitAndExplicit
            && vs.explicit_invalid)
        || (p.blank == BlankVotePolicy::Warn && n == 0)
        || (n > config.max && p.over == OverVotePolicy::AllowedWithMsgAndAlert)
        // QUIRK(S4_GATE_REDERIVES_UNDERVOTE_AT_ZERO): the gate re-derives
        // the under-vote zone with an `n > 0` guard the checker lacks, so
        // it skips the empty ballot the checker just alerted on —
        // UPSTREAM_FINDINGS.md S4 (two independent expressions of one
        // boundary, drifted at n = 0).
        || (n > 0
            && n >= config.min
            && n < config.max
            && p.under == UnderVotePolicy::WarnAndAlert)
        || (p.dup == RankPolicy::AllowedWarnAndDialog
            && em.errors.iter().any(|m| m == DUPLICATED_POSITION))
        || (p.gap == RankPolicy::AllowedWarnAndDialog
            && em.errors.iter().any(|m| m == PREFERENCE_ORDER_WITH_GAPS))
}

// ---------------------------------------------------------------------------
// Tally classifier
// ---------------------------------------------------------------------------

/// The six ballot classes of `velvet-core::classify_ballot`, wire strings
/// as the recorded tables carry them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BallotClass {
    Valid,
    ExplicitInvalid,
    ImplicitInvalid,
    ExplicitBlank,
    ImplicitBlank,
    Declined,
}

/// `classify_ballot` — first matching guard wins (the precedence note in
/// classifier-table.md). `invalid = flag || errors`. Exposed with the raw
/// inputs (not derived from emissions) so the classifier's own 32-cell
/// decision table — which includes synthetic error states — can probe it
/// directly.
///
/// QUIRK(S2_ERROR_OUTRANKS_BLANK_MARKER): a marker-only ballot carrying
/// any error classifies ImplicitInvalid, never ExplicitBlank — the
/// classification half of UPSTREAM_FINDINGS.md S2 (a deliberate blank
/// failing min_votes is booked as implicit invalidity).
pub fn classify(
    decline: bool,
    explicit_invalid: bool,
    has_errors: bool,
    selection: SelectionClass,
) -> BallotClass {
    let invalid = explicit_invalid || has_errors;
    let nothing_selected = selection == SelectionClass::None;
    let blank = !invalid && nothing_selected;
    if decline {
        return if blank {
            BallotClass::Declined
        } else {
            BallotClass::ImplicitInvalid
        };
    }
    if invalid {
        return if explicit_invalid {
            BallotClass::ExplicitInvalid
        } else {
            BallotClass::ImplicitInvalid
        };
    }
    match selection {
        SelectionClass::Mixed => BallotClass::ImplicitInvalid,
        SelectionClass::Marker => BallotClass::ExplicitBlank,
        SelectionClass::None => BallotClass::ImplicitBlank,
        SelectionClass::Regular => BallotClass::Valid,
    }
}

// ---------------------------------------------------------------------------
// Inline surface (the booth message filter, per observation point)
// ---------------------------------------------------------------------------

/// The inline surface's value: one set of rendered message keys per
/// observation point. The point indexes this OUTPUT — it is not an input
/// to the mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineViews {
    /// The untouched-clear: an untouched contest renders nothing.
    pub voting_untouched: Vec<String>,
    pub voting: Vec<String>,
    pub review: Vec<String>,
}

/// One observation point of `InvalidErrorsList.tsx::filterErrorList`,
/// transcribed in production order: alert visibility → dedup → the master
/// keep-list on errors; errors render before alerts.
fn rendered_keys(
    p: &Policies,
    errors: &[String],
    alerts: &[String],
    is_review: bool,
) -> Vec<String> {
    // Alert visibility — the only point-dependent rules.
    let mut kept_alerts: Vec<&String> = alerts
        .iter()
        .filter(|m| {
            !((m.as_str() == UNDER_VOTE
                && !is_review
                && p.under == UnderVotePolicy::WarnOnlyInReview)
                || (m.as_str() == BLANK_VOTE
                    && !is_review
                    && p.blank == BlankVotePolicy::WarnOnlyInReview)
                || (m.as_str() == OVER_VOTE_DISABLED && is_review))
        })
        .collect();
    // Dedup — production's `containsError` searches the visibility-filtered
    // alerts plus the RAW (pre-keep-list) errors.
    // QUIRK(D3_SELECTED_MAX_ALERT_SELF_DEDUP): the selectedMax predicate
    // matches the very alert under examination, so a selectedMax alert is
    // always dropped — UPSTREAM_FINDINGS.md Defect 3, transcribed as-is.
    let present = |m: &str, alerts_now: &[&String]| {
        alerts_now.iter().any(|a| a.as_str() == m) || errors.iter().any(|e| e == m)
    };
    let snapshot = kept_alerts.clone();
    kept_alerts.retain(|m| {
        !((m.as_str() == UNDER_VOTE && present(BLANK_VOTE, &snapshot))
            || (m.as_str() == SELECTED_MAX && present(SELECTED_MAX, &snapshot)))
    });
    // The master keep-list: under `invalid = allowed` every error is hidden
    // except the two carve-outs.
    // QUIRK(S1_ALLOWED_MUTES_IMPLICIT_ERRORS): this suppression (added by
    // 7b0a1c71e8) composes with the gates' `invalid != allowed` conditions
    // into the silent-discount cells — UPSTREAM_FINDINGS.md S1.
    let kept_errors = errors.iter().filter(|m| {
        if p.invalid != InvalidVotePolicy::Allowed {
            return true;
        }
        (m.as_str() == SELECTED_MAX && p.over != OverVotePolicy::Allowed)
            || (m.as_str() == BLANK_VOTE && p.blank == BlankVotePolicy::NotAllowed)
    });
    kept_errors
        .cloned()
        .chain(kept_alerts.into_iter().cloned())
        .collect()
}

/// All three observation points, from a checker record (either predicted
/// via [`emissions`] or observed — the recorded tables' derived view).
pub fn inline_views(p: &Policies, errors: &[String], alerts: &[String]) -> InlineViews {
    InlineViews {
        voting_untouched: Vec::new(),
        voting: rendered_keys(p, errors, alerts, false),
        review: rendered_keys(p, errors, alerts, true),
    }
}

// ---------------------------------------------------------------------------
// Reachability
// ---------------------------------------------------------------------------

/// Can the booth UI form this state at all? Not an effect — a domain
/// property; `f` stays total over states that exist anyway (hand-built or
/// decoded records still run through the checkers as defense-in-depth).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reachability {
    #[serde(rename = "yes")]
    Yes,
    /// Under the DISABLE over-vote policy the (max+1)th control is
    /// disabled, so a past-max state cannot be clicked together.
    #[serde(rename = "inputs_disabled")]
    InputsDisabled,
    /// Selecting the blank marker clears co-selected regulars, so
    /// {blank marker + regular} collapses.
    /// QUIRK(S5_INVALID_MARKER_PRESERVES_CHOICES): the INVALID marker
    /// deliberately does not clear — {regular + null marker} forms, and
    /// the latent choice is encrypted into the cast ballot —
    /// UPSTREAM_FINDINGS.md S5 (preservation declared intentional by
    /// upstream #2949; the privacy facet is the open residue).
    #[serde(rename = "marker_cleared")]
    MarkerCleared,
}

pub fn reachability(config: &Config, vs: &VoteState) -> Reachability {
    if config.policies.over == OverVotePolicy::NotAllowedWithMsgAndDisable
        && selections_with_markers(vs) > config.max
    {
        return Reachability::InputsDisabled;
    }
    if vs.blank_marker && vs.regulars > 0 {
        return Reachability::MarkerCleared;
    }
    Reachability::Yes
}

// ---------------------------------------------------------------------------
// The complete mapping
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dialog {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "dismissible")]
    Dismissible,
    #[serde(rename = "blocking")]
    Blocking,
}

/// The mapping's full output (field names match spec.mjs `f`'s output, so
/// the conformance harness compares structures directly). Three fields are
/// the effect surfaces — `inline`, `gate`, `tally` — holding one value
/// each; the rest are their companions: `emissions` is the checker record
/// the surfaces consume (never voter-perceived; WASM-checkable), `dialog`
/// is the gate surface's voter-facing projection, and `reachability` is a
/// domain property, not an effect
/// (VALIDATION_LOGIC_DISTILLATION.md §1, the correspondence note).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Effects {
    pub emissions: Emissions,
    pub inline: InlineViews,
    pub gate: Gate,
    /// What the voter meets on clicking Next: hard wins when both gates
    /// fire (`VotingScreen.tsx`, `encryptAndReview`).
    pub dialog: Dialog,
    pub reachability: Reachability,
    pub tally: BallotClass,
}

/// The mapping. (config × vote_state) determines everything; the
/// observation point exists only inside the output (see [`InlineViews`]).
pub fn f(config: &Config, vs: &VoteState) -> Effects {
    let em = emissions(config, vs);
    let hard = hard_gate(config, vs, &em);
    let soft = soft_gate(config, vs, &em);
    let inline = inline_views(&config.policies, &em.errors, &em.alerts);
    let tally = classify(
        vs.decline,
        vs.explicit_invalid,
        !em.errors.is_empty(),
        selection_class(vs),
    );
    Effects {
        inline,
        gate: Gate { hard, soft },
        dialog: if hard {
            Dialog::Blocking
        } else if soft {
            Dialog::Dismissible
        } else {
            Dialog::None
        },
        reachability: reachability(config, vs),
        tally,
        emissions: em,
    }
}

// ---------------------------------------------------------------------------
// Quirk registry — the enumerated accidental complexity
// ---------------------------------------------------------------------------

/// A named surprising behaviour this crate reproduces bug-compatibly. Each
/// entry ties a code site (the `QUIRK(...)` comment bearing the same id)
/// to its suspect/defect record. Toggling or removing one is an
/// adjudication decision (the three-state model —
/// characterization/README.md); until then the spec must match the live
/// system, and the conformance harness enforces that it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct QuirkInfo {
    pub id: &'static str,
    /// The UPSTREAM_FINDINGS.md entry (suspect or defect) that records it.
    pub finding: &'static str,
    /// Where in this crate the behaviour lives.
    pub site: &'static str,
    pub description: &'static str,
}

pub fn quirks() -> &'static [QuirkInfo] {
    &[
        QuirkInfo {
            id: "S1_ALLOWED_MUTES_IMPLICIT_ERRORS",
            finding: "S1 (and INVALID_VOTE_POLICY_INTENT.md §5)",
            site: "rendered_keys (master keep-list)",
            description: "under invalid=allowed every error is hidden except \
                          selectedMax (iff over≠allowed) and blankVote (iff \
                          blank=not-allowed); composes with the gates into \
                          the silent-discount cells",
        },
        QuirkInfo {
            id: "S2_ERROR_OUTRANKS_BLANK_MARKER",
            finding: "S2",
            site: "classify (invalid guard precedes marker guard)",
            description: "a marker-only ballot with any error classifies \
                          ImplicitInvalid, never ExplicitBlank — a deliberate \
                          blank failing min_votes is booked as implicit \
                          invalidity",
        },
        QuirkInfo {
            id: "S3_MARKER_COUNTS_AS_SELECTION",
            finding: "S3",
            site: "selections_with_markers",
            description: "the blank marker and the invalid flag each count as \
                          one selection in the min/over/under/blank rules",
        },
        QuirkInfo {
            id: "S4_GATE_REDERIVES_UNDERVOTE_AT_ZERO",
            finding: "S4",
            site: "soft_gate (under-vote clause)",
            description: "the gate re-derives the under-vote zone with an \
                          n > 0 guard the checker lacks — the checker alerts \
                          on the empty ballot, the gate skips it",
        },
        QuirkInfo {
            id: "S4_UNDERVOTE_ZONE_INCLUDES_ZERO",
            finding: "S4 (checker half)",
            site: "emissions (under-vote clause)",
            description: "the checker's zone min ≤ n < max includes n = 0 \
                          when min is 0, overlapping the blank rule",
        },
        QuirkInfo {
            id: "S5_INVALID_MARKER_PRESERVES_CHOICES",
            finding: "S5",
            site: "reachability (no marker_cleared for the invalid marker)",
            description: "the blank marker clears co-selected regulars; the \
                          invalid (null-vote) marker does not — the latent \
                          choice is encrypted into the cast ballot",
        },
        QuirkInfo {
            id: "D3_SELECTED_MAX_ALERT_SELF_DEDUP",
            finding: "Defect 3",
            site: "rendered_keys (dedup block)",
            description: "the dedup predicate matches the very alert under \
                          examination, so a selectedMax alert is always \
                          dropped — only the error copy can render",
        },
    ]
}

// ---------------------------------------------------------------------------
// Tests — a handful of exact cells from the recorded tables (the full
// equivalence run is characterization/rust-conformance.mjs)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(min: u32, max: u32, p: Policies) -> Config {
        Config {
            min,
            max,
            policies: p,
        }
    }

    /// minvote-rule.md, `min=2 × allowed × marker_only` — the S2 cell:
    /// selectedMin error, no gates, silent at both casting points,
    /// ImplicitInvalid.
    #[test]
    fn s2_marker_only_below_min_is_silently_discarded() {
        let config = cfg(2, 3, Policies::default());
        let vs = VoteState {
            blank_marker: true,
            ..Default::default()
        };
        let out = f(&config, &vs);
        assert_eq!(out.emissions.errors, vec![SELECTED_MIN.to_string()]);
        assert!(out.emissions.alerts.is_empty());
        assert_eq!(
            out.gate,
            Gate {
                hard: false,
                soft: false
            }
        );
        assert!(out.inline.voting.is_empty());
        assert!(out.inline.review.is_empty());
        assert_eq!(out.tally, BallotClass::ImplicitInvalid);
        assert_eq!(out.reachability, Reachability::Yes);
    }

    /// overvote-rule.md, `not-allowed-with-msg-and-disable × allowed ×
    /// at_max` — the voting-only hint: visible at voting, hidden at review.
    #[test]
    fn disable_at_max_hint_is_voting_only() {
        let p = Policies {
            over: OverVotePolicy::NotAllowedWithMsgAndDisable,
            ..Default::default()
        };
        let out = f(
            &cfg(0, 1, p),
            &VoteState {
                regulars: 1,
                ..Default::default()
            },
        );
        assert_eq!(out.emissions.alerts, vec![OVER_VOTE_DISABLED.to_string()]);
        assert_eq!(out.inline.voting, vec![OVER_VOTE_DISABLED.to_string()]);
        assert!(out.inline.review.is_empty());
        assert_eq!(out.tally, BallotClass::Valid);
    }

    /// undervote-rule.md, `warn-only-in-review × * × empty` — the timing
    /// rule (hidden at voting, shown at review) and the S4 gate skip.
    #[test]
    fn warn_only_in_review_timing_and_s4_gate_skip() {
        let p = Policies {
            under: UnderVotePolicy::WarnOnlyInReview,
            ..Default::default()
        };
        let out = f(&cfg(0, 2, p), &VoteState::default());
        assert_eq!(out.emissions.alerts, vec![UNDER_VOTE.to_string()]);
        assert!(out.inline.voting.is_empty());
        assert_eq!(out.inline.review, vec![UNDER_VOTE.to_string()]);
        assert_eq!(
            out.gate,
            Gate {
                hard: false,
                soft: false
            }
        );
    }

    /// overvote-rule.md, `allowed-with-msg × allowed × over_max` — the
    /// selectedMax alert is self-dedup'd (Defect 3): only the error copy
    /// renders, kept by the keep-list carve-out.
    #[test]
    fn selected_max_alert_always_dedups() {
        let p = Policies {
            over: OverVotePolicy::AllowedWithMsg,
            ..Default::default()
        };
        let out = f(
            &cfg(0, 1, p),
            &VoteState {
                regulars: 2,
                ..Default::default()
            },
        );
        assert_eq!(out.emissions.alerts, vec![SELECTED_MAX.to_string()]);
        assert_eq!(out.inline.voting, vec![SELECTED_MAX.to_string()]);
        assert_eq!(out.inline.review, vec![SELECTED_MAX.to_string()]);
    }

    /// classifier-table.md: decline + explicit-invalid flag + empty →
    /// ImplicitInvalid (the decline branch tests blankness, not the flag).
    #[test]
    fn decline_with_flag_is_implicit_invalid() {
        assert_eq!(
            classify(true, true, false, SelectionClass::None),
            BallotClass::ImplicitInvalid
        );
        assert_eq!(
            classify(true, false, false, SelectionClass::None),
            BallotClass::Declined
        );
    }

    /// invalid-rule.md, `not-allowed × marker` — both gates fire at once:
    /// the gate surface is a pair, not a three-valued outcome.
    #[test]
    fn both_gates_fire_under_not_allowed() {
        let p = Policies {
            invalid: InvalidVotePolicy::NotAllowed,
            ..Default::default()
        };
        let out = f(
            &cfg(0, 2, p),
            &VoteState {
                explicit_invalid: true,
                ..Default::default()
            },
        );
        assert_eq!(
            out.gate,
            Gate {
                hard: true,
                soft: true
            }
        );
        assert_eq!(out.dialog, Dialog::Blocking);
    }
}
