// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::results::TallySessionResolution;
use anyhow::Result;
use rusqlite::{params, Transaction};
use tracing::instrument;

#[instrument(err, skip_all)]
pub async fn create_tally_session_resolutions_sqlite(
    sqlite_transaction: &Transaction<'_>,
    resolutions: Vec<TallySessionResolution>,
) -> Result<()> {
    sqlite_transaction.execute_batch(
        "CREATE TABLE IF NOT EXISTS tally_session_resolution (
            id TEXT PRIMARY KEY,
            tenant_id TEXT NOT NULL,
            election_event_id TEXT NOT NULL,
            tally_session_id TEXT NOT NULL,
            results_contest_id TEXT,
            contest_id TEXT,
            results_event_id TEXT,
            created_at TEXT,
            last_updated_at TEXT,
            resolution_type TEXT NOT NULL,
            status TEXT NOT NULL,
            resolution_data TEXT,
            resolution TEXT,
            resolved_by_user TEXT,
            resolved_at TEXT
        );",
    )?;

    let mut insert = sqlite_transaction.prepare(
        "INSERT OR REPLACE INTO tally_session_resolution (
            id, tenant_id, election_event_id, tally_session_id,
            results_contest_id, contest_id, results_event_id,
            created_at, last_updated_at, resolution_type, status,
            resolution_data, resolution, resolved_by_user, resolved_at
        ) VALUES (
            ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15
        );",
    )?;

    for r in &resolutions {
        insert.execute(params![
            r.id,
            r.tenant_id,
            r.election_event_id,
            r.tally_session_id,
            r.results_contest_id,
            r.contest_id,
            r.results_event_id,
            r.created_at,
            r.last_updated_at,
            r.resolution_type,
            r.status,
            r.resolution_data,
            r.resolution,
            r.resolved_by_user,
            r.resolved_at,
        ])?;
    }

    Ok(())
}
