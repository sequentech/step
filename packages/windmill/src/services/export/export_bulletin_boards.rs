// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
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
    pub static ref HEADER_RE: Regex = Regex::new(r"^[a-zA-Z0-9._-]+$").unwrap();
    pub static ref ELECTION_ID_COL_NAME: String = String::from("election_id");
    pub static ref ID_COL_NAME: String = String::from("id");
    pub static ref CREATED_COL_NAME: String = "created".to_string();
    pub static ref SENDER_PK_COL_NAME: String = "sender_pk".to_string();
    pub static ref STATEMENT_TIMESTAMP_COL_NAME: String = "statement_timestamp".to_string();
    pub static ref STATEMENT_COL_NAME: String = "statement_kind".to_string();
    pub static ref BATCH_COL_NAME: String = "batch".to_string();
    pub static ref MIX_NUMBER_COL_NAME: String = "mix_number".to_string();
    pub static ref MESSAGE_COL_NAME: String = "message".to_string();
    pub static ref VERSION_COL_NAME: String = "version".to_string();
    pub static ref TRUSTEE_NAME_COL_NAME: String = "trustee".to_string();
    pub static ref TRUSTEE_CONFIG_COL_NAME: String = "config".to_string();
}

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
        return Err(anyhow!("File too large: {} > {}", size, get_max_upload_size()?).into());
    }

    Ok(temp_path)
}

#[instrument(err, skip(transaction))]
pub async fn read_election_event_boards(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<TempPath> {
    let keys_ceremonies = get_keys_ceremonies(transaction, tenant_id, election_event_id).await?;
    let b3_client = get_b3_pgsql_client().await?;
    let mut boards_map: HashMap<String, Vec<B3MessageRow>> = HashMap::new();

    // event board
    {
        let board_name = get_event_board(tenant_id, election_event_id);

        let b3_messages = b3_client.get_messages(&board_name, -1).await?;
        boards_map.insert("".to_string(), b3_messages);
    }

    // elections
    let elections = get_elections(transaction, tenant_id, election_event_id, None).await?;
    for election in elections {
        let board_name = get_election_board(tenant_id, &election.id);
        let b3_messages = b3_client.get_messages(&board_name, -1).await?;
        boards_map.insert(election.id.clone(), b3_messages);
    }

    create_boards_csv(boards_map).await
}

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

    // first the event board
    {
        let board_name = get_event_board(tenant_id, election_event_id);
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
    let elections = get_elections(transaction, tenant_id, election_event_id, None).await?;

    for election in elections {
        let board_name = get_election_board(tenant_id, &election.id);
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
        return Err(anyhow!("File too large: {} > {}", size, get_max_upload_size()?).into());
    }

    Ok(temp_path)
}
