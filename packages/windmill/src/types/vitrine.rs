// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The Hasura actions of Vitrine, which Harvest handles.

use sequent_core::ballot::TransparencyMode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum_macros::{AsRefStr, Display, EnumString};

#[derive(Debug, Clone, Deserialize)]
pub struct GetTransparencyInput {
    pub election_event_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EnableTransparencyInput {
    pub election_event_id: String,
    pub mode: TransparencyMode,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UploadTransparencyPolicyInput {
    pub election_event_id: String,
    pub signed_policy: String,
}

/// An election event's transparency, as the admin portal's Transparency tab
/// shows it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TransparencyOutput {
    pub mode: TransparencyMode,
    /// Why `public` is refused, if it is.
    pub refusal: Option<String>,
    /// Voting opened: the mode and the policy can't change.
    pub locked: bool,
    /// The sidecar's `GET /admin/logs` entries for the event's logs.
    pub logs: Option<Value>,
    /// The sidecar's `GET <log>/status` of each of them, by log name.
    pub status: Option<Value>,
    /// The policy for the Board to seal with `vitrine policy seal`.
    pub draft_policy: Option<String>,
    pub policy: Option<String>,
    pub policy_id: Option<String>,
    /// `[{log, note}]`: each log's witness enrolment note.
    pub enrolments: Option<Value>,
    pub explorer_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VitrineVoterContextInput {
    pub election_event_id: String,
    pub election_id: String,
}

/// What the booth must hold before it encrypts (Vitrine's `specs/entry.md`,
/// section 5.2).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VitrineVoterContext {
    pub mode: TransparencyMode,
    pub policy: Option<String>,
    /// The `ballots` log's acknowledgement of the policy.
    pub policy_ack: Option<String>,
    /// `{statement, entry, chain}` of the election's config statement.
    pub config: Option<Value>,
    /// `{statement, entry, chain}` of the event's `voting-opened` milestone.
    pub opened: Option<Value>,
    /// The hex digests of the election's distinct ballot styles, sorted.
    pub style_hashes: Option<Vec<String>>,
    /// `<package> <version>` of the voting portal, as the config lists it.
    pub release: Option<String>,
}

/// The sidecar's voter routes that Harvest forwards for private events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum VoterRoute {
    Upgrade,
    Opening,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VitrineVoterForwardInput {
    pub election_event_id: String,
    pub route: String,
    pub body: String,
    pub hub: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VitrineForwardOutput {
    pub status: u16,
    pub body: Option<String>,
}

/// What a cast tells the voter's portal about the ballot's Vitrine entry.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, EnumString, AsRefStr,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum VitrineCastKind {
    /// The log sequenced the entry: `text` is its receipt.
    Receipt,
    /// The log promised to: `text` is the promise.
    Promise,
    /// Still queued when the wait ended: `text` is the entry.
    Queued,
    /// The log couldn't answer in time: `text` is the entry, if there is one.
    Unavailable,
}

/// Why a cast has no receipt or promise.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, EnumString, AsRefStr,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum VitrineCastReason {
    /// The log hadn't sequenced the entry when the wait ended.
    Queued,
    /// The log knows nothing of the entry.
    UnknownEntry,
    /// The log's checks refused the entry.
    Rejected,
    /// The log is frozen.
    Frozen,
    /// The sidecar has too many requests in flight.
    Busy,
    /// The sidecar answered with a server error.
    LogUnavailable,
    /// The sidecar didn't answer within the budget.
    Timeout,
    /// The sidecar can't be reached.
    Unreachable,
    /// Recent calls failed, so Harvest doesn't call the sidecar for now.
    BreakerOpen,
    /// The sidecar's answer isn't a receipt for the entry.
    BadResponse,
    /// The ballot box accepted the ballot without queueing its entry.
    NotQueued,
    /// The event has no ballots log.
    NoLog,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VitrineCast {
    pub kind: VitrineCastKind,
    /// The receipt, the promise, or the entry in base64: data, never shown.
    pub text: Option<String>,
    pub reason: Option<VitrineCastReason>,
}
