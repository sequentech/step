// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use chrono::naive::NaiveDateTime;
use serde_json::value::Value;

pub type UUID = String;
pub type uuid = String;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ElectionEvent {
    pub id: UUID,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub tenant_id: UUID,
    pub name: String,
    pub description: Option<String>,
    pub presentation: Option<Value>,
    pub bulletin_board_reference: Option<Value>,
    pub is_archived: bool,
    pub voting_channels: Option<Value>,
    pub dates: Option<Value>,
    pub status: Option<Value>,
    pub user_boards: Option<String>,
    pub encryption_protocol: String,
    pub is_audit: Option<bool>,
    pub audit_election_event_id: Option<UUID>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Election {
    pub id: UUID,
    pub tenant_id: UUID,
    pub election_event_id: UUID,
    pub created_at: Option<NaiveDateTime>,
    pub last_updated_at: Option<NaiveDateTime>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub name: String,
    pub description: Option<String>,
    pub presentation: Option<Value>,
    pub dates: Option<Value>,
    pub status: Option<Value>,
    pub eml: Option<String>,
    pub num_allowed_revotes: Option<i64>,
    pub is_consolidated_ballot_encoding: Option<bool>,
    pub spoil_ballot_option: Option<bool>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Contest {
    pub id: UUID,
    pub tenant_id: UUID,
    pub election_event_id: UUID,
    pub election_id: UUID,
    pub created_at: Option<NaiveDateTime>,
    pub last_updated_at: Option<NaiveDateTime>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub is_acclaimed: Option<bool>,
    pub is_active: Option<bool>,
    pub name: String,
    pub description: Option<String>,
    pub presentation: Option<Value>,
    pub min_votes: Option<i64>,
    pub max_votes: Option<i64>,
    pub voting_type: Option<String>,
    pub counting_algorithm: Option<String>,
    pub is_encrypted: Option<bool>,
    pub tally_configuration: Option<Value>,
    pub conditions: Option<Value>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Candidate {
    pub id: UUID,
    pub tenant_id: UUID,
    pub election_event_id: UUID,
    pub contest_id: UUID,
    pub created_at: Option<NaiveDateTime>,
    pub last_updated_at: Option<NaiveDateTime>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub name: String,
    pub description: Option<String>,
    pub r#type: Option<String>,
    pub presentation: Option<Value>,
    pub is_public: Option<bool>,
}