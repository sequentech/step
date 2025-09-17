// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::utils::opt_f64;
use crate::types::results::ResultsAreaContestCandidate;
use anyhow::Result;
use rusqlite::{params, Transaction};
use tracing::instrument;
use crate::types::results::ResultDocuments;
use serde_json::from_str; 

#[instrument(err, skip_all)]
pub async fn create_results_area_contest_candidates_sqlite(
    sqlite_transaction: &Transaction<'_>,
    area_contest_candidates: Vec<ResultsAreaContestCandidate>,
) -> Result<()> {
    sqlite_transaction.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS results_area_contest_candidate (
            id TEXT PRIMARY KEY,
            tenant_id TEXT NOT NULL,
            election_event_id TEXT NOT NULL,
            election_id TEXT NOT NULL,
            contest_id TEXT NOT NULL,
            area_id TEXT NOT NULL,
            candidate_id TEXT NOT NULL,
            results_event_id TEXT NOT NULL,
            cast_votes INTEGER,
            winning_position INTEGER,
            points INTEGER,
            created_at TEXT DEFAULT (datetime('now')),
            last_updated_at TEXT DEFAULT (datetime('now')),
            labels TEXT,
            annotations TEXT,
            cast_votes_percent REAL,
            documents TEXT
        );",
    )?;

    let mut insert = sqlite_transaction.prepare(
        "
        INSERT OR REPLACE INTO results_area_contest_candidate (
            id, tenant_id, election_event_id, election_id, contest_id,
            area_id, candidate_id, results_event_id, cast_votes,
            winning_position, points, cast_votes_percent
        ) VALUES (
            ?1,?2,?3,?4,?5,
            ?6,?7,?8,?9,?10,
            ?11,?12
        );",
    )?;

    for c in &area_contest_candidates {
        insert.execute(params![
            c.id,
            c.tenant_id,
            c.election_event_id,
            c.election_id,
            c.contest_id,
            c.area_id,
            c.candidate_id,
            c.results_event_id,
            c.cast_votes,
            c.winning_position,
            c.points,
            opt_f64(&c.cast_votes_percent),
        ])?;
    }

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
            results_area_contest_candidate
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