// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use chrono::Local;
use ordered_float::NotNan;
use rusqlite::{params, Transaction};
use sequent_core::types::results::ResultsContest;
use serde_json::Value;
use tracing::instrument;
use uuid::Uuid;

fn opt_str<T: ToString>(opt: &Option<T>) -> Option<String> {
    opt.as_ref().map(|v| v.to_string())
}

fn opt_json(opt: &Option<serde_json::Value>) -> Option<String> {
    opt.as_ref().and_then(|v| serde_json::to_string(v).ok())
}

fn opt_f64(opt: &Option<ordered_float::NotNan<f64>>) -> Option<f64> {
    opt.map(|n| n.into_inner())
}

// #[instrument(err, skip_all)]
// pub async fn create_results_event_table(
//     sqlite_transaction: &Transaction<'_>,
//     tenant_id: &str,
//     election_event_id: &str,
//     results_event_id: &str,
// ) -> Result<String> {
//     sqlite_transaction.execute_batch(
//         "
//         CREATE TABLE results_contest (
//             id TEXT NOT NULL PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
//             tenant_id TEXT NOT NULL,
//             election_event_id TEXT NOT NULL,
//             name TEXT,
//             created_at TEXT DEFAULT (datetime('now')),
//             last_updated_at TEXT DEFAULT (datetime('now')),
//             annotations TEXT,
//             labels TEXT,
//             documents TEXT
//         );",
//     )?;
//     let mut statement = sqlite_transaction.prepare(
//         "INSERT INTO results_event (
//                 id, tenant_id, election_event_id
//             ) VALUES (
//                 $1, $2, $3
//             )",
//     )?;

//     statement.execute(params![results_event_id, tenant_id, election_event_id,])?;

//     Ok(results_event_id.to_string())
// }

#[instrument(err, skip_all)]
pub async fn create_results_contest_table(
    sqlite_transaction: &Transaction<'_>,
    contests: Vec<ResultsContest>,
) -> Result<()> {
    // 1) create table (if you prefer idempotence, you could use CREATE TABLE IF NOT EXISTS)
    sqlite_transaction.execute_batch(
        "
        CREATE TABLE results_contest (
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

    // 2) prepare the insert
    let mut insert = sqlite_transaction.prepare(
        "
        INSERT INTO results_contest (
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

    // 4) execute each
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
