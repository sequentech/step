// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use rusqlite::{params, Transaction};
use sequent_core::types::results::ResultDocuments;
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

    statement.execute(params![results_event_id, tenant_id, election_event_id,])?;

    Ok(results_event_id.to_string())
}

#[instrument(err, skip_all)]
pub async fn update_results_event_documents_sqlite(
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
