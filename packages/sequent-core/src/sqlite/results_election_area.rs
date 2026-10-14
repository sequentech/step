// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::utils::{ensure_column, opt_f64};
use crate::types::results::ResultDocuments;
use anyhow::{anyhow, Result};
use ordered_float::NotNan;
use rusqlite::{params, Transaction};
use serde_json::to_string;
use tracing::instrument;

#[instrument(err, skip_all)]
#[expect(
    clippy::too_many_arguments,
    reason = "Keep the existing results-row API with explicit scope identities, documents and blank-ballot values"
)]
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
    // Migrate pre-existing databases (e.g. a results.db copied forward from
    // a report generated before these columns existed) that the CREATE
    // TABLE IF NOT EXISTS above won't touch.
    ensure_column(
        sqlite_transaction,
        "results_election_area",
        "blank_ballots",
        "INTEGER",
    )?;
    ensure_column(
        sqlite_transaction,
        "results_election_area",
        "blank_ballots_percent",
        "REAL",
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

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[tokio::test]
    async fn migrates_pre_existing_table_missing_blank_ballots_columns() {
        let mut connection = Connection::open_in_memory().unwrap();

        // Simulate a results.db produced before blank_ballots existed: the
        // table is already there, without the new columns.
        connection
            .execute_batch(
                "
                CREATE TABLE results_election_area (
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
                );
                ",
            )
            .unwrap();

        let transaction = connection.transaction().unwrap();

        create_results_election_area_sqlite(
            &transaction,
            "tenant-1",
            "results-1",
            "event-1",
            "election-1",
            "area-1",
            "Area 1",
            &ResultDocuments::default(),
            Some(2),
            None,
        )
        .await
        .expect(
            "migration + insert against a pre-existing table should succeed",
        );

        transaction.commit().unwrap();

        let blank_ballots: Option<i64> = connection
            .query_row(
                "SELECT blank_ballots FROM results_election_area WHERE area_id = 'area-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(blank_ballots, Some(2));
    }
}
