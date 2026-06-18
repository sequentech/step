// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Hasura/PostgreSQL row types for the database schema.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use serde_json::value::Value;
use std::str::FromStr;

use crate::{
    ballot::{
        ConsolidatedReportPolicy, ContestEncryptionPolicy,
        DecodedBallotsInclusionPolicy, DelegatedVotingPolicy,
    },
    serialization::deserialize_with_path::deserialize_value,
    types::{
        ceremonies::{
            CeremoniesPolicy, KeysCeremonyExecutionStatus, KeysCeremonyStatus,
        },
        tally_sheets::AreaContestResults,
    },
};

/// A generated document preview request.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Preview {
    /// Unique preview identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Source document being previewed.
    pub document_id: String,
    /// URL where the preview can be accessed.
    pub url: String,
    /// User who requested the preview.
    pub requested_by: String,
    /// Creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last update timestamp.
    pub updated_at: Option<DateTime<Local>>,
    /// Metadata.
    pub annotations: Option<Value>,
}

/// A batch publication of cast ballots to the bulletin board.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct BallotPublication {
    /// Unique publication identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata.
    pub annotations: Option<Value>,
    /// Creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Soft-delete timestamp, when applicable.
    pub deleted_at: Option<DateTime<Local>>,
    /// Administrator who created the publication.
    pub created_by_user_id: Option<String>,
    /// When true, ballots were auto-generated rather than cast by voters.
    pub is_generated: Option<bool>,
    /// Elections included in this publication batch.
    pub election_ids: Option<Vec<String>>,
    /// When the publication was released to the bulletin board.
    pub published_at: Option<DateTime<Local>>,
    /// Single-election scope, when the publication covers one election only.
    pub election_id: Option<String>,
}

/// Database record linking a voter's ballot style to a publication batch.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct BallotStyle {
    /// Unique ballot-style record identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Election this ballot style belongs to.
    pub election_id: String,
    /// Geographic area scoping this ballot style.
    pub area_id: Option<String>,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata.
    pub annotations: Option<Value>,
    /// EML representation of the ballot layout.
    pub ballot_eml: Option<String>,
    /// Cryptographic signature over the ballot-style content.
    pub ballot_signature: Option<Vec<u8>>,
    /// Processing status of this ballot style record.
    pub status: Option<String>,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Soft-delete timestamp, when applicable.
    pub deleted_at: Option<DateTime<Local>>,
    /// Publication batch this ballot style was issued under.
    pub ballot_publication_id: String,
}

/// A geographic or organizational subdivision within an election event.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Area {
    /// Unique area identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Labels
    pub labels: Option<Value>,
    /// Structured metadata (weight, tally operation, etc.) as JSON.
    pub annotations: Option<Value>,
    /// Display name.
    pub name: Option<String>,
    /// Description text.
    pub description: Option<String>,
    /// Area classification (e.g. district, precinct).
    pub r#type: Option<String>,
    /// Parent area identifier for hierarchical geographies.
    pub parent_id: Option<String>,
    /// JSON presentation overrides (e.g. early voting policy).
    pub presentation: Option<Value>,
}

/// Top-level container for one or more elections run together.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ElectionEvent {
    /// Unique election event identifier.
    pub id: String,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub updated_at: Option<DateTime<Local>>,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata.
    pub annotations: Option<Value>,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Description text.
    pub description: Option<String>,
    /// JSON portal presentation and policy configuration.
    pub presentation: Option<Value>,
    /// Reference to the tamper-evident bulletin board for this event.
    pub bulletin_board_reference: Option<Value>,
    /// When true, the event is archived and read-only.
    pub is_archived: bool,
    /// Which delivery channels (online, kiosk, etc.) are enabled.
    pub voting_channels: Option<Value>,
    /// Voting status JSON (per-channel states and dates).
    pub status: Option<Value>,
    /// `ImmuDB` user-board identifier for audit logging.
    pub user_boards: Option<String>,
    /// Cryptographic protocol identifier (e.g. exponential `ElGamal`).
    pub encryption_protocol: String,
    /// When true, this is an audit clone of another event.
    pub is_audit: Option<bool>,
    /// Source event identifier when `is_audit` is true.
    pub audit_election_event_id: Option<String>,
    /// Election public key produced by the key ceremony.
    pub public_key: Option<String>,
    /// Notification counters and other aggregate statistics.
    pub statistics: Option<Value>,
    /// Client-specific external reference identifier.
    pub external_id: Option<String>,
}

/// A single election (race or referendum) within an election event.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Election {
    /// Unique election identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata.
    pub annotations: Option<Value>,
    /// Description text.
    pub description: Option<String>,
    /// JSON portal presentation and policy configuration.
    pub presentation: Option<Value>,
    /// Runtime voting status JSON (per-channel states and dates).
    pub status: Option<Value>,
    /// Serialized ballot layout (Election Markup Language) for this election.
    pub eml: Option<String>,
    /// Client-specific external reference identifier.
    pub external_id: Option<String>,
    /// Maximum number of times a voter may recast their ballot.
    pub num_allowed_revotes: Option<i64>,
    /// When true, multiple contests share one consolidated ballot encoding.
    pub is_consolidated_ballot_encoding: Option<bool>,
    /// When true, voters may explicitly spoil (invalidate) their ballot.
    pub spoil_ballot_option: Option<bool>,
    /// When true, this election is configured for kiosk-only voting.
    pub is_kiosk: Option<bool>,
    /// Enabled delivery channels (online, kiosk, etc.) as JSON.
    pub voting_channels: Option<Value>,
    /// Reference to a stored image document (logo, header, etc.).
    pub image_document_id: Option<String>,
    /// Aggregate counters and other election-level statistics.
    pub statistics: Option<Value>,
    /// Receipt configuration and delivery settings as JSON.
    pub receipts: Option<Value>,
    /// Keycloak permission label gating access to this election.
    pub permission_label: Option<String>,
    /// When true, the initialization report has been generated.
    pub initialization_report_generated: Option<bool>,
    /// Key ceremony that produced the encryption key for this election.
    pub keys_ceremony_id: Option<String>,
}

/// A contest (question or race) within an election.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Contest {
    /// Unique contest identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Parent election identifier.
    pub election_id: String,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata.
    pub annotations: Option<Value>,
    /// When true, the contest is won by acclamation (no vote required).
    pub is_acclaimed: Option<bool>,
    /// When false, the contest is excluded from active ballots.
    pub is_active: Option<bool>,
    /// Description text.
    pub description: Option<String>,
    /// JSON portal presentation and policy configuration.
    pub presentation: Option<Value>,
    /// Minimum number of selections the voter must make.
    pub min_votes: Option<i64>,
    /// Maximum number of selections the voter may make.
    pub max_votes: Option<i64>,
    /// Number of candidates that win this contest.
    pub winning_candidates_num: Option<i64>,
    /// Voting mechanism identifier (e.g. plurality, approval).
    pub voting_type: Option<String>,
    /// Tally algorithm identifier (e.g. plurality, Borda).
    pub counting_algorithm: Option<String>,
    /// When true, contest choices are encrypted on the ballot.
    pub is_encrypted: Option<bool>,
    /// Tally-session settings (tie resolution, etc.) as JSON.
    pub tally_configuration: Option<Value>,
    /// Reference to a stored image document for this contest.
    pub image_document_id: Option<String>,
    /// Eligibility or display conditions evaluated at ballot generation.
    pub conditions: Option<Value>,
    /// Client-specific external reference identifier.
    pub external_id: Option<String>,
}

/// A candidate or choice option within a contest.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    /// Unique candidate identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Parent contest identifier, when scoped to one contest.
    pub contest_id: Option<String>,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata.
    pub annotations: Option<Value>,
    /// Description text.
    pub description: Option<String>,
    /// Candidate classification (e.g. person, party, option).
    pub r#type: Option<String>,
    /// JSON portal presentation overrides.
    pub presentation: Option<Value>,
    /// When true, the candidate is visible on public-facing portals.
    pub is_public: Option<bool>,
    /// Reference to a stored image document (photo, logo, etc.).
    pub image_document_id: Option<String>,
    /// Client-specific external reference identifier.
    pub external_id: Option<String>,
}

/// A file stored in object storage and referenced by elections or events.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Unique document identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: Option<String>,
    /// Parent election event identifier, when event-scoped.
    pub election_event_id: Option<String>,
    /// Display name or original filename.
    pub name: Option<String>,
    /// MIME type of the stored file.
    pub media_type: Option<String>,
    /// File size in bytes.
    pub size: Option<i64>,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata.
    pub annotations: Option<Value>,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub last_updated_at: Option<DateTime<Local>>,
    /// When true, the document is accessible without authentication.
    pub is_public: Option<bool>,
}

/// Voter-facing informational content shown in the voting portal.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct SupportMaterial {
    /// Unique support-material identifier.
    pub id: String,
    /// Record creation timestamp.
    pub created_at: DateTime<Local>,
    /// Last modification timestamp.
    pub last_updated_at: DateTime<Local>,
    /// Content type discriminator (e.g. link, text, document).
    pub kind: String,
    /// Payload for the material (URL, HTML, etc.) as JSON.
    pub data: Value,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Labels
    pub labels: Value,
    /// Metadata.
    pub annotations: Value,
    /// Linked document identifier, when the material references a file.
    pub document_id: Option<String>,
    /// When true, the material is hidden from voters.
    pub is_hidden: Option<bool>,
}

/// Delivery channels enabled for voting at the election or event level.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct VotingChannels {
    /// Online (web) voting channel.
    pub online: Option<bool>,
    /// In-person kiosk voting channel.
    pub kiosk: Option<bool>,
    /// Telephone voting channel.
    pub telephone: Option<bool>,
    /// Paper ballot channel.
    pub paper: Option<bool>,
    /// Early voting period channel.
    pub early_voting: Option<bool>,
}

impl Default for VotingChannels {
    fn default() -> Self {
        Self {
            online: Some(true),
            kiosk: None,
            telephone: None,
            paper: None,
            early_voting: None,
        }
    }
}

/// Tenant-defined classification for elections (e.g. primary, general).
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ElectionType {
    /// Unique election-type identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: Option<String>,
    /// Display name.
    pub name: Option<String>,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub updated_at: Option<DateTime<Local>>,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata.
    pub annotations: Option<Value>,
}
/*
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct CastVote {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_id: Uuid,
    pub area_id: Uuid,
    pub created_at: Option<DateTime<Local>>,
    pub last_updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub content: Option<String>,
    pub cast_ballot_signature: Vec<u8>,
    pub voter_id_string: Option<String>,
    pub election_event_id: String,
    pub ballot_id: Option<String>,
}
*/

/// A reusable notification template (email, SMS, etc.) for a tenant.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    /// Unique template alias used when sending notifications.
    pub alias: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Template body and variables as JSON.
    pub template: Value,
    /// User who created the template.
    pub created_by: String,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata.
    pub annotations: Option<Value>,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub updated_at: Option<DateTime<Local>>,
    /// Delivery channel (e.g. email, sms).
    pub communication_method: String,
    /// Template category or purpose.
    pub r#type: String,
}

/// A voter enrollment or eligibility application for an election event.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Application {
    /// Unique application identifier.
    pub id: String,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub updated_at: Option<DateTime<Local>>,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Geographic area the applicant belongs to, when applicable.
    pub area_id: Option<String>,
    /// Applicant user identifier.
    pub applicant_id: String,
    /// Submitted form data as JSON.
    pub applicant_data: Value,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata.
    pub annotations: Option<Value>,
    /// How the application is verified (e.g. manual, automatic).
    pub verification_type: String,
    /// Current workflow status (e.g. pending, approved, rejected).
    pub status: String,
}

/// Links a geographic area to a contest for ballot scoping and tally grouping.
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub struct AreaContest {
    /// Unique area-contest association identifier.
    pub id: String,
    /// Geographic area identifier.
    pub area_id: String,
    /// Contest identifier.
    pub contest_id: String,
}

/// Published tally results for one contest in one area.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct TallySheet {
    /// Unique tally sheet identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Parent election identifier.
    pub election_id: String,
    /// Contest these results belong to.
    pub contest_id: String,
    /// Geographic area these results cover.
    pub area_id: String,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata.
    pub annotations: Option<Value>,
    /// When the results were published to voters or auditors.
    pub published_at: Option<DateTime<Local>>,
    /// User who published the tally sheet.
    pub published_by_user_id: Option<String>,
    /// Decoded vote counts and candidate results.
    pub content: Option<AreaContestResults>,
    /// Delivery channel these results apply to (online, paper, etc.).
    pub channel: Option<String>,
    /// Soft-delete timestamp, when applicable.
    pub deleted_at: Option<DateTime<Local>>,
    /// User who created the tally sheet record.
    pub created_by_user_id: String,
}

/// Ceremony data for generating keys for an election event.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct KeysCeremony {
    /// Unique ceremony identifier.
    pub id: String,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// List of trustee identifiers.
    pub trustee_ids: Vec<String>,
    /// Value of `KeysCeremonyStatus`
    pub status: Option<Value>,
    /// Value of `KeysCeremonyExecutionStatus`
    pub execution_status: Option<String>,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata.
    pub annotations: Option<Value>,
    /// Threshold for key generation.
    pub threshold: i64,
    /// Name of the ceremony.
    pub name: Option<String>,
    /// Settings for the ceremony.
    pub settings: Option<Value>,
    /// When true, the ceremony is the default for the election event.
    pub is_default: Option<bool>,
    /// Permission labels for the ceremony.
    pub permission_label: Option<Vec<String>>,
}

impl KeysCeremony {
    /// Returns whether this ceremony is the default for its election event.
    ///
    /// Defaults to `true` when the database field is unset.
    pub fn is_default(&self) -> bool {
        self.is_default.clone().unwrap_or(true)
    }

    /// Parses the raw `execution_status` string into a typed enum.
    ///
    /// # Errors
    ///
    /// Returns an error when the stored value is not a valid execution status.
    pub fn execution_status(&self) -> Result<KeysCeremonyExecutionStatus> {
        let execution_status_str =
            self.execution_status.clone().unwrap_or_default();
        KeysCeremonyExecutionStatus::from_str(&execution_status_str)
            .map_err(|err| anyhow!("{:?}", err))
    }

    /// Deserializes the JSON `status` field into a typed ceremony status.
    ///
    /// # Errors
    ///
    /// Returns an error when the JSON cannot be parsed into a ceremony status.
    pub fn status(&self) -> Result<KeysCeremonyStatus> {
        deserialize_value(self.status.clone().unwrap_or_default())
            .map_err(|err| anyhow!("{:?}", err))
    }

    /// Reads the ceremony automation policy from the `settings` JSON.
    ///
    /// Falls back to [`CeremoniesPolicy::MANUAL_CEREMONIES`] when unset or invalid.
    pub fn policy(&self) -> CeremoniesPolicy {
        let settings = self.settings.as_ref().unwrap_or(&Value::Null);
        settings
            .get("policy")
            .and_then(|value: &Value| value.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| CeremoniesPolicy::MANUAL_CEREMONIES.to_string())
            .parse::<CeremoniesPolicy>()
            .unwrap_or(CeremoniesPolicy::MANUAL_CEREMONIES)
    }
}

/// Policy and report options applied when running a tally session.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize, Default)]
pub struct TallySessionConfiguration {
    /// Template used when generating tally report content.
    pub report_content_template_id: Option<String>,
    /// Whether the voter encrypts all contests at once or one at a time.
    pub contest_encryption_policy: Option<ContestEncryptionPolicy>,
    /// Whether decoded (plaintext) ballots appear in tally exports.
    pub decoded_ballots_inclusion_policy: Option<DecodedBallotsInclusionPolicy>,
    /// Whether delegated voting is permitted.
    pub delegated_voting_policy: Option<DelegatedVotingPolicy>,
    /// Whether a consolidated report is produced.
    pub consolidated_report_policy: Option<ConsolidatedReportPolicy>,
}

impl TallySessionConfiguration {
    /// Returns the contest encryption policy, defaulting when unset.
    pub fn get_contest_encryption_policy(&self) -> ContestEncryptionPolicy {
        self.contest_encryption_policy.clone().unwrap_or_default()
    }
    /// Returns the delegated voting policy, defaulting when unset.
    pub fn get_delegated_voting_policy(&self) -> DelegatedVotingPolicy {
        self.delegated_voting_policy.clone().unwrap_or_default()
    }
    /// Returns the decoded-ballots inclusion policy, defaulting when unset.
    pub fn get_decoded_ballots_policy(&self) -> DecodedBallotsInclusionPolicy {
        self.decoded_ballots_inclusion_policy
            .clone()
            .unwrap_or_default()
    }
    /// Returns the consolidated report policy, defaulting when unset.
    pub fn get_consolidated_report_policy(&self) -> ConsolidatedReportPolicy {
        self.consolidated_report_policy.clone().unwrap_or_default()
    }
}

/// A configured tally run that decrypts and counts ballots for selected elections.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct TallySession {
    /// Unique tally session identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata.
    pub annotations: Option<Value>,
    /// Elections included in this tally session.
    pub election_ids: Option<Vec<String>>,
    /// Geographic areas included in this tally session.
    pub area_ids: Option<Vec<String>>,
    /// When true, the tally pipeline has finished successfully.
    pub is_execution_completed: bool,
    /// Key ceremony whose private key shares are used to decrypt ballots.
    pub keys_ceremony_id: String,
    /// Current pipeline execution status as a string enum value.
    pub execution_status: Option<String>,
    /// Minimum number of trustees required to reconstruct the decryption key.
    pub threshold: i64,
    /// Tally policies and report settings for this session.
    pub configuration: Option<TallySessionConfiguration>,
    /// Tally algorithm or mode identifier.
    pub tally_type: Option<String>,
    /// Keycloak permission labels gating access to this session.
    pub permission_label: Option<Vec<String>>,
}
/// Ballot-count statistics stored on a tally-session contest record.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct TallySessionContestAnnotations {
    /// Number of voters eligible to vote in this contest and area.
    pub elegible_voters: u64,
    /// Ballots cast without an associated voter record.
    pub ballots_without_voter: u64,
    /// Total ballots cast for this contest and area.
    pub casted_ballots: u64,
}

/// Links one contest in one area to a parent tally session.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct TallySessionContest {
    /// Unique tally-session-contest identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Geographic area this tally row covers.
    pub area_id: String,
    /// Contest being tallied, when scoped to a single contest.
    pub contest_id: Option<String>,
    /// Mixnet session index within the tally pipeline.
    pub session_id: i32,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata (typically [`TallySessionContestAnnotations`]).
    pub annotations: Option<Value>,
    /// Parent tally session identifier.
    pub tally_session_id: String,
    /// Parent election identifier.
    pub election_id: String,
}

/// Tracks in-progress execution of a tally session through the mixnet pipeline.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct TallySessionExecution {
    /// Unique execution record identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata.
    pub annotations: Option<Value>,
    /// Index of the last processed mixnet message.
    pub current_message_id: i32,
    /// Parent tally session identifier.
    pub tally_session_id: String,
    /// Mixnet session indices participating in this execution.
    pub session_ids: Option<Vec<i32>>,
    /// Pipeline status as JSON.
    pub status: Option<Value>,
    /// Election event that receives published tally results.
    pub results_event_id: Option<String>,
    /// Generated tally documents (reports, exports) as JSON.
    pub documents: Option<Value>,
}

/// Record of a background task run (export, notification, tally step, etc.).
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct TasksExecution {
    /// Unique task execution identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier, when event-scoped.
    pub election_event_id: Option<String>,
    /// Human-readable task name.
    pub name: String,
    /// Task category identifier (matches the worker task type).
    pub task_type: String,
    /// Current execution status (see [`crate::types::hasura::extra::TasksExecutionStatus`]).
    pub execution_status: String,
    /// Record creation timestamp.
    pub created_at: DateTime<Local>,
    /// When the worker started processing the task.
    pub start_at: Option<DateTime<Local>>,
    /// When the worker finished or failed the task.
    pub end_at: Option<DateTime<Local>>,
    /// Metadata.
    pub annotations: Option<Value>,
    /// Labels
    pub labels: Option<Value>,
    /// Worker log output as JSON.
    pub logs: Option<Value>,
    /// User who triggered the task.
    pub executed_by_user: String,
}

/// A trustee who holds a share of the election decryption key.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Trustee {
    /// Unique trustee identifier.
    pub id: String,
    /// Trustee public key used during the key ceremony.
    pub public_key: Option<String>,
    /// Display name.
    pub name: Option<String>,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata.
    pub annotations: Option<Value>,
    /// Owning tenant identifier.
    pub tenant_id: String,
}

/// A tenant (organization) in the multi-tenant platform.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    /// Unique tenant identifier.
    pub id: String,
    /// URL-safe tenant slug used in portal paths.
    pub slug: String,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub updated_at: Option<DateTime<Local>>,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata.
    pub annotations: Option<Value>,
    /// When false, the tenant is disabled and its portals are inaccessible.
    pub is_active: bool,
    /// Default delivery channels enabled for new election events.
    pub voting_channels: Option<Value>,
    /// Tenant-wide configuration as JSON.
    pub settings: Option<Value>,
    /// Internal test flag.
    pub test: Option<i32>,
}
