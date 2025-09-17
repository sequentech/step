// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::utils::opt_f64;
use crate::types::results::{ResultDocuments, ResultsElection};
use anyhow::{anyhow, Result};
use rusqlite::{params, Transaction};
use serde_json::{from_str, to_string};
use tracing::instrument;

#[instrument(err, skip_all)]
pub async fn create_results_election_sqlite(
    sqlite_transaction: &Transaction<'_>,
    elections: Vec<ResultsElection>,
) -> Result<()> {
    sqlite_transaction.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS results_election (
            id TEXT PRIMARY KEY,
            tenant_id TEXT NOT NULL,
            election_event_id TEXT NOT NULL,
            election_id TEXT NOT NULL,
            results_event_id TEXT NOT NULL,
            name TEXT,
            elegible_census INTEGER,
            total_voters INTEGER,
            created_at TEXT DEFAULT (datetime('now')),
            last_updated_at TEXT DEFAULT (datetime('now')),
            labels TEXT,
            annotations TEXT,
            total_voters_percent REAL,
            documents TEXT
        );",
    )?;

    let mut insert = sqlite_transaction.prepare(
        "
        INSERT OR REPLACE INTO results_election (
            id, tenant_id, election_event_id, election_id, results_event_id,
            name, elegible_census, total_voters, total_voters_percent
        ) VALUES (
            ?1,?2,?3,?4,?5,
            ?6,?7,?8,
            ?9
        );",
    )?;

    for e in &elections {
        insert.execute(params![
            e.id,
            e.tenant_id,
            e.election_event_id,
            e.election_id,
            e.results_event_id,
            e.name,
            e.elegible_census,
            e.total_voters,
            opt_f64(&e.total_voters_percent),
        ])?;
    }

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn update_results_election_documents_sqlite(
    sqlite_transaction: &Transaction<'_>,
    tenant_id: &str,
    results_event_id: &str,
    election_event_id: &str,
    election_id: &str,
    documents: &ResultDocuments,
    json_hash: &str,
) -> Result<()> {
    let docs_json = to_string(documents)
        .map_err(|e| anyhow!("Failed to serialize documents to JSON: {}", e))?;

    let insert_count = sqlite_transaction.execute(
        "
        UPDATE results_election
           SET documents   = ?1,
               annotations = json_set(
                   coalesce(annotations, '{}'),
                   '$.results_hash',
                   ?2
               )
         WHERE tenant_id        = ?3
           AND results_event_id = ?4
           AND election_event_id= ?5
           AND election_id      = ?6
        ",
        params![
            docs_json,
            json_hash,
            tenant_id,
            results_event_id,
            election_event_id,
            election_id
        ],
    )?;

    match insert_count {
        1 => Ok(()),
        0 => Err(anyhow!("Rows not found in table results_election")),
        n => Err(anyhow!(
            "Too many affected rows in table results_election: {}",
            n
        )),
    }
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
            results_election
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