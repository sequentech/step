// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(non_camel_case_types)]

use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::default::Default;
use strum_macros::{Display, EnumString};

#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    Default,
)]
pub enum KeysCeremonyExecutionStatus {
    USER_CONFIGURATION, // user can configure the ceremony at this step
    #[default]
    STARTED, /* process starts but the config message hasn't
                         * been added to the board */
    IN_PROGRESS, /* config message has been added to the board and trustees
                  * are working */
    SUCCESS,   // successful completion
    CANCELLED, // cancelation
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Log {
    pub created_date: String,
    pub log_text: String,
}

#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString,
)]
pub enum TrusteeStatus {
    WAITING,
    KEY_GENERATED,
    KEY_RETRIEVED,
    KEY_CHECKED,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Trustee {
    pub name: String,
    pub status: TrusteeStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeysCeremonyStatus {
    pub stop_date: Option<String>,
    pub public_key: Option<String>,
    pub logs: Vec<Log>,
    pub trustees: Vec<Trustee>,
}

#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    Default,
    JsonSchema,
)]
pub enum TallyExecutionStatus {
    #[default]
    STARTED,
    CONNECTED,
    IN_PROGRESS,
    AWAITING_INPUT,
    SUCCESS,
    CANCELLED,
}

/// Why a tally session execution was created, recorded on the
/// `tally_session_execution` row itself so that the reason survives the loss of
/// the celery message that would otherwise carry it.
///
/// The task reads the reason from the newest execution row, and a completed run
/// appends a fresh `NORMAL` row -- so finishing the work consumes the reason
/// without a separate clearing step, while a run that bails out early writes no
/// row at all and is therefore retried by the next `process_board` tick.
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    Default,
    JsonSchema,
)]
pub enum TallyRunReason {
    /// Advance the tally with board messages that have not been processed yet.
    #[default]
    NORMAL,
    /// Re-run a completed session over the same board messages, producing a
    /// fresh results event.
    RECOUNT,
    /// Re-run after tie-break resolutions were submitted.
    TIE_BREAK_RERUN,
}

#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    Default,
)]
pub enum TallyTrusteeStatus {
    #[default]
    WAITING,
    KEY_RESTORED,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RestorePrivateKeyOutcome {
    Restored,
    Invalid,
    AlreadyRestored,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TallyTrustee {
    pub name: String,
    pub status: TallyTrusteeStatus,
}

#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    Default,
)]
pub enum TallyElectionStatus {
    #[default]
    WAITING,
    MIXING,
    DECRYPTING,
    SUCCESS,
    ERROR,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TallyElection {
    pub election_id: String,
    pub status: TallyElectionStatus,
    pub progress: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TallyCeremonyStatus {
    pub stop_date: Option<String>,
    pub logs: Vec<Log>,
    pub trustees: Vec<TallyTrustee>,
    pub elections_status: Vec<TallyElection>,
}

#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    Default,
    JsonSchema,
)]
pub enum TallyType {
    #[default]
    #[strum(serialize = "ELECTORAL_RESULTS")]
    ELECTORAL_RESULTS,
    #[strum(serialize = "INITIALIZATION_REPORT")]
    INITIALIZATION_REPORT,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct TallySessionDocuments {
    pub sqlite: Option<String>,
    pub xlsx: Option<String>,
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    Default,
    JsonSchema,
)]
pub enum CeremoniesPolicy {
    #[default]
    #[strum(serialize = "manual-ceremonies")]
    #[serde(rename = "manual-ceremonies")]
    MANUAL_CEREMONIES,
    #[strum(serialize = "automated-ceremonies")]
    #[serde(rename = "automated-ceremonies")]
    AUTOMATED_CEREMONIES,
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    Default,
    JsonSchema,
)]
pub enum AutomaticRecountPolicy {
    #[strum(serialize = "enabled")]
    #[serde(rename = "enabled")]
    ENABLED,
    #[default]
    #[strum(serialize = "disabled")]
    #[serde(rename = "disabled")]
    DISABLED,
}

#[derive(
    Debug,
    Display,
    EnumString,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
)]
pub enum TallyOperation {
    #[strum(serialize = "process-ballots-all")]
    #[serde(rename = "process-ballots-all")]
    ProcessBallotsAll, /* Process ballots to calculate Candidate Results
                        * and participation
                        * statistics */
    #[strum(serialize = "aggregate-results")]
    #[serde(rename = "aggregate-results")]
    AggregateResults, /* Aggregate results that have been processed in
                       * every area */
    #[strum(serialize = "skip-candidate-results")]
    #[serde(rename = "skip-candidate-results")]
    SkipCandidateResults, /* Needs the ballots to calculate participation
                           * statistics but without the Candidate Results */
}

#[derive(Debug, Display)]
pub enum ScopeOperation {
    Area(TallyOperation),
    Contest(TallyOperation),
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    JsonSchema,
    PartialEq,
    Eq,
)]
pub enum TieBreakingMethod {
    Random,
    ExternalProcedure,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    JsonSchema,
    PartialEq,
    Eq,
)]
pub struct TallySessionResolutionData {
    pub round_number: Option<u64>,
    pub tied_candidate_ids: Vec<String>,
    pub vote_count: u64,
    pub method_used: TieBreakingMethod,
    pub resolved_by_candidate_id: Option<String>,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Display, EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TallySessionResolutionType {
    IrvTieBreak,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Display, EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum TallySessionResolutionStatus {
    Pending,
    Resolved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TallySessionResolution {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub tally_session_id: String,
    pub contest_id: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub resolution_type: TallySessionResolutionType,
    pub status: TallySessionResolutionStatus,
    pub resolution_data: Option<TallySessionResolutionData>,
    pub resolved_by_user: Option<String>,
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TallyResolution {
    pub contest_id: String,
    pub selected_candidate_id: String,
}

#[derive(
    Eq,
    PartialEq,
    Debug,
    EnumString,
    Display,
    Default,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    JsonSchema,
    Clone,
    Copy,
)]
// Stored as free text on the Hasura `contest.counting_algorithm` column, so
// parsing tolerates case that differs from the canonical form rather than
// silently falling through to the default.
#[strum(ascii_case_insensitive)]
pub enum CountingAlgType {
    #[strum(serialize = "plurality-at-large")]
    #[serde(rename = "plurality-at-large")]
    #[default]
    PluralityAtLarge,
    #[strum(serialize = "instant-runoff")]
    #[serde(rename = "instant-runoff")]
    InstantRunoff,
    #[strum(serialize = "borda-nauru")]
    #[serde(rename = "borda-nauru")]
    BordaNauru,
    #[strum(serialize = "borda")]
    #[serde(rename = "borda")]
    Borda,
    #[strum(serialize = "borda-mas-madrid")]
    #[serde(rename = "borda-mas-madrid")]
    BordaMasMadrid,
    #[strum(serialize = "pairwise-beta")]
    #[serde(rename = "pairwise-beta")]
    PairwiseBeta,
    #[strum(serialize = "desborda3")]
    #[serde(rename = "desborda3")]
    Desborda3,
    #[strum(serialize = "desborda2")]
    #[serde(rename = "desborda2")]
    Desborda2,
    #[strum(serialize = "desborda")]
    #[serde(rename = "desborda")]
    Desborda,
    #[strum(serialize = "cumulative")]
    #[serde(rename = "cumulative")]
    Cumulative,
}

impl CountingAlgType {
    /// Returns true if the counting algorithm is preferential (ranked-choice).
    pub fn is_preferential(&self) -> bool {
        matches!(
            self,
            CountingAlgType::InstantRunoff
                | CountingAlgType::Borda
                | CountingAlgType::BordaNauru
                | CountingAlgType::BordaMasMadrid
                | CountingAlgType::PairwiseBeta
                | CountingAlgType::Desborda
                | CountingAlgType::Desborda2
                | CountingAlgType::Desborda3
        )
    }

    /// Returns true if a voter may give multiple points to the same
    /// candidate, so per-candidate marks must be bounded by a checkbox
    /// budget instead of a single mark per ballot.
    pub fn is_cumulative(&self) -> bool {
        matches!(self, CountingAlgType::Cumulative)
    }

    pub fn get_default_tally_operation_for_contest(&self) -> TallyOperation {
        if self.is_preferential() {
            TallyOperation::ProcessBallotsAll
        } else {
            TallyOperation::AggregateResults
        }
    }

    pub fn get_default_tally_operation_for_area(&self) -> TallyOperation {
        if self.is_preferential() {
            TallyOperation::SkipCandidateResults
        } else {
            TallyOperation::ProcessBallotsAll
        }
    }
}
