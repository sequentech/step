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

/// Progress of a distributed key-generation ceremony on the bulletin board.
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
    /// Trustees and administrators may still configure ceremony parameters.
    USER_CONFIGURATION, // user can configure the ceremony at this step
    /// Ceremony process starts but the config message hasn't
    /// been added to the board
    #[default]
    STARTED,
    /// config message has been added to the board and trustees are working
    IN_PROGRESS,
    /// All trustees completed key generation successfully.
    SUCCESS,
    /// Ceremony was cancelled before completion.
    CANCELLED,
}

/// A timestamped log entry from a ceremony worker process.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Log {
    /// ISO 8601 timestamp when the log line was recorded.
    pub created_date: String,
    /// Human-readable log message.
    pub log_text: String,
}

/// Progress of an individual trustee during key generation.
#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString,
)]
pub enum TrusteeStatus {
    /// Trustee has not yet acted in this ceremony step.
    WAITING,
    /// Trustee generated their key share locally.
    KEY_GENERATED,
    /// Trustee retrieved the combined public key from the board.
    KEY_RETRIEVED,
    /// Trustee verified the published public key.
    KEY_CHECKED,
}

/// A trustee participating in a key-generation ceremony.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Trustee {
    /// Display name of the trustee.
    pub name: String,
    /// Current step status for this trustee.
    pub status: TrusteeStatus,
}

/// Live status snapshot of a key-generation ceremony.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeysCeremonyStatus {
    /// When the ceremony was stopped, if applicable.
    pub stop_date: Option<String>,
    /// Election public key produced by the ceremony, once available.
    pub public_key: Option<String>,
    /// Chronological log entries from the ceremony process.
    pub logs: Vec<Log>,
    /// Per-trustee progress within the ceremony.
    pub trustees: Vec<Trustee>,
}

/// Progress of a tally (mixnet + decryption) ceremony.
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
    /// Tally process has been initiated.
    #[default]
    STARTED,
    /// All trustees connected to the tally coordinator.
    CONNECTED,
    /// Mixing/decryption is actively running.
    IN_PROGRESS,
    /// Waiting for administrator input (e.g. tie resolution).
    AWAITING_INPUT,
    /// Tally completed successfully.
    SUCCESS,
    /// Tally was cancelled before completion.
    CANCELLED,
}

/// Progress of an individual trustee during tallying.
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
    /// Trustee has not yet restored their key for this tally.
    #[default]
    WAITING,
    /// Trustee restored their decryption key share.
    KEY_RESTORED,
}

/// A trustee participating in a tally ceremony.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TallyTrustee {
    /// Display name of the trustee.
    pub name: String,
    /// Current step status for this trustee.
    pub status: TallyTrusteeStatus,
}

/// Per-election progress within a multi-election tally ceremony.
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
    /// Election queued but not yet being processed.
    #[default]
    WAITING,
    /// Mixnet shuffle is running for this election.
    MIXING,
    /// Ballots are being decrypted for this election.
    DECRYPTING,
    /// Tally completed for this election.
    SUCCESS,
    /// An error occurred while tallying this election.
    ERROR,
}

/// Tally progress for one election within a ceremony.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TallyElection {
    /// Election being tallied.
    pub election_id: String,
    /// Current processing stage.
    pub status: TallyElectionStatus,
    /// Completion fraction in the range 0.0–1.0.
    pub progress: f64,
}

/// Live status snapshot of a tally ceremony across all elections.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TallyCeremonyStatus {
    /// When the ceremony was stopped, if applicable.
    pub stop_date: Option<String>,
    /// Chronological log entries from the tally process.
    pub logs: Vec<Log>,
    /// Per-trustee progress within the ceremony.
    pub trustees: Vec<TallyTrustee>,
    /// Per-election processing status and progress.
    pub elections_status: Vec<TallyElection>,
}

/// Kind of report produced by a tally session.
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
    /// Standard electoral results tally.
    #[default]
    #[strum(serialize = "ELECTORAL_RESULTS")]
    ELECTORAL_RESULTS,
    /// Initialization report tally (pre-voting verification).
    #[strum(serialize = "INITIALIZATION_REPORT")]
    INITIALIZATION_REPORT,
}

/// Document references produced by a completed tally session.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct TallySessionDocuments {
    /// URL or path to the SQLite results export.
    pub sqlite: Option<String>,
    /// URL or path to the Excel results export.
    pub xlsx: Option<String>,
}

/// Whether key-generation and tally ceremonies are run manually or automated.
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
    /// Administrators trigger ceremonies explicitly from the admin portal.
    #[default]
    #[strum(serialize = "manual-ceremonies")]
    #[serde(rename = "manual-ceremonies")]
    MANUAL_CEREMONIES,
    /// Ceremonies start automatically based on election schedule and status.
    #[strum(serialize = "automated-ceremonies")]
    #[serde(rename = "automated-ceremonies")]
    AUTOMATED_CEREMONIES,
}

/// What a tally pass computes at a given scope (area or contest).
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
    /// Process ballots to calculate Candidate Results
    /// and participation statistics
    #[strum(serialize = "process-ballots-all")]
    #[serde(rename = "process-ballots-all")]
    ProcessBallotsAll,
    /// Aggregate results that have been processed in every area
    #[strum(serialize = "aggregate-results")]
    #[serde(rename = "aggregate-results")]
    AggregateResults,
    /// Needs the ballots to calculate participation statistics
    /// but without the Candidate Results
    #[strum(serialize = "skip-candidate-results")]
    #[serde(rename = "skip-candidate-results")]
    SkipCandidateResults,
}

/// Pairs a [`TallyOperation`] with the scope (area or contest) it applies to.
#[derive(Debug, Display)]
pub enum ScopeOperation {
    /// Operation applied at geographic area level.
    Area(TallyOperation),
    /// Operation applied at individual contest level.
    Contest(TallyOperation),
}

/// Method used to break a tie during IRV or similar preferential counting.
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
    /// Resolve the tie by random selection.
    Random,
    /// Resolve via an external procedure recorded by administrators.
    ExternalProcedure,
}

/// Details of a tie encountered during preferential counting.
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
    /// Preferential counting round where the tie occurred.
    pub round_number: Option<u64>,
    /// Candidate IDs involved in the tie.
    pub tied_candidate_ids: Vec<String>,
    /// Vote count shared by the tied candidates at that round.
    pub vote_count: u64,
    /// How the tie was or will be broken.
    pub method_used: TieBreakingMethod,
    /// Winning candidate chosen to break the tie, once resolved.
    pub resolved_by_candidate_id: Option<String>,
}

/// Category of manual intervention required during tallying.
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Display, EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TallySessionResolutionType {
    /// Instant-runoff tie requiring administrator selection.
    IrvTieBreak,
}

/// Whether a tally-session resolution has been completed.
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Display, EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum TallySessionResolutionStatus {
    /// Awaiting administrator action.
    Pending,
    /// Tie has been resolved and recorded.
    Resolved,
}

/// Database record tracking a tie or other issue requiring manual tally input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TallySessionResolution {
    /// Unique resolution record identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Tally session this resolution belongs to.
    pub tally_session_id: String,
    /// Contest requiring resolution, when scoped to one contest.
    pub contest_id: Option<String>,
    /// Record creation timestamp.
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Last modification timestamp.
    pub last_updated_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Kind of resolution required.
    pub resolution_type: TallySessionResolutionType,
    /// Current resolution state.
    pub status: TallySessionResolutionStatus,
    /// Tie details and outcome, when applicable.
    pub resolution_data: Option<TallySessionResolutionData>,
    /// Administrator who resolved the issue.
    pub resolved_by_user: Option<String>,
    /// When the resolution was finalized.
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Labels for UI filtering.
    pub labels: Option<Value>,
    /// Metadata
    pub annotations: Option<Value>,
}

/// Administrator's tie-breaking choice submitted to the tally pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TallyResolution {
    /// Contest where the tie was resolved.
    pub contest_id: String,
    /// Candidate selected to break the tie.
    pub selected_candidate_id: String,
}

/// Ballot counting algorithm configured on a contest.
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
pub enum CountingAlgType {
    /// Multi-seat plurality (vote for up to N candidates).
    #[strum(serialize = "plurality-at-large")]
    #[serde(rename = "plurality-at-large")]
    #[default]
    PluralityAtLarge,
    /// Instant-runoff voting (ranked-choice, single winner).
    #[strum(serialize = "instant-runoff")]
    #[serde(rename = "instant-runoff")]
    InstantRunoff,
    /// Borda count using Nauru-style diminishing weights.
    #[strum(serialize = "borda-nauru")]
    #[serde(rename = "borda-nauru")]
    BordaNauru,
    /// Standard Borda count.
    #[strum(serialize = "borda")]
    #[serde(rename = "borda")]
    Borda,
    /// Borda count with Madrid-specific tie rules.
    #[strum(serialize = "borda-mas-madrid")]
    #[serde(rename = "borda-mas-madrid")]
    BordaMasMadrid,
    /// Pairwise comparison with beta distribution tie handling.
    #[strum(serialize = "pairwise-beta")]
    #[serde(rename = "pairwise-beta")]
    PairwiseBeta,
    /// Desborda variant with three-point scale.
    #[strum(serialize = "desborda3")]
    #[serde(rename = "desborda3")]
    Desborda3,
    /// Desborda variant with two-point scale.
    #[strum(serialize = "desborda2")]
    #[serde(rename = "desborda2")]
    Desborda2,
    /// Standard Desborda preferential counting.
    #[strum(serialize = "desborda")]
    #[serde(rename = "desborda")]
    Desborda,
    /// Cumulative voting (multiple votes per candidate up to a total).
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

    /// Default tally operation when processing at contest scope for this algorithm.
    pub fn get_default_tally_operation_for_contest(&self) -> TallyOperation {
        if self.is_preferential() {
            TallyOperation::ProcessBallotsAll
        } else {
            TallyOperation::AggregateResults
        }
    }

    /// Default tally operation when processing at area scope for this algorithm.
    pub fn get_default_tally_operation_for_area(&self) -> TallyOperation {
        if self.is_preferential() {
            TallyOperation::SkipCandidateResults
        } else {
            TallyOperation::ProcessBallotsAll
        }
    }
}
