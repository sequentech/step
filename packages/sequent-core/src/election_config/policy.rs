// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! How a ballot behaves, in the platform's own words.
//!
//! Every enum here is one of the platform's, variant for variant, and serialises
//! as the exact string `contest.presentation` holds. There is **no mapping
//! table**, and that is the point: a plan carries what the bundle will say, so
//! nothing can be lossy and no value the platform rejects can be invented.
//!
//! # Why not a friendlier vocabulary
//!
//! The obvious design — and the one both previous implementations chose — is a
//! small `Allowed | Warn | Restricted` and a function that maps it to the wire.
//! It reads better and it does not work:
//!
//! * `Restricted` for an under-vote mapped to `not-allowed`, which is not a
//!   member of `EUnderVotePolicy`;
//! * `Warn` for an over-vote, shown only in review, mapped to
//!   `warn-only-in-review`, which is not a member of `EOverVotePolicy` either;
//! * candidate order mapped `alphabetic` when `CandidatesOrder` says
//!   `alphabetical`.
//!
//! Three escapes from the value space in one thirty-line file, none of which a
//! type could catch, and every one of them a contest that imports cleanly and
//! then behaves in a way nobody chose.
//!
//! The deeper objection is that it makes the *plan* a lower-fidelity document
//! than the bundle it produces, so the wizard can never express a configuration
//! the Admin Portal can — which makes it a dead end for every election past the
//! simplest.
//!
//! # The dropdown-maze worry
//!
//! Five values for over-vote is a lot to put in front of somebody. But
//! `packages/admin-portal/src/translations/*.ts` already keys human labels off
//! these exact strings, in eight languages, so the enums *are* the UI
//! vocabulary and there is nothing to map. A front end shows the three presets
//! below and reveals the real values behind a "customise" disclosure.
//!
//! # Where this lives
//!
//! Beside the architect rather than inside it, and ungated, because
//! [`super::validate`] needs the value space to check a bundle and carries no
//! feature.

use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

use super::paths::Cell;

/// A `contest.presentation` value whose variants are exactly the platform's.
pub trait PolicyValue: Copy + Sized + 'static {
    /// The column this is written to.
    const COLUMN: &'static str;

    /// The translation namespace in the Admin Portal, so a front end can label
    /// a value without a second table.
    const LABELS: &'static str;

    /// Every value, most permissive first — the order somebody reasons in.
    fn values() -> Vec<Self>;

    fn as_str(self) -> &'static str;
}

macro_rules! policy_value {
    (
        $(#[$outer:meta])*
        $name:ident, column = $column:literal, labels = $labels:literal,
        default = $default:ident,
        { $( $(#[$inner:meta])* $variant:ident ),* $(,)? }
    ) => {
        $(#[$outer])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash,
            Serialize, Deserialize, Display, EnumIter, EnumString, IntoStaticStr,
        )]
        #[serde(rename_all = "kebab-case")]
        #[strum(serialize_all = "kebab-case")]
        pub enum $name {
            $( $(#[$inner])* $variant, )*
        }

        impl Default for $name {
            fn default() -> Self { $name::$default }
        }

        impl PolicyValue for $name {
            const COLUMN: &'static str = $column;
            const LABELS: &'static str = $labels;
            fn values() -> Vec<Self> { vec![ $( $name::$variant, )* ] }
            fn as_str(self) -> &'static str { self.into() }
        }
    };
}

policy_value! {
    /// What the ballot does when a voter selects more than `max_votes`.
    ///
    /// `EOverVotePolicy`. Note it has no plain `warn`: over-voting is either
    /// permitted with a message or refused, and there is no "warn in review"
    /// — which is the value the previous implementation emitted for it.
    OverVote, column = "presentation.over_vote_policy",
    labels = "overVotePolicy", default = NotAllowedWithMsgAndDisable,
    {
        Allowed,
        AllowedWithMsg,
        AllowedWithMsgAndAlert,
        NotAllowedWithMsgAndAlert,
        NotAllowedWithMsgAndDisable,
    }
}

policy_value! {
    /// Choosing nothing at all. `EBlankVotePolicy`.
    BlankVote, column = "presentation.blank_vote_policy",
    labels = "blankVotePolicy", default = WarnOnlyInReview,
    { Allowed, Warn, WarnOnlyInReview, NotAllowed }
}

policy_value! {
    /// Choosing fewer than `max_votes`. `EUnderVotePolicy`.
    ///
    /// No `not-allowed`: an under-vote cannot be refused, only warned about,
    /// which is why mapping a "restricted" choice onto one produced a value the
    /// platform does not have.
    UnderVote, column = "presentation.under_vote_policy",
    labels = "underVotePolicy", default = WarnOnlyInReview,
    { Allowed, Warn, WarnOnlyInReview, WarnAndAlert }
}

policy_value! {
    /// Deliberately spoiling the ballot. `EInvalidVotePolicy`.
    InvalidVote, column = "presentation.invalid_vote_policy",
    labels = "invalidVotePolicy", default = WarnInvalidImplicitAndExplicit,
    { Allowed, Warn, WarnInvalidImplicitAndExplicit, NotAllowed }
}

policy_value! {
    /// Ranking two candidates equally. `EDuplicatedRankPolicy`.
    DuplicatedRank, column = "presentation.duplicated_rank_policy",
    labels = "duplicatedRankPolicy", default = AllowedWarnAndDialog,
    { AllowedWarnAndDialog, NotAllowedWarnAndDialog }
}

policy_value! {
    /// Leaving a gap in a ranking. `EPreferenceGapsPolicy`.
    PreferenceGaps, column = "presentation.preference_gaps_policy",
    labels = "preferenceGapsPolicy", default = AllowedWarnAndDialog,
    { AllowedWarnAndDialog, NotAllowedWarnAndDialog }
}

policy_value! {
    /// The order candidates appear in. `CandidatesOrder`.
    ///
    /// `Random` is a written fairness requirement for several clients, and
    /// `Custom` means the order the plan lists them in.
    CandidatesOrder, column = "presentation.candidates_order",
    labels = "candidatesOrder", default = Custom,
    { Custom, Alphabetical, Random }
}

/// Declare a set of policies and its patch together.
///
/// One invocation, two types. Adding a policy to only one of them is the
/// mistake this exists to make impossible — and it is the one that would
/// happen, because the patch is the type nobody remembers.
macro_rules! policy_set {
    ($resolved:ident / $patch:ident { $( $field:ident : $ty:ty ),* $(,)? }) => {
        /// What a contest ends up behaving like.
        #[derive(
            Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize,
        )]
        pub struct $resolved {
            $( #[serde(default)] pub $field: $ty, )*
        }

        /// What one level *says*, as opposed to what a contest *gets*.
        #[derive(
            Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize,
        )]
        pub struct $patch {
            $(
                #[serde(default, skip_serializing_if = "Option::is_none")]
                pub $field: Option<$ty>,
            )*
        }

        impl $resolved {
            /// This set with everything the patch names replaced.
            pub fn apply(self, patch: &$patch) -> Self {
                $resolved { $( $field: patch.$field.unwrap_or(self.$field), )* }
            }

            /// The sheet columns this set writes.
            pub fn columns(self) -> Vec<(&'static str, Cell)> {
                vec![ $( (<$ty>::COLUMN, Cell::text(self.$field.as_str())), )* ]
            }
        }

        impl $patch {
            pub fn is_empty(&self) -> bool { true $( && self.$field.is_none() )* }
        }
    };
}

policy_set!(
    Policies
        / PolicyPatch {
            over_vote: OverVote,
            blank_vote: BlankVote,
            under_vote: UnderVote,
            invalid_vote: InvalidVote,
            duplicated_rank: DuplicatedRank,
            preference_gaps: PreferenceGaps,
            candidates_order: CandidatesOrder,
        }
);

impl Policies {
    /// Accept whatever the voter does, and say nothing.
    ///
    /// For an election where a spoiled or partial ballot is a legitimate
    /// choice rather than a mistake to catch.
    pub fn permissive() -> Self {
        Policies {
            over_vote: OverVote::Allowed,
            blank_vote: BlankVote::Allowed,
            under_vote: UnderVote::Allowed,
            invalid_vote: InvalidVote::Allowed,
            ..Default::default()
        }
    }

    /// Say something, but let the voter through. The platform's own defaults.
    pub fn standard() -> Self {
        Policies::default()
    }

    /// Refuse what can be refused, and warn loudly about the rest.
    pub fn strict() -> Self {
        Policies {
            over_vote: OverVote::NotAllowedWithMsgAndDisable,
            blank_vote: BlankVote::NotAllowed,
            // Not `NotAllowed` — `EUnderVotePolicy` has no such value, and
            // inventing one is exactly the bug this module exists to prevent.
            under_vote: UnderVote::WarnAndAlert,
            invalid_vote: InvalidVote::NotAllowed,
            duplicated_rank: DuplicatedRank::NotAllowedWarnAndDialog,
            preference_gaps: PreferenceGaps::NotAllowedWarnAndDialog,
            ..Default::default()
        }
    }
}

/// How a contest is voted and counted.
///
/// Separate from [`Policies`] because these land on the contest itself rather
/// than on its presentation, and because two of them have to agree:
/// [`super::validate`] refuses a preferential contest counted by plurality,
/// which imports cleanly and then reads a voter's rankings as an unordered set.
///
/// Until this existed the wizard could not produce a ranked election at all —
/// every contest took `contest.hbs`'s defaults, which are non-preferential and
/// plurality-at-large.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tally {
    /// `preferential` or `non-preferential`, per the Admin Portal's `IVotingType`.
    #[serde(default = "non_preferential")]
    pub voting_type: String,

    /// One of [`super::validate::COUNTING_ALGORITHMS`].
    #[serde(default = "plurality")]
    pub counting_algorithm: String,

    /// How few a voter may choose. Zero means a blank ballot is a ballot.
    ///
    /// Was hard-coded to zero, with a comment saying the wizard does not ask —
    /// so "rank at least three" was unexpressible.
    #[serde(default)]
    pub min_votes: i64,

    /// Whether ballots are encrypted. The difference between an election and a
    /// poll, and unrecoverable if wrong.
    #[serde(default = "yes")]
    pub is_encrypted: bool,

    /// How a tie for the last seat is settled — `ITieBreakingPolicy`.
    ///
    /// `random` breaks it inside the tally, deterministically from the ballot
    /// box; `external-procedure` leaves the tie in the result for whoever runs
    /// the election to settle by their own rules — a coin toss, a casting vote,
    /// a re-run.
    ///
    /// A `String` for the same reason `counting_algorithm` is: the value space
    /// belongs to the platform, `validate` checks it against
    /// [`super::validate::TIE_BREAKING_POLICIES`], and a typed enum here would
    /// be a third copy of a list that already exists twice.
    #[serde(default = "external_procedure")]
    pub tie_breaking_policy: String,
}

fn non_preferential() -> String {
    "non-preferential".to_string()
}

fn plurality() -> String {
    "plurality-at-large".to_string()
}

/// The platform's own default, and the safer one.
///
/// A random tie-break is defensible and it is also a result nobody can explain
/// from the ballots alone, which is exactly the result that gets challenged.
/// Leaving the tie visible makes it somebody's decision, taken under their own
/// rules, on the record.
fn external_procedure() -> String {
    "external-procedure".to_string()
}

fn yes() -> bool {
    true
}

impl Default for Tally {
    fn default() -> Self {
        Tally {
            voting_type: non_preferential(),
            counting_algorithm: plurality(),
            min_votes: 0,
            is_encrypted: true,
            tie_breaking_policy: external_procedure(),
        }
    }
}

/// What a level says about how its contests are counted.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TallyPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voting_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counting_algorithm: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_votes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_encrypted: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tie_breaking_policy: Option<String>,
}

impl Tally {
    pub fn apply(&self, patch: &TallyPatch) -> Tally {
        Tally {
            voting_type: patch
                .voting_type
                .clone()
                .unwrap_or_else(|| self.voting_type.clone()),
            counting_algorithm: patch
                .counting_algorithm
                .clone()
                .unwrap_or_else(|| self.counting_algorithm.clone()),
            min_votes: patch.min_votes.unwrap_or(self.min_votes),
            is_encrypted: patch.is_encrypted.unwrap_or(self.is_encrypted),
            tie_breaking_policy: patch
                .tie_breaking_policy
                .clone()
                .unwrap_or_else(|| self.tie_breaking_policy.clone()),
        }
    }

    /// The columns this writes. Not `presentation.*` — these are the contest's.
    pub fn columns(&self) -> Vec<(&'static str, Cell)> {
        vec![
            ("voting_type", Cell::text(self.voting_type.clone())),
            (
                "counting_algorithm",
                Cell::text(self.counting_algorithm.clone()),
            ),
            ("min_votes", Cell::Int(self.min_votes)),
            ("is_encrypted", Cell::Bool(self.is_encrypted)),
            // Dotted, because it lives under the contest's
            // `tally_configuration` rather than on the contest itself.
            // `paths::set` splits on the dot, so a nested destination costs
            // nothing here.
            (
                "tally_configuration.tie_breaking_policy",
                Cell::text(self.tie_breaking_policy.clone()),
            ),
        ]
    }
}

impl TallyPatch {
    pub fn is_empty(&self) -> bool {
        self.voting_type.is_none()
            && self.counting_algorithm.is_none()
            && self.min_votes.is_none()
            && self.is_encrypted.is_none()
    }
}

/// How a contest's options are arranged on the page.
///
/// Its own group rather than more fields on [`Policies`], for two reasons. The
/// `policy_set!` macro requires every field to be an enum with a `COLUMN` and an
/// `as_str()`, and these are a number and two enums with no natural default of
/// that shape. And they are a different kind of decision: `Policies` is about
/// what happens when a voter does something unusual, and this is about what the
/// page looks like before they do anything at all.
///
/// Every one of these is exposed per contest in the Admin Portal's own
/// `EditContestDataForm`, so an operator could set them and a wizard user could
/// not — which is the gap this closes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layout {
    /// How many columns the options are laid out in. One is a single list.
    ///
    /// The Voting Portal reads `presentation.columns` and the wizard's own
    /// ballot preview already honours it, so this was visible in the preview and
    /// unsettable in the plan.
    #[serde(default = "one_column")]
    pub columns: i64,

    /// Whether lists of candidates can be folded up — `ECollapsibleLists`.
    ///
    /// Only means anything for a contest whose candidates are grouped into
    /// lists. `disabled` for everything else, which is the default.
    #[serde(default = "disabled")]
    pub collapsible_lists: String,

    /// What a voter may select when candidates are grouped —
    /// `EEnableCheckableLists`.
    ///
    /// A whole list, individual candidates, or either. `disabled` for an
    /// ungrouped contest.
    #[serde(default = "disabled")]
    pub enable_checkable_lists: String,

    /// A cap per candidate type, for a contest whose candidates carry types.
    ///
    /// Zero means no cap, which is the default and is what an ordinary contest
    /// wants — `max_votes` is the only limit there.
    #[serde(default)]
    pub max_selections_per_type: i64,
}

impl Default for Layout {
    fn default() -> Self {
        Layout {
            columns: 1,
            collapsible_lists: "disabled".to_string(),
            enable_checkable_lists: "disabled".to_string(),
            max_selections_per_type: 0,
        }
    }
}

/// What one level says about the layout.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collapsible_lists: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_checkable_lists: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_selections_per_type: Option<i64>,
}

impl Layout {
    pub fn apply(&self, patch: &LayoutPatch) -> Layout {
        Layout {
            columns: patch.columns.unwrap_or(self.columns),
            collapsible_lists: patch
                .collapsible_lists
                .clone()
                .unwrap_or_else(|| self.collapsible_lists.clone()),
            enable_checkable_lists: patch
                .enable_checkable_lists
                .clone()
                .unwrap_or_else(|| self.enable_checkable_lists.clone()),
            max_selections_per_type: patch
                .max_selections_per_type
                .unwrap_or(self.max_selections_per_type),
        }
    }

    /// The columns this writes, all under the contest's presentation.
    pub fn columns_for_sheet(&self) -> Vec<(&'static str, Cell)> {
        vec![
            ("presentation.columns", Cell::Int(self.columns)),
            (
                "presentation.collapsible_lists",
                Cell::text(self.collapsible_lists.clone()),
            ),
            (
                "presentation.enable_checkable_lists",
                Cell::text(self.enable_checkable_lists.clone()),
            ),
            (
                "presentation.max_selections_per_type",
                Cell::Int(self.max_selections_per_type),
            ),
        ]
    }
}

impl LayoutPatch {
    pub fn is_empty(&self) -> bool {
        self.columns.is_none()
            && self.collapsible_lists.is_none()
            && self.enable_checkable_lists.is_none()
            && self.max_selections_per_type.is_none()
    }
}

fn one_column() -> i64 {
    1
}

fn disabled() -> String {
    "disabled".to_string()
}

/// What a level says about how its contests behave, all together.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Overrides {
    #[serde(default)]
    pub policies: PolicyPatch,
    #[serde(default)]
    pub tally: TallyPatch,
    #[serde(default)]
    pub layout: LayoutPatch,
}

impl Overrides {
    pub fn is_empty(&self) -> bool {
        self.policies.is_empty()
            && self.tally.is_empty()
            && self.layout.is_empty()
    }
}

/// Everything a contest ends up with.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Behaviour {
    #[serde(default)]
    pub policies: Policies,
    #[serde(default)]
    pub tally: Tally,
    #[serde(default)]
    pub layout: Layout,
}

impl Behaviour {
    /// This, with one level's overrides applied.
    pub fn apply(&self, overrides: &Overrides) -> Behaviour {
        Behaviour {
            policies: self.policies.apply(&overrides.policies),
            tally: self.tally.apply(&overrides.tally),
            layout: self.layout.apply(&overrides.layout),
        }
    }

    /// Every column, presentation and contest together.
    pub fn columns(&self) -> Vec<(&'static str, Cell)> {
        let mut all = self.policies.columns();
        all.extend(self.tally.columns());
        all.extend(self.layout.columns_for_sheet());
        all
    }

    /// Whether `field` is a policy and `value` one of the values it has.
    ///
    /// The one gate a client profile's own presets pass through. Presets live in
    /// Rust rather than as a list of strings the browser renders precisely so
    /// that a client-facing button cannot select a behaviour no importer
    /// accepts — two earlier versions of this tool shipped three such values
    /// between them, each an election that imports cleanly and then behaves in
    /// a way nobody chose.
    ///
    /// Deliberately spelled by round-tripping through the patch's own
    /// deserializer rather than by a `match` over field names. A `match` is a
    /// second list of the policies, and the way a second list fails is a policy
    /// added to one and forgotten in the other — so this asks the type that
    /// already knows.
    pub fn accepts(&self, field: &str, value: &str) -> Result<(), String> {
        let one = serde_json::json!({ field: value });

        // Unknown fields are refused rather than ignored, so a typo in a
        // profile is a load error instead of a preset that quietly does less
        // than it says.
        let patch: Result<PolicyPatch, _> = serde_json::from_value(one.clone());
        if let Ok(patch) = patch {
            return if patch.is_empty() {
                Err(format!(
                    "'{field}' is not a ballot rule. The rules are the fields \
                     `policyCatalog()` lists."
                ))
            } else {
                Ok(())
            };
        }

        let tally: Result<TallyPatch, _> = serde_json::from_value(one);
        match tally {
            Ok(patch) if !patch.is_empty() => Ok(()),
            Ok(_) => Err(format!(
                "'{field}' is not a ballot rule. The rules are the fields \
                 `policyCatalog()` lists."
            )),
            Err(why) => Err(format!(
                "'{value}' is not one of the values '{field}' has: {why}"
            )),
        }
    }
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod policy_tests;
