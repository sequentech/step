// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::results::ResultDocuments;
use crate::types::results::ResultsEvent;
use anyhow::{anyhow, Result};
use chrono::{Local, NaiveDateTime};
use rusqlite::{params, Transaction};
use serde_json::to_string;
use tracing::instrument;

#[instrument(err, skip_all)]
pub async fn create_results_event_sqlite(
    sqlite_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    results_event_id: &str,
) -> Result<String> {
    sqlite_transaction.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS results_event (
            id TEXT NOT NULL PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))), 
            tenant_id TEXT NOT NULL,
            election_event_id TEXT NOT NULL,
            name TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            last_updated_at TEXT DEFAULT (datetime('now')),
            annotations TEXT,
            labels TEXT,
            documents TEXT
        );",
    )?;
    let mut statement = sqlite_transaction.prepare(
        "INSERT OR REPLACE INTO results_event (
                id, tenant_id, election_event_id
            ) VALUES (
                $1, $2, $3
            )",
    )?;

    statement.execute(params![
        results_event_id,
        tenant_id,
        election_event_id,
    ])?;

    Ok(results_event_id.to_string())
}

pub fn find_results_event_sqlite(
    sqlite_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<ResultsEvent> {
    // The query is defined to select all columns needed for the ResultsEvent
    // struct.
    let query = "
        SELECT 
            id, tenant_id, election_event_id, name, created_at,
            last_updated_at, labels, annotations, documents
        FROM
            results_event
        WHERE
            tenant_id = ?1 AND election_event_id = ?2
    ";

    // `query_row` is used to execute the query on the transaction and expects
    // exactly one row to be returned.
    sqlite_transaction
        .query_row(query, params![tenant_id, election_event_id], |row| {
            let created_at_str: Option<String> = row.get(4)?;
            let last_updated_at_str: Option<String> = row.get(5)?;
            let labels_json_str: Option<String> = row.get(6)?;
            let annotations_json_str: Option<String> = row.get(7)?;
            let documents_json_str: Option<String> = row.get(8)?;

            Ok(ResultsEvent {
                id: row.get(0)?,
                tenant_id: row.get(1)?,
                election_event_id: row.get(2)?,
                name: row.get(3)?,
                created_at: created_at_str
                    .and_then(|s| {
                        NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                            .ok()
                    })
                    .map(|nd| nd.and_local_timezone(Local).unwrap()),
                last_updated_at: last_updated_at_str
                    .and_then(|s| {
                        NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                            .ok()
                    })
                    .map(|nd| nd.and_local_timezone(Local).unwrap()),
                labels: labels_json_str
                    .and_then(|s| serde_json::from_str(&s).ok()),
                annotations: annotations_json_str
                    .and_then(|s| serde_json::from_str(&s).ok()),
                documents: documents_json_str
                    .and_then(|s| serde_json::from_str(&s).ok()),
            })
        })
        .map_err(|error| {
            anyhow!("Error getting results_event from sqlite database: {error}")
        })
}

#[instrument(err, skip_all)]
pub fn update_results_event_documents_sqlite(
    sqlite_transaction: &Transaction<'_>,
    tenant_id: &str,
    results_event_id: &str,
    election_event_id: &str,
    documents: &ResultDocuments,
) -> Result<()> {
    let docs_json = to_string(documents)
        .map_err(|e| anyhow!("Failed to serialize documents to JSON: {}", e))?;

    let insert_count = sqlite_transaction.execute(
        "
        UPDATE results_event
        SET documents = ?1
        WHERE tenant_id = ?2
          AND id = ?3
          AND election_event_id = ?4
        ",
        params![docs_json, tenant_id, results_event_id, election_event_id],
    )?;

    match insert_count {
        1 => Ok(()),
        0 => Err(anyhow!("Rows not found in table results_event")),
        count => Err(anyhow!(
            "Too many affected rows in table results_event: {}",
            count
        )),
    }
}
