// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::document::get_document;
use crate::services::documents::get_document_as_temp_file;
use crate::services::documents::upload_and_return_document;
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use rusqlite::{types::Type, Connection};
use rust_xlsxwriter::Workbook;
use sequent_core::temp_path::generate_temp_file;
use sequent_core::temp_path::get_file_size;
use std::path::Path;
use tracing::instrument;

#[instrument(err)]
pub async fn export_tally_results_to_xlsx(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    results_sqlite_document_id: String,
    results_event_id: String,
    document_id: String,
) -> Result<()> {
    let sqlite_document = get_document(
        hasura_transaction,
        &tenant_id,
        Some(election_event_id.clone()),
        &results_sqlite_document_id,
    )
    .await
    .map_err(|e| anyhow!("Failed to get document: {}", e))?;

    if sqlite_document.is_none() {
        return Err(anyhow!("Document not found"));
    }

    let sqlite_document = sqlite_document.unwrap();

    let sqlite_file = get_document_as_temp_file(&tenant_id, &sqlite_document)
        .await
        .map_err(|e| anyhow!("Failed to get sqlite document as temp file: {}", e))?;

    let xlsx_file_name = format!("results-{}", results_event_id.clone());
    let xlsx_file = generate_temp_file(&xlsx_file_name, ".xlsx")?;

    convert_db_to_xlsx(&sqlite_file.path(), &xlsx_file.path())
        .await
        .map_err(|e| anyhow!("Failed to convert DB to XLSX: {}", e))?;
    println!("XLSX file created at: {}", xlsx_file.path().display());

    let xlsx_file_path = xlsx_file.into_temp_path();
    println!("XLSX 1");
    let xlsx_file_path_string = xlsx_file_path.to_string_lossy().to_string();
    println!("XLSX 2");
    let xlsx_file_size = get_file_size(xlsx_file_path_string.as_str())
        .map_err(|e| anyhow!("Failed to get XLSX file size: {}", e))?;

    println!("XLSX file size: {}", xlsx_file_size.clone());

    let _ = upload_and_return_document(
        hasura_transaction,
        &xlsx_file_path_string,
        xlsx_file_size,
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        &tenant_id,
        Some(election_event_id),
        &format!("{}.xlsx", xlsx_file_name),
        Some(document_id),
        false,
    )
    .await
    .map_err(|e| anyhow!("Failed to upload XLSX document: {}", e))?;

    Ok(())
}

/// Converts a SQLite database file to an XLSX file, with each table as a worksheet.
async fn convert_db_to_xlsx(db_path: &Path, xlsx_path: &Path) -> Result<()> {
    // Open the SQLite database
    let db_conn = Connection::open(db_path)?;

    // Create a new Excel workbook
    let mut workbook = Workbook::new();

    // Get a list of all tables in the database
    let mut stmt =
        db_conn.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")?;
    let table_names_iter = stmt.query_map([], |row| row.get(0))?;

    println!("Starting conversion to XLSX...");

    // Iterate through each table and create a new worksheet
    for table_result in table_names_iter {
        let table_name: String = table_result?;
        println!("  - Processing table: '{}'", table_name);
        if !table_name.contains("results") {
            println!("    - Skipping table: '{}'", table_name);
            continue;
        }

        let mut worksheet = workbook.add_worksheet();
        worksheet.set_name(&table_name)?;

        // Prepare the statement for the current table
        let mut table_stmt = db_conn.prepare(&format!("SELECT * FROM `{}`", table_name.clone()))?;

        // Get the column names and count before creating the rows iterator
        let column_names: Vec<String> = table_stmt
            .column_names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        let column_count = table_stmt.column_count();

        for (col_index, col_name) in column_names.iter().enumerate() {
            worksheet.write_string(0, col_index as u16, col_name)?;
        }

        // Now, get the rows
        let mut rows = table_stmt.query([])?;

        // Write the data rows
        let mut row_index = 1;
        while let Some(row) = rows.next()? {
            for col_index in 0..column_count {
                let value_ref = row.get_ref(col_index)?;
                // Handle different data types for writing to Excel
                match value_ref.data_type() {
                    Type::Integer => {
                        let num: i64 = value_ref.as_i64()?;
                        worksheet.write_number(row_index, col_index as u16, num as f64)?;
                    }
                    Type::Real => {
                        let num: f64 = value_ref.as_f64()?;
                        worksheet.write_number(row_index, col_index as u16, num)?;
                    }
                    Type::Text => {
                        let text: String = value_ref.as_str()?.to_string();
                        worksheet.write_string(row_index, col_index as u16, &text)?;
                    }
                    _ => {
                        // For other types like Null, Blob, etc., write as a string representation
                        worksheet.write_string(
                            row_index,
                            col_index as u16,
                            &format!("{:?}", value_ref),
                        )?;
                    }
                }
            }
            row_index += 1;
        }
    }

    // Finalize the XLSX file
    workbook.save(xlsx_path)?;
    println!(
        "Conversion successful! XLSX file created at: {}",
        xlsx_path.display()
    );

    Ok(())
}
