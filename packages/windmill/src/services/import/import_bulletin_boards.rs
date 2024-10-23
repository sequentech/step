// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::export::export_bulletin_boards::*;
use crate::services::{
    import::import_users::HEADER_RE,
    protocol_manager::{get_election_board, get_event_board},
};
use anyhow::{anyhow, Result};
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

#[instrument(err)]
pub fn import_bulletin_boards(
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
    let headers_vec = headers.iter().map(String::from).collect::<Vec<String>>();
    for result in rdr.records() {
        let record = match result {
            Ok(record) => record,
            Err(err) => {
                return Err(anyhow!("Error reading CSV record: {err}"));
            }
        };
        let (election_id, board_record) = get_board_record(record)?;
        let new_election_id = replacement_map
            .get(&election_id)
            .ok_or(anyhow!("Can't find election id in replacement map"))?
            .clone();
    }
    /*let board = get_election_board(tenant_id, &election_id);

    get_event_board(tenant_id, &election_event_id);*/

    Ok(())
}
