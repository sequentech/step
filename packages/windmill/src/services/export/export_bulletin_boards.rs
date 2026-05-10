// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! CSV exports of ImmuDB/bulletin-board message rows and protocol-manager secrets for an election event.
use crate::postgres::election::get_elections;
use crate::postgres::keys_ceremony::get_keys_ceremonies;
use crate::postgres::trustee::get_all_trustees;
use crate::services::protocol_manager::{
    get_election_board, get_event_board, get_protocol_manager_secret_path,
};
use crate::services::vault;
use crate::services::{
    ceremonies::keys_ceremony::get_keys_ceremony_board, protocol_manager::get_b3_pgsql_client,
};
use anyhow::{anyhow, Context, Result};
use b3::client::pgsql::B3MessageRow;
use base64::engine::general_purpose;
use base64::Engine;
use deadpool_postgres::{Client as DbClient, Transaction};
use futures::future::try_join_all;
use regex::Regex;
use sequent_core::util::aws::get_max_upload_size;
use sequent_core::util::temp_path::generate_temp_file;
use std::collections::HashMap;
use tempfile::{NamedTempFile, TempPath};
use tracing::{event, info, instrument, Level};

lazy_static! {
    /// Validates bulletin-board CSV column names (alphanumeric, dot, underscore, hyphen).
    pub static ref HEADER_RE: Regex = Regex::new(r"^[a-zA-Z0-9._-]+$").unwrap();
    /// CSV header: owning election id (empty string for the event-level board).
    pub static ref ELECTION_ID_COL_NAME: String = String::from("election_id");
    /// CSV header: message row id.
    pub static ref ID_COL_NAME: String = String::from("id");
    /// CSV header: row creation timestamp.
    pub static ref CREATED_COL_NAME: String = "created".to_string();
    /// CSV header: sender public key.
    pub static ref SENDER_PK_COL_NAME: String = "sender_pk".to_string();
    /// CSV header: statement timestamp.
    pub static ref STATEMENT_TIMESTAMP_COL_NAME: String = "statement_timestamp".to_string();
    /// CSV header: statement kind discriminator.
    pub static ref STATEMENT_COL_NAME: String = "statement_kind".to_string();
    /// CSV header: batch index.
    pub static ref BATCH_COL_NAME: String = "batch".to_string();
    /// CSV header: mix round number.
    pub static ref MIX_NUMBER_COL_NAME: String = "mix_number".to_string();
    /// CSV header: base64-encoded payload.
    pub static ref MESSAGE_COL_NAME: String = "message".to_string();
    /// CSV header: row schema/version tag.
    pub static ref VERSION_COL_NAME: String = "version".to_string();
    /// CSV header used in trustee config exports (trustee display name).
    pub static ref TRUSTEE_NAME_COL_NAME: String = "trustee".to_string();
    /// CSV header for trustee-side configuration blob.
    pub static ref TRUSTEE_CONFIG_COL_NAME: String = "config".to_string();
}

/// Converts a single B3 bulletin-board row into a CSV record (message is standard base64, no padding).
#[instrument]
fn get_board_record(election_id: &str, row: B3MessageRow) -> Vec<String> {
    let message_b64 = general_purpose::STANDARD_NO_PAD.encode(row.message.clone());
    vec![
        election_id.to_string(),
        row.id.to_string(),
        row.created.to_string(),
        row.sender_pk.to_string(),
        row.statement_timestamp.to_string(),
        row.statement_kind.clone(),
        row.batch.to_string(),
        row.mix_number.to_string(),
        message_b64,
        row.version.clone(),
    ]
}

/// Writes all boards in `boards_map` to a comma-separated CSV temp file.
///
/// # Errors
///
/// Returns an error when temp file creation, CSV writes, or size checks fail.
#[instrument(err)]
async fn create_boards_csv(boards_map: HashMap<String, Vec<B3MessageRow>>) -> Result<TempPath> {
    let mut writer = csv::WriterBuilder::new().delimiter(b',').from_writer(
        generate_temp_file("export-boards-", ".csv")
            .with_context(|| "Error creating temporary file")?,
    );
    let headers: Vec<String> = vec![
        ELECTION_ID_COL_NAME.to_string(),
        ID_COL_NAME.to_string(),
        CREATED_COL_NAME.to_string(),
        SENDER_PK_COL_NAME.to_string(),
        STATEMENT_TIMESTAMP_COL_NAME.to_string(),
        STATEMENT_COL_NAME.to_string(),
        BATCH_COL_NAME.to_string(),
        MIX_NUMBER_COL_NAME.to_string(),
        MESSAGE_COL_NAME.to_string(),
        VERSION_COL_NAME.to_string(),
    ];
    writer.write_record(&headers)?;
    for (board_name, board_rows) in boards_map {
        for board_row in board_rows {
            let record = get_board_record(&board_name, board_row);
            writer
                .write_record(&record)
                .with_context(|| "Error writing record")?;
        }
    }
    writer
        .flush()
        .with_context(|| "Error flushing CSV writer")?;

    let temp_path = writer
        .into_inner()
        .with_context(|| "Error getting inner writer")?
        .into_temp_path();

    let size = temp_path.metadata()?.len();
    if size > get_max_upload_size()? as u64 {
        return Err(anyhow!(
            "File too large: {} > {}",
            size,
            get_max_upload_size()?
        ));
    }

    Ok(temp_path)
}

/// Fetches every bulletin board for the event and creates a temporary file.
///
/// # Errors
///
/// Propagates missing `ENV_SLUG`, database errors, B3 client failures, or [`create_boards_csv`] errors.
#[instrument(err, skip(transaction))]
pub async fn read_election_event_boards(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<TempPath> {
    let keys_ceremonies = get_keys_ceremonies(transaction, tenant_id, election_event_id).await?;
    let b3_client = get_b3_pgsql_client().await?;
    let mut boards_map: HashMap<String, Vec<B3MessageRow>> = HashMap::new();
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;

    // event board
    {
        let board_name = get_event_board(tenant_id, election_event_id, &slug);

        let b3_messages = b3_client.get_messages(&board_name, -1).await?;
        boards_map.insert("".to_string(), b3_messages);
    }

    // elections
    let elections = get_elections(transaction, tenant_id, election_event_id).await?;
    for election in elections {
        let board_name = get_election_board(tenant_id, &election.id, &slug);
        let b3_messages = b3_client.get_messages(&board_name, -1).await?;
        boards_map.insert(election.id.clone(), b3_messages);
    }

    create_boards_csv(boards_map).await
}

/// Exports protocol-manager shared secrets for the event board and each
/// election board as two-column CSV (`election_id`, `key`).
///
/// # Errors
///
/// Propagates vault read failures, missing secrets, oversize output, or database/env errors.
#[instrument(err, skip(transaction))]
pub async fn read_protocol_manager_keys(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<TempPath> {
    let mut writer = csv::WriterBuilder::new().delimiter(b',').from_writer(
        generate_temp_file("export-protocol-keys-", ".csv")
            .with_context(|| "Error creating temporary file")?,
    );
    let headers = vec!["election_id".to_string(), "key".to_string()];
    writer.write_record(&headers)?;
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;

    // first the event board
    {
        let board_name = get_event_board(tenant_id, election_event_id, &slug);
        let protocol_manager_key = get_protocol_manager_secret_path(&board_name);
        let protocol_manager_data = vault::read_secret(
            transaction,
            tenant_id,
            Some(election_event_id),
            &protocol_manager_key,
        )
        .await?
        .ok_or(anyhow!("protocol manager secret not found"))?;
        let record = vec!["".into(), protocol_manager_data];
        writer
            .write_record(&record)
            .with_context(|| "Error writing record")?;
    }

    // now loop over all elections
    let elections = get_elections(transaction, tenant_id, election_event_id).await?;

    for election in elections {
        let board_name = get_election_board(tenant_id, &election.id, &slug);
        let protocol_manager_key = get_protocol_manager_secret_path(&board_name);
        let protocol_manager_data = vault::read_secret(
            transaction,
            tenant_id,
            Some(election_event_id),
            &protocol_manager_key,
        )
        .await?
        .ok_or(anyhow!("protocol manager secret not found"))?;
        let record = vec![election.id.clone(), protocol_manager_data];
        writer
            .write_record(&record)
            .with_context(|| "Error writing record")?;
    }
    writer
        .flush()
        .with_context(|| "Error flushing CSV writer")?;

    let temp_path = writer
        .into_inner()
        .with_context(|| "Error getting inner writer")?
        .into_temp_path();

    let size = temp_path.metadata()?.len();
    if size > get_max_upload_size()? as u64 {
        return Err(anyhow!(
            "File too large: {} > {}",
            size,
            get_max_upload_size()?
        ));
    }

    Ok(temp_path)
}
