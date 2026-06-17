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

/// Filename for the JSON config in a working directory.
pub const EXTERNAL_CONFIG_FILE_NAME: &str = "external_config.json";

/// Top-level configuration for voter and application generation.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExternalConfigData {
    /// Path to the election event JSON export file.
    pub election_event_json_file: String,
    /// Keycloak realm name for the tenant.
    pub realm_name: String,
    /// Tenant identifier.
    pub tenant_id: String,
    /// Election event identifier.
    pub election_event_id: String,
    /// Default area identifier for generated voters.
    pub area_id: String,
    /// Election identifier for vote duplication tests.
    pub election_id: String,
    /// Settings for bulk voter generation from CSV.
    pub generate_voters: GenerateVoters,
    /// Settings for cloning an existing voter row.
    pub duplicate_votes: DuplicateVotes,
    /// Settings for generating voter applications.
    pub generate_applications: GenerateApplications,
}

/// CSV-driven voter generation settings.
#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateVoters {
    /// Source CSV filename relative to the working directory.
    pub csv_file_name: String,
    /// Column names to include when generating voters.
    pub fields: Vec<String>,
    /// CSV columns to skip during import.
    pub excluded_columns: Vec<String>,
    /// Prefix prepended to generated email addresses.
    pub email_prefix: String,
    /// Email domain for generated voters.
    pub domain: String,
    /// Whether to append a sequence number to email addresses.
    pub sequence_email_number: bool,
    /// Starting sequence number when numbering is enabled.
    pub sequence_start_number: i64,
    /// Plain-text password assigned to generated voters.
    pub voter_password: String,
    /// Salt used when hashing voter passwords.
    pub password_salt: String,
    /// Precomputed hashed password, when used instead of plain text.
    pub hashed_password: String,
    /// Reference label for overseas voters in the CSV.
    pub overseas_reference: String,
    /// Minimum voter age filter.
    pub min_age: i64,
    /// Maximum voter age filter.
    pub max_age: i64,
    /// Number of elections each generated voter may access.
    pub authorized_elections_count: i64,
    /// Whether generated voters have verified email addresses.
    pub email_verified: bool,
}

/// Configuration for duplicating an existing voter's ballot row.
#[derive(Debug, Serialize, Deserialize)]
pub struct DuplicateVotes {
    /// CSV row ID whose vote should be cloned.
    pub row_id_to_clone: String,
}

/// Configuration for generating voter registration applications.
#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateApplications {
    /// Applicant field values keyed by attribute name.
    pub applicant_data: HashMap<String, Value>,
    /// Extra annotations attached to generated applications.
    pub annotations: HashMap<String, Value>,
}

/// Loads [`ExternalConfigData`] from `external_config.json` in `working_dir`.
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
