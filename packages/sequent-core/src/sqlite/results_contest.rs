// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::utils::{opt_f64, opt_json};
use crate::types::results::{ResultDocuments, ResultsContest};
use anyhow::{anyhow, Result};
use rusqlite::{params, Transaction};
use serde_json::{from_str, to_string};
use tracing::instrument;

#[instrument(err, skip_all)]
pub async fn create_results_contest_sqlite(
    sqlite_transaction: &Transaction<'_>,
    contests: Vec<ResultsContest>,
) -> Result<()> {
    sqlite_transaction.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS results_contest (
            id TEXT PRIMARY KEY,
            tenant_id TEXT NOT NULL,
            election_event_id TEXT NOT NULL,
            election_id TEXT NOT NULL,
            contest_id TEXT NOT NULL,
            results_event_id TEXT NOT NULL,
            elegible_census INTEGER,
            total_valid_votes INTEGER,
            explicit_invalid_votes INTEGER,
            implicit_invalid_votes INTEGER,
            blank_votes INTEGER,
            voting_type TEXT,
            counting_algorithm TEXT,
            name TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            last_updated_at TEXT DEFAULT (datetime('now')),
            labels TEXT,
            annotations TEXT,
            total_invalid_votes INTEGER,
            total_invalid_votes_percent REAL,
            total_valid_votes_percent REAL,
            explicit_invalid_votes_percent REAL,
            implicit_invalid_votes_percent REAL,
            blank_votes_percent REAL,
            total_votes INTEGER,
            total_votes_percent REAL,
            documents TEXT,
            total_auditable_votes INTEGER,
            total_auditable_votes_percent REAL
        );",
    )?;

    let mut insert = sqlite_transaction.prepare(
        "
        INSERT OR REPLACE INTO results_contest (
            id, tenant_id, election_event_id, election_id, contest_id,
            results_event_id, elegible_census, total_valid_votes,
            explicit_invalid_votes, implicit_invalid_votes, blank_votes,
            voting_type, counting_algorithm, name, labels, annotations, total_invalid_votes,
            total_invalid_votes_percent, total_valid_votes_percent,
            explicit_invalid_votes_percent, implicit_invalid_votes_percent,
            blank_votes_percent, total_votes, total_votes_percent,
            total_auditable_votes, total_auditable_votes_percent
        ) VALUES (
            ?1,?2,?3,?4,?5,
            ?6,?7,?8,
            ?9,?10,?11,
            ?12,?13,?14,?15,
            ?16,?17,?18,?19,
            ?20,?21,?22,
            ?23,?24,?25,?26
        );",
    )?;

    for c in &contests {
        insert.execute(params![
            c.id,
            c.tenant_id,
            c.election_event_id,
            c.election_id,
            c.contest_id,
            c.results_event_id,
            c.elegible_census,
            c.total_valid_votes,
            c.explicit_invalid_votes,
            c.implicit_invalid_votes,
            c.blank_votes,
            c.voting_type,
            c.counting_algorithm,
            c.name,
            opt_json(&c.labels),
            opt_json(&c.annotations),
            c.total_invalid_votes,
            opt_f64(&c.total_invalid_votes_percent),
            opt_f64(&c.total_valid_votes_percent),
            opt_f64(&c.explicit_invalid_votes_percent),
            opt_f64(&c.implicit_invalid_votes_percent),
            opt_f64(&c.blank_votes_percent),
            c.total_votes,
            opt_f64(&c.total_votes_percent),
            c.total_auditable_votes,
            opt_f64(&c.total_auditable_votes_percent),
        ])?;
    }

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn update_results_contest_documents_sqlite(
    sqlite_transaction: &Transaction<'_>,
    tenant_id: &str,
    results_event_id: &str,
    election_event_id: &str,
    election_id: &str,
    contest_id: &str,
    documents: &ResultDocuments,
) -> Result<()> {
    let docs_json = to_string(documents)
        .map_err(|e| anyhow!("Failed to serialize documents to JSON: {}", e))?;

    let insert_count = sqlite_transaction.execute(
        "
        UPDATE results_contest
        SET documents = ?1
        WHERE 
            tenant_id = ?2
            AND results_event_id = ?3
            AND election_event_id = ?4
            AND election_id = ?5
            AND contest_id = ?6
        ",
        params![
            docs_json,
            tenant_id,
            results_event_id,
            election_event_id,
            election_id,
            contest_id
        ],
    )?;

    match insert_count {
        1 => Ok(()),
        0 => Err(anyhow!("Rows not found in table results_contest")),
        count => Err(anyhow!(
            "Too many affected rows in table results_contest: {}",
            count
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
            results_contest
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