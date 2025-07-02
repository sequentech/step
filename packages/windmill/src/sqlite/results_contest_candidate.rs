// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::utils::opt_f64;
use anyhow::Result;
use rusqlite::{params, Transaction};
use sequent_core::types::results::ResultsContestCandidate;
use tracing::instrument;

#[instrument(err, skip_all)]
pub async fn create_results_contest_candidates_sqlite(
    sqlite_transaction: &Transaction<'_>,
    contest_candidates: Vec<ResultsContestCandidate>,
) -> Result<()> {
    sqlite_transaction.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS results_contest_candidate (
            id TEXT PRIMARY KEY,
            tenant_id TEXT NOT NULL,
            election_event_id TEXT NOT NULL,
            election_id TEXT NOT NULL,
            contest_id TEXT NOT NULL,
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
        INSERT OR REPLACE INTO results_contest_candidate (
            id, tenant_id, election_event_id, election_id, contest_id,
            candidate_id, results_event_id, cast_votes, winning_position,
            points, cast_votes_percent
        ) VALUES (
            ?1,?2,?3,?4,?5,
            ?6,?7,?8,?9,?10,?11
        );",
    )?;

    for c in &contest_candidates {
        insert.execute(params![
            c.id,
            c.tenant_id,
            c.election_event_id,
            c.election_id,
            c.contest_id,
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
