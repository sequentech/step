// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::ceremonies::TallySessionResolution;
use anyhow::Result;
use rusqlite::{params, Transaction};
use serde_json::to_string;
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
            contest_id TEXT,
            created_at TEXT,
            last_updated_at TEXT,
            resolution_type TEXT NOT NULL,
            status TEXT NOT NULL,
            resolution_data TEXT,
            resolved_by_user TEXT,
            resolved_at TEXT,
            labels TEXT,
            annotations TEXT
        );",
    )?;

    let mut insert = sqlite_transaction.prepare(
        "INSERT OR REPLACE INTO tally_session_resolution (
            id, tenant_id, election_event_id, tally_session_id,
            contest_id,
            created_at, last_updated_at, resolution_type, status,
            resolution_data, resolved_by_user, resolved_at,
            labels, annotations
        ) VALUES (
            ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14
        );",
    )?;

    for r in &resolutions {
        insert.execute(params![
            r.id,
            r.tenant_id,
            r.election_event_id,
            r.tally_session_id,
            r.contest_id,
            r.created_at.map(|dt| dt.to_rfc3339()),
            r.last_updated_at.map(|dt| dt.to_rfc3339()),
            r.resolution_type.to_string(),
            r.status.to_string(),
            r.resolution_data.as_ref().map(to_string).transpose()?,
            r.resolved_by_user,
            r.resolved_at.map(|dt| dt.to_rfc3339()),
            r.labels.as_ref().map(to_string).transpose()?,
            r.annotations.as_ref().map(to_string).transpose()?,
        ])?;
    }

    Ok(())
}
