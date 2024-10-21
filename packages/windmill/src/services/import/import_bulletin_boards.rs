// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::import::import_users::HEADER_RE;
use anyhow::{anyhow, Result};
use csv::StringRecord;
use std::collections::HashMap;
use tempfile::NamedTempFile;
use tracing::{info, instrument};

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

    // Obtain statements
    /*
    let (
        voters_table,
        create_table_query,
        copy_from_query,
        voters_table_input_columns_names,
        voters_table_processed_columns_names,
        voters_table_processed_columns_types,
    ) = match get_copy_from_query(&headers) {
        Ok(result) => result,
        Err(err) => {
            return Err(anyhow!(
                "Error obtaining copy_from query: {err}"
            ));
        }
    };*/
    Ok(())
}
