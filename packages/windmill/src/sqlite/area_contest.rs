// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use chrono::Local;
use rusqlite::{params, Transaction};
use sequent_core::types::hasura::core::AreaContest;
use tracing::instrument;

#[instrument(err, skip_all)]
pub async fn create_area_contest_table(
    sqlite_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_contests: Vec<AreaContest>,
) -> Result<()> {
    sqlite_transaction.execute_batch(
        "
        CREATE TABLE area_contest (
            id TEXT PRIMARY KEY UNIQUE,
            tenant_id TEXT,
            election_event_id TEXT,
            contest_id TEXT,
            area_id TEXT,
            created_at TEXT,
            last_updated_at TEXT,
            labels TEXT,
            annotations TEXT
        );",
    )?;

    let mut statement = sqlite_transaction.prepare(
        "INSERT INTO area_contest (
            id, tenant_id, election_event_id, contest_id,
            area_id, created_at, last_updated_at, labels,
            annotations
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5,
            ?6, ?7, ?8, ?9
        )",
    )?;

    for area_contest in area_contests {
        statement.execute(params![
            area_contest.id,
            tenant_id,
            election_event_id,
            area_contest.contest_id,
            area_contest.area_id,
            Some(Local::now().to_string()),
            Some(Local::now().to_string()),
            None::<String>,
            None::<String>
        ])?;
    }

    Ok(())
}
