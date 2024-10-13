// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use serde_json::value::Value;

use crate::{
    ballot::{ElectionEventPresentation, ElectionPresentation},
    serialization::deserialize_with_path::deserialize_value,
    types::tally_sheets::AreaContestResults,
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
    pub initializion_report_generated: Option<bool>,
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

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct VotingChannels {
    pub online: Option<bool>,
    pub kiosk: Option<bool>,
    pub telephone: Option<bool>,
    pub paper: Option<bool>,
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
    pub id: String,
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
    pub status: Option<Value>,
    pub execution_status: Option<String>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub threshold: i64,
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct TallySessionConfiguration {
    pub report_content_template_id: Option<String>,
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
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct TallySessionContest {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub area_id: String,
    pub contest_id: String,
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
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct TasksExecution {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
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
