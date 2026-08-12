// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

pub const EXTERNAL_CONFIG_FILE_NAME: &str = "external_config.json";

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
    /// Governs how each generated voter's plaintext password is chosen. Defaults to
    /// `Fixed` (every voter gets `voter_password` verbatim) so existing
    /// `external_config.json` files - which predate this field - keep working unchanged.
    #[serde(default)]
    pub voter_password_policy: VoterPasswordPolicy,
    pub password_salt: String,
    pub hashed_password: String,
    pub overseas_reference: String,
    pub min_age: i64,
    pub max_age: i64,
    pub authorized_elections_count: i64,
    pub email_verified: bool,
}

/// Policy for the plaintext password `generate-voters` assigns to each generated voter.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum VoterPasswordPolicy {
    /// Every voter gets the literal `voter_password` value.
    #[default]
    Fixed,
    /// Every voter gets its own random numeric password of `digits` digits (e.g. a
    /// 16-digit PIN), independent of `voter_password`.
    RandomNumeric { digits: u32 },
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

pub fn load_external_config(
    working_dir: &str,
) -> Result<ExternalConfigData, Box<dyn Error>> {
    let config_path =
        PathBuf::from(working_dir).join(EXTERNAL_CONFIG_FILE_NAME);
    let file = File::open(config_path)?;
    let reader = BufReader::new(file);
    let config = serde_json::from_reader(reader)?;
    Ok(config)
}
