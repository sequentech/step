// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use rusqlite::{params, Transaction};
use tracing::instrument;
use uuid::Uuid;

#[instrument(err, skip_all)]
pub async fn create_results_event_table(
    sqlite_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    results_event_id: &str,
) -> Result<String> {
    sqlite_transaction.execute_batch(
        "
        CREATE TABLE results_event (
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
        "INSERT INTO results_event (
                id, tenant_id, election_event_id
            ) VALUES (
                $1, $2, $3
            )",
    )?;

    statement.execute(params![results_event_id, tenant_id, election_event_id,])?;

    Ok(results_event_id.to_string())
}
