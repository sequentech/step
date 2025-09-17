// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::types::results::ResultDocuments;
use anyhow::{anyhow, Result};
use rusqlite::{params, Transaction};
use serde_json::{from_str, to_string};
use tracing::instrument;

#[instrument(err, skip_all)]
pub async fn create_results_election_area_sqlite(
    sqlite_transaction: &Transaction<'_>,
    tenant_id: &str,
    results_event_id: &str,
    election_event_id: &str,
    election_id: &str,
    area_id: &str,
    area_name: &str,
    documents: &ResultDocuments,
) -> Result<()> {
    sqlite_transaction.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS results_election_area (
            id TEXT NOT NULL PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
            tenant_id TEXT NOT NULL,
            election_event_id TEXT NOT NULL,
            election_id TEXT NOT NULL,
            area_id TEXT NOT NULL,
            results_event_id TEXT NOT NULL,
            created_at TEXT DEFAULT (datetime('now')),
            last_updated_at TEXT DEFAULT (datetime('now')),
            documents TEXT,
            name TEXT
        );",
    )?;

    let mut insert = sqlite_transaction.prepare(
        "
        INSERT OR REPLACE INTO results_election_area (
            tenant_id, election_event_id, election_id, area_id ,results_event_id,
            name, documents
        ) VALUES (
            ?1,?2,?3,?4,?5,
            ?6,?7
        );",
    )?;

    let docs_json = to_string(documents)
        .map_err(|e| anyhow!("Failed to serialize documents to JSON: {}", e))?;

    insert.execute(params![
        tenant_id,
        election_event_id,
        election_id,
        area_id,
        results_event_id,
        area_name,
        docs_json,
    ])?;

    Ok(())
}


#[instrument(err, skip_all)]
pub async fn get_all_documents(
    sqlite_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    results_event_id: &str,
) -> Result<Vec<ResultDocuments>> {
    let mut statement = sqlite_transaction.prepare(
        "
        SELECT 
            documents 
        FROM 
            results_election_area
        WHERE
            tenant_id = ?1
            AND election_event_id = ?2
            AND results_event_id = ?3;
        ",
    )?;

    // Use query_map to get an iterator of documents
    let document_rows = statement.query_map(
        params![tenant_id, election_event_id, results_event_id],
        |row| {
            // Get the documents as a String from the TEXT column
            let documents_text: String = row.get(0)?;
            
            // Deserialize the JSON string into your ResultDocuments struct
            from_str(&documents_text)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))
        },
    )?;

    // Collect the MappedRows iterator into a Vec<Result<ResultDocuments, rusqlite::Error>>
    let mut documents = Vec::new();
    for row in document_rows {
        // The row itself is a Result, so we need to unwrap it with '?'
        documents.push(row?);
    }
    
    // Return the collected Vec
    Ok(documents)
}