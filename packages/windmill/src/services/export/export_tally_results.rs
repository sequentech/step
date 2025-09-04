// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::document::get_document;
use crate::postgres::tally_session_execution::get_last_tally_session_execution;
use crate::postgres::tally_session_execution::update_tally_session_execution_documents;
use crate::services::documents::get_document_as_temp_file;
use crate::services::documents::upload_and_return_document;
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use rusqlite::{types::Type, Connection};
use rust_xlsxwriter::Workbook;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::temp_path::generate_temp_file;
use sequent_core::temp_path::get_file_size;
use sequent_core::types::ceremonies::TallySessionDocuments;
use std::path::Path;
use tracing::instrument;

const EXCEL_STRING_LIMIT: usize = 32767;

#[instrument(err)]
pub async fn export_tally_results_to_xlsx(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    tally_session_execution_id: String,
    results_event_id: String,
    tally_session_documents: TallySessionDocuments,
    document_id: String,
) -> Result<()> {
    let sqlite_document_id = tally_session_documents
        .sqlite
        .as_ref()
        .ok_or(anyhow!("No SQLite document found"))?;

    let sqlite_document = get_document(
        hasura_transaction,
        &tenant_id,
        Some(election_event_id.clone()),
        &sqlite_document_id,
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

    let xlsx_file_path = xlsx_file.into_temp_path();
    let xlsx_file_path_string = xlsx_file_path.to_string_lossy().to_string();
    let xlsx_file_size = get_file_size(xlsx_file_path_string.as_str())
        .map_err(|e| anyhow!("Failed to get XLSX file size: {}", e))?;

    let new_tally_session_documents = TallySessionDocuments {
        xlsx: Some(document_id.clone()),
        ..tally_session_documents
    };

    update_tally_session_execution_documents(
        hasura_transaction,
        &tenant_id,
        &election_event_id.clone(),
        &tally_session_execution_id,
        new_tally_session_documents,
    )
    .await
    .map_err(|e| anyhow!("Failed to update tally session execution documents: {}", e))?;

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

fn truncate_string_for_excel(value_str: String) -> String {
    let truncated_text = if value_str.len() > EXCEL_STRING_LIMIT {
        value_str
            .chars()
            .take(EXCEL_STRING_LIMIT)
            .collect::<String>()
    } else {
        value_str.to_string()
    };
    return truncated_text;
}

/// Converts a SQLite database file to an XLSX file, with each table as a worksheet.
async fn convert_db_to_xlsx(db_path: &Path, xlsx_path: &Path) -> Result<()> {
    let db_conn = Connection::open(db_path)?;
    let mut workbook = Workbook::new();

    // Get a list of all tables in the database
    let mut stmt =
        db_conn.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")?;
    let table_names_iter = stmt.query_map([], |row| row.get(0))?;

    for table_result in table_names_iter {
        let table_name: String = table_result?;
        println!("  - Processing table: '{}'", table_name);
        let mut worksheet = workbook.add_worksheet();
        worksheet.set_name(&table_name)?;

        let mut table_stmt = db_conn.prepare(&format!("SELECT * FROM `{}`", table_name.clone()))?;

        let column_names: Vec<String> = table_stmt
            .column_names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        let column_count = table_stmt.column_count();

        for (col_index, col_name) in column_names.iter().enumerate() {
            worksheet.write_string(0, col_index as u16, col_name)?;
        }

        let mut rows = table_stmt.query([])?;

        // Write the data rows
        let mut row_index = 1;
        while let Some(row) = rows.next()? {
            for col_index in 0..column_count {
                let value_ref = row.get_ref(col_index)?;
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
                        let truncated_text = truncate_string_for_excel(text);
                        worksheet.write_string(row_index, col_index as u16, &truncated_text)?;
                    }
                    _ => {
                        // For other types like Null, Blob, etc., write as a string representation
                        let value_text = value_ref.as_str().unwrap_or("NULL");
                        let truncated_text = truncate_string_for_excel(value_text.to_string());
                        worksheet.write_string(row_index, col_index as u16, &truncated_text)?;
                    }
                }
            }
            row_index += 1;
        }
    }

    workbook.save(xlsx_path)?;
    println!(
        "Conversion successful! XLSX file created at: {}",
        xlsx_path.display()
    );

    Ok(())
}

#[instrument(err)]
pub async fn get_tally_session_execution_results_sqlite_file(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
) -> Result<(TallySessionDocuments, String, String)> {
    let tally_session_execution = get_last_tally_session_execution(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await
    .map_err(|e| anyhow!("Failed to get last tally session execution: {}", e))?
    .ok_or(anyhow!(
        "No tally session execution found for tally session id: {}",
        tally_session_id
    ))?;

    if tally_session_execution.documents.is_none() {
        return Err(anyhow!(
            "No documents found for tally session id: {}",
            tally_session_id
        ));
    }

    let documents = serde_json::to_string(&tally_session_execution.documents.unwrap().clone())?;
    let documents = deserialize_str::<TallySessionDocuments>(&documents)?;

    if (documents.sqlite.is_none()) {
        return Err(anyhow!(
            "No SQLite document found for tally session id: {}",
            tally_session_id
        ));
    }

    let results_event_id = tally_session_execution.results_event_id.ok_or(anyhow!(
        "No results event id found for tally session id: {}",
        tally_session_id
    ))?;

    Ok((documents, results_event_id, tally_session_execution.id))
}
