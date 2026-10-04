// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What a scheduled opening or closing will do, and why (VOTE-LIFECYCLE
//! design §5a–§5c). One function in windmill (`scheduled_outcome`) computes
//! an [`Explanation`]; the scheduler acts on it, the Admin Portal shows it and
//! the electoral log records it, so the three can't disagree.

use crate::ballot::{EInitializeReportPolicy, LifecyclePolicies};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use strum_macros::{Display, EnumString};

/// What happens when the scheduled time arrives.
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Display, EnumString,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum ScheduledOutcomeKind {
    /// Runs: no signatures needed, or authorized by a signed configuration.
    Runs,
    /// A close that runs at its deadline without signatures (policy).
    RunsUnsigned,
    /// Doesn't run; people take the action (with signatures).
    Refused,
    /// Authorized opening remains pending until initialization is complete.
    WaitingForInitialization,
}

/// The questions an outcome depends on, in the order they are shown.
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Display, EnumString,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum CheckId {
    /// Does the action need signatures (current and published rule)?
    NeedsSignatures,
    /// Is this exact schedule in the signed configuration?
    Covered,
    /// (closes only) What happens without signatures?
    UnsignedClose,
    /// Do the current and the published copy differ, and which one decides?
    StricterCopy,
    /// Is anything published yet?
    Defaults,
    /// Current and published initialization requirements.
    Initialization,
    /// The effective current or authoritative signed close already passed.
    VotingClose,
}

/// One check: its values in the current and the published configuration,
/// and whether it allows the transition. Values are i18n message keys plus
/// parameters, so screens, logs and documents word them the same way.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Check {
    pub id: CheckId,
    pub current: CheckValue,
    pub published: Option<CheckValue>,
    pub allows: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CheckValue {
    pub message_key: String,
    #[serde(default)]
    pub params: serde_json::Map<String, serde_json::Value>,
}

/// The configuration approval that authorizes a covered transition.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AuthorizedBy {
    pub request_id: String,
    pub code: String,
    pub signers: Vec<String>,
}

/// An outcome with its reasons.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Explanation {
    pub outcome: ScheduledOutcomeKind,
    pub checks: Vec<Check>,
    /// The check that decided the outcome.
    pub deciding: CheckId,
    /// What would change the outcome (an i18n key), e.g. "publish and approve
    /// the configuration".
    pub next_step: CheckValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorized_by: Option<AuthorizedBy>,
}

/// A signing rule as a publication recorded it.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct RuleSnapshot {
    pub required: bool,
    pub signatures: Option<u32>,
    /// SHA-256 of the rule's canonical text.
    pub digest: String,
}

/// A scheduled opening or closing as a publication recorded it.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ScheduledTransition {
    pub scheduled_event_id: String,
    pub event_processor: String,
    pub election_id: Option<String>,
    pub scheduled_date: Option<String>,
    pub local: Option<String>,
    pub timezone: Option<String>,
    pub voting_channels: Option<Vec<String>>,
    /// SHA-256 of the canonical text of the fields above.
    pub fingerprint: String,
}

/// What each publication records (and a configuration approval signs) about
/// the lifecycle: the policies, the Open/Close voting rules and the schedule.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct LifecycleSnapshot {
    pub policies: LifecyclePolicies,
    pub open_voting: RuleSnapshot,
    pub close_voting: RuleSnapshot,
    pub schedule: Vec<ScheduledTransition>,
    /// Per-Post report requirements; old publications predate this evidence.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub initialization_report_policies:
        BTreeMap<String, EInitializeReportPolicy>,
    /// Frozen country membership of each Post in the generated publication.
    /// None predates this evidence; a present Post key can prove an empty list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initialization_countries: Option<BTreeMap<String, Vec<String>>>,
}
