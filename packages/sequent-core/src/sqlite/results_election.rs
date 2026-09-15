// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::utils::{ensure_column, opt_f64};
use crate::types::results::{ResultDocuments, ResultsElection};
use anyhow::{anyhow, Result};
use rusqlite::{params, Transaction};
use serde_json::to_string;
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
            documents TEXT,
            blank_ballots INTEGER,
            blank_ballots_percent REAL
        );",
    )?;
    // Migrate pre-existing databases (e.g. a results.db copied forward from
    // a report generated before these columns existed) that the CREATE
    // TABLE IF NOT EXISTS above won't touch.
    ensure_column(
        sqlite_transaction,
        "results_election",
        "blank_ballots",
        "INTEGER",
    )?;
    ensure_column(
        sqlite_transaction,
        "results_election",
        "blank_ballots_percent",
        "REAL",
    )?;

    let mut insert = sqlite_transaction.prepare(
        "
        INSERT OR REPLACE INTO results_election (
            id, tenant_id, election_event_id, election_id, results_event_id,
            name, elegible_census, total_voters, total_voters_percent,
            blank_ballots, blank_ballots_percent
        ) VALUES (
            ?1,?2,?3,?4,?5,
            ?6,?7,?8,?9,
            ?10,?11
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
            e.blank_ballots,
            opt_f64(&e.blank_ballots_percent),
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
                CREATE TABLE results_election (
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
                );
                INSERT INTO results_election (id, tenant_id, election_event_id, election_id, results_event_id)
                VALUES ('legacy', 'tenant-1', 'event-1', 'election-1', 'results-1');
                ",
            )
            .unwrap();

        let transaction = connection.transaction().unwrap();

        create_results_election_sqlite(
            &transaction,
            vec![ResultsElection {
                id: "new".to_string(),
                tenant_id: "tenant-1".to_string(),
                election_event_id: "event-1".to_string(),
                election_id: "election-1".to_string(),
                results_event_id: "results-1".to_string(),
                name: None,
                elegible_census: None,
                total_voters: None,
                created_at: None,
                last_updated_at: None,
                labels: None,
                annotations: None,
                total_voters_percent: None,
                documents: None,
                blank_ballots: Some(3),
                blank_ballots_percent: None,
            }],
        )
        .await
        .expect(
            "migration + insert against a pre-existing table should succeed",
        );

        transaction.commit().unwrap();

        let blank_ballots: Option<i64> = connection
            .query_row(
                "SELECT blank_ballots FROM results_election WHERE id = 'new'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(blank_ballots, Some(3));

        // The pre-existing row survives the migration untouched.
        let legacy_blank_ballots: Option<i64> = connection
            .query_row(
                "SELECT blank_ballots FROM results_election WHERE id = 'legacy'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(legacy_blank_ballots, None);
    }
}
