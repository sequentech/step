// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
pub struct ConfigData {
    pub endpoint_url: String,
    pub tenant_id: String,
    pub keycloak_url: String,
    pub auth_token: String,
    pub refresh_token: String,
    pub client_id: String,
    pub client_secret: String,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExternalConfigData {
    pub election_event_json_file: String,
    pub realm_name: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub area_id: String,
    pub election_id: String,
    pub generate_voters: GenerateVoters,
    pub duplicate_votes: DuplicateVotes,
    pub generate_applications: GenerateApplications,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateVoters {
    pub csv_file_name: String,
    pub fields: Vec<String>,
    pub excluded_columns: Vec<String>,
    pub email_prefix: String,
    pub domain: String,
    pub sequence_email_number: bool,
    pub sequence_start_number: i64,
    pub voter_password: String,
    pub password_salt: String,
    pub hashed_password: String,
    pub overseas_reference: String,
    pub min_age: i64,
    pub max_age: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DuplicateVotes {
    pub row_id_to_clone: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateApplications {
    pub applicant_data: HashMap<String, Value>,
    pub annotations: HashMap<String, Value>,
}
