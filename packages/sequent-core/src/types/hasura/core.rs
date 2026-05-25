// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

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

/// Election event preview url and metadata.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Preview {
    pub id: String,
    pub tenant_id: String,
    pub document_id: String,
    pub url: String,
    pub requested_by: String,
    pub created_at: Option<DateTime<Local>>,
    pub updated_at: Option<DateTime<Local>>,
    pub annotations: Option<Value>,
}

/// Ballot publication metadata.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct BallotPublication {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub created_at: Option<DateTime<Local>>,
    pub deleted_at: Option<DateTime<Local>>,
    pub created_by_user_id: Option<String>,
    pub is_generated: Option<bool>,
    pub election_ids: Option<Vec<String>>,
    pub published_at: Option<DateTime<Local>>,
    pub election_id: Option<String>,
}

/// Ballot style metadata.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct BallotStyle {
    pub id: String,
    pub tenant_id: String,
    pub election_id: String,
    pub area_id: Option<String>,
    pub created_at: Option<DateTime<Local>>,
    pub last_updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub ballot_eml: Option<String>,
    pub ballot_signature: Option<Vec<u8>>,
    pub status: Option<String>,
    pub election_event_id: String,
    pub deleted_at: Option<DateTime<Local>>,
    pub ballot_publication_id: String,
}

/// Electoral area or district.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Area {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub created_at: Option<DateTime<Local>>,
    pub last_updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub r#type: Option<String>,
    pub parent_id: Option<String>,
    pub presentation: Option<Value>,
}

/// Election event metadata.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ElectionEvent {
    pub id: String,
    pub created_at: Option<DateTime<Local>>,
    pub updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub tenant_id: String,
    pub description: Option<String>,
    pub presentation: Option<Value>,
    pub bulletin_board_reference: Option<Value>,
    pub is_archived: bool,
    pub voting_channels: Option<Value>,
    pub status: Option<Value>,
    pub user_boards: Option<String>,
    pub encryption_protocol: String,
    pub is_audit: Option<bool>,
    pub audit_election_event_id: Option<String>,
    pub public_key: Option<String>,
    pub statistics: Option<Value>,
    pub external_id: Option<String>,
}

/// Election within an event.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Election {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub created_at: Option<DateTime<Local>>,
    pub last_updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub description: Option<String>,
    pub presentation: Option<Value>,
    pub status: Option<Value>,
    pub eml: Option<String>,
    pub external_id: Option<String>,
    pub num_allowed_revotes: Option<i64>,
    pub is_consolidated_ballot_encoding: Option<bool>,
    pub spoil_ballot_option: Option<bool>,
    pub is_kiosk: Option<bool>,
    pub voting_channels: Option<Value>,
    pub image_document_id: Option<String>,
    pub statistics: Option<Value>,
    pub receipts: Option<Value>,
    pub permission_label: Option<String>,
    pub initialization_report_generated: Option<bool>,
    pub keys_ceremony_id: Option<String>,
}

/// Contest within an election.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Contest {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
    pub created_at: Option<DateTime<Local>>,
    pub last_updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub is_acclaimed: Option<bool>,
    pub is_active: Option<bool>,
    pub description: Option<String>,
    pub presentation: Option<Value>,
    pub min_votes: Option<i64>,
    pub max_votes: Option<i64>,
    pub winning_candidates_num: Option<i64>,
    pub voting_type: Option<String>,
    pub counting_algorithm: Option<String>,
    pub is_encrypted: Option<bool>,
    pub tally_configuration: Option<Value>,
    pub image_document_id: Option<String>,
    pub conditions: Option<Value>,
    pub external_id: Option<String>,
}

/// Candidate standing in a contest.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub contest_id: Option<String>,
    pub created_at: Option<DateTime<Local>>,
    pub last_updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub description: Option<String>,
    pub r#type: Option<String>,
    pub presentation: Option<Value>,
    pub is_public: Option<bool>,
    pub image_document_id: Option<String>,
    pub external_id: Option<String>,
}

/// Stored document metadata.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub tenant_id: Option<String>,
    pub election_event_id: Option<String>,
    pub name: Option<String>,
    pub media_type: Option<String>,
    pub size: Option<i64>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub created_at: Option<DateTime<Local>>,
    pub last_updated_at: Option<DateTime<Local>>,
    pub is_public: Option<bool>,
}

/// Support material attached to an event.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct SupportMaterial {
    pub id: String,
    pub created_at: DateTime<Local>,
    pub last_updated_at: DateTime<Local>,
    pub kind: String,
    pub data: Value,
    pub tenant_id: String,
    pub election_event_id: String,
    pub labels: Value,
    pub annotations: Value,
    pub document_id: Option<String>,
    pub is_hidden: Option<bool>,
}

/// Store if voting is enabled in each channel.
#[allow(missing_docs)]
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct VotingChannels {
    pub online: Option<bool>,
    pub kiosk: Option<bool>,
    pub telephone: Option<bool>,
    pub paper: Option<bool>,
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

/// Minimal metadata for an election.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ElectionType {
    pub id: String,
    pub tenant_id: Option<String>,
    pub name: Option<String>,
    pub created_at: Option<DateTime<Local>>,
    pub updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
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

/// Template for generated reports or communications.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub alias: String,
    pub tenant_id: String,
    pub template: Value,
    pub created_by: String,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub created_at: Option<DateTime<Local>>,
    pub updated_at: Option<DateTime<Local>>,
    pub communication_method: String,
    pub r#type: String,
}

/// Application submitted by a voter.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Application {
    pub id: String,
    pub created_at: Option<DateTime<Local>>,
    pub updated_at: Option<DateTime<Local>>,
    pub tenant_id: String,
    pub election_event_id: String,
    pub area_id: Option<String>,
    pub applicant_id: String,
    pub applicant_data: Value,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub verification_type: String,
    pub status: String,
}

/// Mapping between area and contest.
#[allow(missing_docs)]
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub struct AreaContest {
    pub id: String,
    pub area_id: String,
    pub contest_id: String,
}

/// Tally sheet contents and metadata.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct TallySheet {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
    pub contest_id: String,
    pub area_id: String,
    pub created_at: Option<DateTime<Local>>,
    pub last_updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub published_at: Option<DateTime<Local>>,
    pub published_by_user_id: Option<String>,
    pub content: Option<AreaContestResults>,
    pub channel: Option<String>,
    pub deleted_at: Option<DateTime<Local>>,
    pub created_by_user_id: String,
}

/// Keys ceremony configuration and state.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct KeysCeremony {
    pub id: String,
    pub created_at: Option<DateTime<Local>>,
    pub last_updated_at: Option<DateTime<Local>>,
    pub tenant_id: String,
    pub election_event_id: String,
    pub trustee_ids: Vec<String>,
    pub status: Option<Value>, // KeysCeremonyStatus
    pub execution_status: Option<String>, // KeysCeremonyExecutionStatus
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub threshold: i64,
    pub name: Option<String>,
    pub settings: Option<Value>,
    pub is_default: Option<bool>,
    pub permission_label: Option<Vec<String>>,
}

impl KeysCeremony {
    /// Returns true if this is the default ceremony.
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.is_default.unwrap_or(true)
    }

    /// Returns the execution status.
    ///
    /// # Errors
    /// Returns an error if the status string cannot be parsed.
    pub fn execution_status(&self) -> Result<KeysCeremonyExecutionStatus> {
        let execution_status_str =
            self.execution_status.as_deref().unwrap_or("");
        KeysCeremonyExecutionStatus::from_str(execution_status_str)
            .map_err(|err| anyhow!("{err:?}"))
    }

    /// # Errors
    /// Returns an error if the status value cannot be deserialized.
    pub fn status(&self) -> Result<KeysCeremonyStatus> {
        deserialize_value(self.status.clone().unwrap_or_default())
            .map_err(|err| anyhow!("{err:?}"))
    }

    /// Returns the ceremonies policy, defaulting to manual ceremonies if not set or invalid.
    #[must_use]
    pub fn policy(&self) -> CeremoniesPolicy {
        let settings = self.settings.as_ref().unwrap_or(&Value::Null);
        settings
            .get("policy")
            .and_then(|value| value.as_str())
            .and_then(|s| s.parse::<CeremoniesPolicy>().ok())
            .unwrap_or(CeremoniesPolicy::MANUAL_CEREMONIES)
    }
}

/// Tally session configuration options.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize, Default)]
pub struct TallySessionConfiguration {
    pub report_content_template_id: Option<String>,
    pub contest_encryption_policy: Option<ContestEncryptionPolicy>,
    pub decoded_ballots_inclusion_policy: Option<DecodedBallotsInclusionPolicy>,
    pub delegated_voting_policy: Option<DelegatedVotingPolicy>,
    pub consolidated_report_policy: Option<ConsolidatedReportPolicy>,
}

impl TallySessionConfiguration {
    /// Returns the contest encryption policy.
    #[must_use]
    pub fn get_contest_encryption_policy(&self) -> ContestEncryptionPolicy {
        self.contest_encryption_policy.clone().unwrap_or_default()
    }
    /// Returns the delegated voting policy.
    #[must_use]
    pub fn get_delegated_voting_policy(&self) -> DelegatedVotingPolicy {
        self.delegated_voting_policy.clone().unwrap_or_default()
    }
    /// Returns the decoded ballots inclusion policy.
    #[must_use]
    pub fn get_decoded_ballots_policy(&self) -> DecodedBallotsInclusionPolicy {
        self.decoded_ballots_inclusion_policy
            .clone()
            .unwrap_or_default()
    }
    /// Returns the consolidated report policy.
    #[must_use]
    pub fn get_consolidated_report_policy(&self) -> ConsolidatedReportPolicy {
        self.consolidated_report_policy.clone().unwrap_or_default()
    }
}

/// Tally session record.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct TallySession {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub created_at: Option<DateTime<Local>>,
    pub last_updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub election_ids: Option<Vec<String>>,
    pub area_ids: Option<Vec<String>>,
    pub is_execution_completed: bool,
    pub keys_ceremony_id: String,
    pub execution_status: Option<String>,
    pub threshold: i64,
    pub configuration: Option<TallySessionConfiguration>,
    pub tally_type: Option<String>,
    pub permission_label: Option<Vec<String>>,
}
/// Aggregate annotations for a session contest.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct TallySessionContestAnnotations {
    pub elegible_voters: u64,
    pub ballots_without_voter: u64,
    pub casted_ballots: u64,
}

/// Contest entry for a tally session.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct TallySessionContest {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub area_id: String,
    pub contest_id: Option<String>,
    pub session_id: i32,
    pub created_at: Option<DateTime<Local>>,
    pub last_updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub tally_session_id: String,
    pub election_id: String,
}

/// Execution details for a tally session.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct TallySessionExecution {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub created_at: Option<DateTime<Local>>,
    pub last_updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub current_message_id: i32,
    pub tally_session_id: String,
    pub session_ids: Option<Vec<i32>>,
    pub status: Option<Value>,
    pub results_event_id: Option<String>,
    pub documents: Option<Value>,
}

/// Task execution record for background tasks.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct TasksExecution {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: Option<String>,
    pub name: String,
    pub task_type: String,
    pub execution_status: String,
    pub created_at: DateTime<Local>,
    pub start_at: Option<DateTime<Local>>,
    pub end_at: Option<DateTime<Local>>,
    pub annotations: Option<Value>,
    pub labels: Option<Value>,
    pub logs: Option<Value>,
    pub executed_by_user: String,
}

/// Trustee (key holder) information.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Trustee {
    pub id: String,
    pub public_key: Option<String>,
    pub name: Option<String>,
    pub created_at: Option<DateTime<Local>>,
    pub last_updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub tenant_id: String,
}

/// Tenant configuration and metadata.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: String,
    pub slug: String,
    pub created_at: Option<DateTime<Local>>,
    pub updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub is_active: bool,
    pub voting_channels: Option<Value>,
    pub settings: Option<Value>,
    pub test: Option<i32>,
}
