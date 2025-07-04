// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use serde_json::value::Value;
use std::str::FromStr;

use crate::{
    ballot::ContestEncryptionPolicy,
    serialization::deserialize_with_path::deserialize_value,
    types::{
        ceremonies::{KeysCeremonyExecutionStatus, KeysCeremonyStatus},
        tally_sheets::AreaContestResults,
    },
};

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

impl BallotStyle {
    pub fn new(
        id: String,
        tenant_id: String,
        election_id: String,
        area_id: Option<String>,
        created_at: Option<DateTime<Local>>,
        last_updated_at: Option<DateTime<Local>>,
        labels: Option<Value>,
        annotations: Option<Value>,
        ballot_eml: Option<String>,
        ballot_signature: Option<Vec<u8>>,
        status: Option<String>,
        election_event_id: String,
        deleted_at: Option<DateTime<Local>>,
        ballot_publication_id: String,
    ) -> Self {
        BallotStyle {
            id,
            tenant_id,
            election_id,
            area_id,
            created_at,
            last_updated_at,
            labels,
            annotations,
            ballot_eml,
            ballot_signature,
            status,
            election_event_id,
            deleted_at,
            ballot_publication_id,
        }
    }
}

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
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ElectionEvent {
    pub id: String,
    pub created_at: Option<DateTime<Local>>,
    pub updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub tenant_id: String,
    pub name: String,
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
    pub alias: Option<String>,
    pub statistics: Option<Value>,
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Election {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub created_at: Option<DateTime<Local>>,
    pub last_updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub name: String,
    pub description: Option<String>,
    pub presentation: Option<Value>,
    pub status: Option<Value>,
    pub eml: Option<String>,
    pub num_allowed_revotes: Option<i64>,
    pub is_consolidated_ballot_encoding: Option<bool>,
    pub spoil_ballot_option: Option<bool>,
    pub is_kiosk: Option<bool>,
    pub alias: Option<String>,
    pub voting_channels: Option<Value>,
    pub image_document_id: Option<String>,
    pub statistics: Option<Value>,
    pub receipts: Option<Value>,
    pub permission_label: Option<String>,
    pub initialization_report_generated: Option<bool>,
    pub keys_ceremony_id: Option<String>,
}

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
    pub name: Option<String>,
    pub alias: Option<String>,
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
}

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
    pub name: Option<String>,
    pub alias: Option<String>,
    pub description: Option<String>,
    pub r#type: Option<String>,
    pub presentation: Option<Value>,
    pub is_public: Option<bool>,
    pub image_document_id: Option<String>,
}

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

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct VotingChannels {
    pub online: Option<bool>,
    pub kiosk: Option<bool>,
    pub telephone: Option<bool>,
    pub paper: Option<bool>,
}

impl Default for VotingChannels {
    fn default() -> Self {
        Self {
            online: Some(true),
            kiosk: None,
            telephone: None,
            paper: None,
        }
    }
}

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

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub struct AreaContest {
    pub id: String,
    pub area_id: String,
    pub contest_id: String,
}

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
    pub fn is_default(&self) -> bool {
        self.is_default.clone().unwrap_or(true)
    }

    pub fn execution_status(&self) -> Result<KeysCeremonyExecutionStatus> {
        let execution_status_str =
            self.execution_status.clone().unwrap_or_default();
        KeysCeremonyExecutionStatus::from_str(&execution_status_str)
            .map_err(|err| anyhow!("{:?}", err))
    }

    pub fn status(&self) -> Result<KeysCeremonyStatus> {
        deserialize_value(self.status.clone().unwrap_or_default())
            .map_err(|err| anyhow!("{:?}", err))
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize, Default)]
pub struct TallySessionConfiguration {
    pub report_content_template_id: Option<String>,
    pub contest_encryption_policy: Option<ContestEncryptionPolicy>,
}

impl TallySessionConfiguration {
    pub fn get_contest_encryption_policy(&self) -> ContestEncryptionPolicy {
        self.contest_encryption_policy.clone().unwrap_or_default()
    }
}

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
