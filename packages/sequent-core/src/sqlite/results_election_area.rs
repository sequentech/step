// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::utils::opt_f64;
use crate::types::results::ResultDocuments;
use anyhow::{anyhow, Result};
use ordered_float::NotNan;
use rusqlite::{params, Transaction};
use serde_json::to_string;
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
    blank_ballots: Option<i64>,
    blank_ballots_percent: Option<NotNan<f64>>,
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
            name TEXT,
            blank_ballots INTEGER,
            blank_ballots_percent REAL
        );",
    )?;

    let mut insert = sqlite_transaction.prepare(
        "
        INSERT OR REPLACE INTO results_election_area (
            tenant_id, election_event_id, election_id, area_id ,results_event_id,
            name, documents, blank_ballots, blank_ballots_percent
        ) VALUES (
            ?1,?2,?3,?4,?5,
            ?6,?7,?8,?9
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
        blank_ballots,
        opt_f64(&blank_ballots_percent),
    ])?;

    Ok(())
}
