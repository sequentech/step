// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use rusqlite::{params, Transaction};
use sequent_core::types::hasura::core::Area;
use serde_json::to_string;
use tracing::instrument;

#[instrument(err, skip_all)]
pub async fn create_area_sqlite(
    sqlite_transaction: &Transaction<'_>,
    areas: Vec<Area>,
) -> Result<()> {
    sqlite_transaction.execute_batch(
        "CREATE TABLE area (
            id TEXT PRIMARY KEY,
            tenant_id TEXT NOT NULL,
            election_event_id TEXT NOT NULL,
            created_at TEXT,
            last_updated_at TEXT,
            labels TEXT,
            annotations TEXT,
            name TEXT,
            description TEXT,
            type TEXT,
            parent_id TEXT
        );",
    )?;

    let mut statement = sqlite_transaction.prepare(
        "INSERT INTO area (
            id, tenant_id, election_event_id, created_at,
            last_updated_at, labels, annotations, name,
            description, type, parent_id
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5,
            ?6, ?7, ?8, ?9, ?10, ?11
        )",
    )?;

    for area in areas {
        statement.execute(params![
            area.id,
            area.tenant_id,
            area.election_event_id,
            area.created_at.as_ref().map(|dt| dt.to_string()),
            area.last_updated_at.as_ref().map(|dt| dt.to_string()),
            area.labels.as_ref().and_then(|v| to_string(v).ok()),
            area.annotations.as_ref().and_then(|v| to_string(v).ok()),
            area.name,
            area.description,
            area.r#type,
            area.parent_id
        ])?;
    }

    Ok(())
}
