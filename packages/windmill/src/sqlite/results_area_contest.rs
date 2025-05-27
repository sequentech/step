// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use chrono::Local;
use ordered_float::NotNan;
use rusqlite::{params, Transaction};
use sequent_core::types::results::ResultsAreaContest;
use serde_json::Value;
use tracing::instrument;
use uuid::Uuid;

fn opt_str<T: ToString>(opt: &Option<T>) -> Option<String> {
    opt.as_ref().map(|v| v.to_string())
}

fn opt_json(opt: &Option<Value>) -> Option<String> {
    opt.as_ref().and_then(|v| serde_json::to_string(v).ok())
}

fn opt_f64(opt: &Option<NotNan<f64>>) -> Option<f64> {
    opt.map(|n| n.into_inner())
}

#[instrument(err, skip_all)]
pub async fn create_results_area_contests_table(
    sqlite_transaction: &Transaction<'_>,
    area_contests: Vec<ResultsAreaContest>,
) -> Result<Vec<ResultsAreaContest>> {
    // 1) create the SQLite table
    sqlite_transaction.execute_batch(
        "
        CREATE TABLE results_area_contest (
            tenant_id TEXT NOT NULL,
            election_event_id TEXT NOT NULL,
            election_id TEXT NOT NULL,
            contest_id TEXT NOT NULL,
            area_id TEXT NOT NULL,
            results_event_id TEXT NOT NULL,
            elegible_census INTEGER,
            total_valid_votes INTEGER,
            explicit_invalid_votes INTEGER,
            implicit_invalid_votes INTEGER,
            blank_votes INTEGER,
            created_at TEXT DEFAULT (datetime('now')),
            last_updated_at TEXT DEFAULT (datetime('now')),
            labels TEXT,
            annotations TEXT,
            total_valid_votes_percent REAL,
            total_invalid_votes INTEGER,
            total_invalid_votes_percent REAL,
            explicit_invalid_votes_percent REAL,
            blank_votes_percent REAL,
            implicit_invalid_votes_percent REAL,
            total_votes INTEGER,
            total_votes_percent REAL,
            documents TEXT,
            total_auditable_votes INTEGER,
            total_auditable_votes_percent REAL
        );",
    )?;

    let mut insert = sqlite_transaction.prepare(
        "
        INSERT OR REPLACE INTO results_area_contest (
            id, tenant_id, election_event_id, election_id, contest_id,
            area_id, results_event_id, elegible_census, total_valid_votes,
            explicit_invalid_votes, implicit_invalid_votes, blank_votes, labels, annotations,
            total_valid_votes_percent, total_invalid_votes,
            total_invalid_votes_percent, explicit_invalid_votes_percent,
            blank_votes_percent, implicit_invalid_votes_percent,
            total_votes, total_votes_percent,
            total_auditable_votes, total_auditable_votes_percent
        ) VALUES (
            ?1,?2,?3,?4,?5,
            ?6,?7,?8,?9,?10,
            ?11,?12,?13,?14,?15,?16,
            ?17,?18,?19,?20,?21,?22,
            ?23,?24
        );",
    )?;

    // 3) Execute for each ResultsAreaContest
    for c in &area_contests {
        insert.execute(params![
            // 1–7: UUIDs/text
            c.id,
            c.tenant_id,
            c.election_event_id,
            c.election_id,
            c.contest_id,
            c.area_id,
            c.results_event_id,
            // 8–12: integer counts
            c.elegible_census,
            c.total_valid_votes,
            c.explicit_invalid_votes,
            c.implicit_invalid_votes,
            c.blank_votes,
            opt_json(&c.labels),
            opt_json(&c.annotations),
            // 17–22: vote percentages & invalid votes
            opt_f64(&c.total_valid_votes_percent),
            c.total_invalid_votes,
            opt_f64(&c.total_invalid_votes_percent),
            opt_f64(&c.explicit_invalid_votes_percent),
            opt_f64(&c.blank_votes_percent),
            opt_f64(&c.implicit_invalid_votes_percent),
            // 23–27: totals, documents, audit fields
            c.total_votes,
            opt_f64(&c.total_votes_percent),
            c.total_auditable_votes,
            opt_f64(&c.total_auditable_votes_percent),
        ])?;
    }

    Ok(area_contests)
}
