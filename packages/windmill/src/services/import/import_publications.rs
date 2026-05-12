// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! Imports ballot publications and the election-event config file into the public bucket.

use anyhow::{anyhow, Context, Result};
use csv::StringRecord;
use deadpool_postgres::Transaction;
use regex::Regex;
use sequent_core::services::date::ISO8601;
use sequent_core::types::hasura::core::BallotPublication;
use sequent_core::{ballot::BallotStyle, serialization::deserialize_with_path::deserialize_str};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use tempfile::NamedTempFile;
use tracing::{info, instrument};
use uuid::Uuid;

use crate::services::ballot_styles::ballot_style::{ElectionEventConfig, EVENT_CONFIG_FILE_NAME};
use crate::services::documents::upload_and_return_public_event_document;

/// JSON shape holding a ballot publication id and its serialized ballot styles.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ballot_design {
    /// Ballot publication id referenced by the styles in this entry.
    ballot_publication_id: String,
    /// Ballot styles belonging to the publication.
    ballot_styles: Vec<BallotStyle>,
}

/// Imports ballot publications from a JSON file into the database.
///
/// # Errors
///
/// Returns an error if the input cannot be read or deserialized.
#[instrument(err, skip(replacement_map))]
#[allow(clippy::implicit_hasher)]
pub async fn import_ballot_publications(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    temp_file: NamedTempFile,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    let mut file = File::open(temp_file)?;
    let mut data_str = String::new();
    file.read_to_string(&mut data_str)?;
    let original_data: Vec<ballot_design> = deserialize_str(&data_str)?;

    //TODO: implement import
    Ok(())
}

/// Imports the election event config file,
/// This file contains the election event presentation and is created during publication.
///
/// # Errors
///
/// Returns an error if the input cannot be read/deserialized,
/// the temp file cannot be written, or upload fails.
#[instrument(err, skip(replacement_map))]
#[allow(clippy::implicit_hasher)]
pub async fn import_election_event_config_file(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    temp_file: NamedTempFile,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    let mut file = File::open(temp_file)?;
    let mut data_str = String::new();
    file.read_to_string(&mut data_str)?;
    let original_data: ElectionEventConfig = deserialize_str(&data_str)?;

    let new_id = Uuid::new_v4();

    let new_election_event_config = ElectionEventConfig {
        id: new_id.to_string(),
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_event_presentation: original_data.election_event_presentation,
    };

    let config_json = serde_json::to_string(&new_election_event_config)?;

    // Write to temp file
    let mut written_temp = NamedTempFile::new()?;
    written_temp.write_all(config_json.as_bytes())?;

    let temp_file_path = written_temp.path().to_string_lossy().to_string();
    let file_size = config_json.len() as u64;

    // Upload to S3 public bucket with election_event_id in path
    let _document = upload_and_return_public_event_document(
        hasura_transaction,
        &temp_file_path,
        file_size,
        "application/json",
        tenant_id,
        election_event_id,
        EVENT_CONFIG_FILE_NAME,
        Some(new_id.to_string()),
    )
    .await?;

    Ok(())
}
