// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::export::export_bulletin_boards::*;
use crate::services::protocol_manager::get_b3_pgsql_client;
use crate::services::protocol_manager::get_protocol_manager_secret_path;
use crate::services::vault;
use crate::services::{
    import::import_users::HEADER_RE,
    protocol_manager::{get_election_board, get_event_board},
};
use anyhow::{anyhow, Context, Result};
use b3::client::pgsql::B3MessageRow;
use base64::engine::general_purpose;
use base64::Engine;
use csv::StringRecord;
use std::collections::HashMap;
use tempfile::NamedTempFile;
use tracing::{info, instrument};

#[instrument]
fn get_board_record(record: StringRecord) -> Result<(String, B3MessageRow)> {
    let fields: Vec<String> = record.iter().map(|val| val.to_string()).collect();

    if fields.len() < 10 {
        return Err(anyhow!(
            "Missing fields, required at least 10 but got {}",
            fields.len()
        ));
    }

    let election_id = fields[0].clone();
    let id = fields[1]
        .clone()
        .parse::<i64>()
        .map_err(|err| anyhow!("{:?}", err))?;
    let created = fields[2]
        .clone()
        .parse::<u64>()
        .map_err(|err| anyhow!("{:?}", err))?;
    let sender_pk = fields[3].clone();
    let statement_timestamp = fields[4]
        .clone()
        .parse::<u64>()
        .map_err(|err| anyhow!("{:?}", err))?;
    let statement_kind = fields[5].clone();
    let batch = fields[6]
        .clone()
        .parse::<i32>()
        .map_err(|err| anyhow!("{:?}", err))?;
    let mix_number = fields[7]
        .parse::<i32>()
        .map_err(|err| anyhow!("{:?}", err))?;
    let message = general_purpose::STANDARD_NO_PAD
        .decode(fields[8].clone())
        .map_err(|err| anyhow!("{:?}", err))?;
    let version = fields[9].clone();

    let row = B3MessageRow {
        id: id,
        created: created,
        // Base64 encoded spki der representation.
        sender_pk: sender_pk,
        statement_timestamp: statement_timestamp,
        statement_kind: statement_kind,
        batch: batch,
        // When signing mixes, specifies which mix in the chain is being signed.
        // This allows creating a unique index for which otherwise there would be duplicate
        // mix signature messages
        mix_number: mix_number,
        message: message,
        version: version,
    };

    Ok((election_id, row))
}

#[instrument]
fn get_board_name_for_event_or_election(
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<String>,
) -> String {
    if let Some(election_id) = election_id.clone() {
        get_election_board(tenant_id, &election_id)
    } else {
        get_event_board(tenant_id, election_event_id)
    }
}

#[instrument(err)]
pub async fn import_protocol_manager_keys(
    tenant_id: &str,
    election_event_id: &str,
    temp_file: NamedTempFile,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    let separator = b',';

    // Read the first line of the file to get the columns
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(separator)
        .from_reader(temp_file);

    let headers = match rdr.headers() {
        Ok(headers) => headers.clone(),
        Err(err) => {
            return Err(anyhow!("Error reading CSV headers from voters file: {err}"));
        }
    };

    // Validate headers
    info!("headers: {headers:?}");
    for header in headers.iter() {
        if !HEADER_RE.is_match(header) {
            return Err(anyhow!(
                "CSV Header contains characters not allowed: {header}"
            ));
        }
    }
    for result in rdr.records() {
        let record = match result {
            Ok(record) => record,
            Err(err) => {
                return Err(anyhow!("Error reading CSV record: {err}"));
            }
        };
        let fields: Vec<String> = record.iter().map(|val| val.to_string()).collect();

        if fields.len() < 2 {
            return Err(anyhow!(
                "Missing fields, required at least 2 but got {}",
                fields.len()
            ));
        }

        let election_id = fields[0].clone();
        let new_election_id = if election_id.trim().len() > 0 {
            Some(
                replacement_map
                    .get(&election_id)
                    .ok_or(anyhow!("Can't find election id in replacement map"))?
                    .clone(),
            )
        } else {
            None
        };

        let value = fields[1].clone();
        let board_name =
            get_board_name_for_event_or_election(tenant_id, election_event_id, new_election_id);
        let protocol_manager_key = get_protocol_manager_secret_path(&board_name);
        vault::save_secret(protocol_manager_key, value)
            .await
            .context("protocol manager secret not saved")?;
    }
    Ok(())
}

#[instrument(err)]
pub async fn import_bulletin_boards(
    tenant_id: &str,
    election_event_id: &str,
    temp_file: NamedTempFile,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    let separator = b',';

    // Read the first line of the file to get the columns
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(separator)
        .from_reader(temp_file);

    let headers = match rdr.headers() {
        Ok(headers) => headers.clone(),
        Err(err) => {
            return Err(anyhow!("Error reading CSV headers from voters file: {err}"));
        }
    };

    // Validate headers
    info!("headers: {headers:?}");
    for header in headers.iter() {
        if !HEADER_RE.is_match(header) {
            return Err(anyhow!(
                "CSV Header contains characters not allowed: {header}"
            ));
        }
    }
    let mut boards_map: HashMap<String, Vec<B3MessageRow>> = HashMap::new();
    for result in rdr.records() {
        let record = match result {
            Ok(record) => record,
            Err(err) => {
                return Err(anyhow!("Error reading CSV record: {err}"));
            }
        };
        let (election_id, board_record) = get_board_record(record)?;

        // Add board_record to the vector in boards_map, indexed by election_id
        boards_map
            .entry(election_id)
            .or_insert_with(Vec::new)
            .push(board_record);
    }

    for (election_id, records) in boards_map {
        let new_election_id = if election_id.trim().len() > 0 {
            Some(
                replacement_map
                    .get(&election_id)
                    .ok_or(anyhow!("Can't find election id in replacement map"))?
                    .clone(),
            )
        } else {
            None
        };
        let board_name =
            get_board_name_for_event_or_election(tenant_id, election_event_id, new_election_id);

        // Sort messages by 'created' in ascending order
        let mut new_records = records.clone();
        new_records.sort_by_key(|msg| msg.created);

        let mut client = get_b3_pgsql_client().await?;
        client.create_board_ine(&board_name).await?;
        client.insert_messages(&board_name, &new_records).await?;
    }

    /*let board = get_election_board(tenant_id, &election_id);

    get_event_board(tenant_id, &election_event_id);*/

    Ok(())
}
