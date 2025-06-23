// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use rusqlite::{params, Transaction};
use sequent_core::types::hasura::core::ElectionEvent;
use serde_json::to_string;
use tracing::instrument;

#[instrument(err, skip_all)]
pub async fn create_election_event_sqlite(
    sqlite_transaction: &Transaction<'_>,
    election_event: ElectionEvent,
) -> Result<()> {
    sqlite_transaction.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS election_event (
                id TEXT NOT NULL PRIMARY KEY,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
                labels TEXT,
                annotations TEXT,
                tenant_id TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                presentation TEXT,
                bulletin_board_reference TEXT,
                is_archived INTEGER DEFAULT 0,
                voting_channels TEXT,
                status TEXT,
                user_boards TEXT,
                encryption_protocol TEXT NOT NULL,
                is_audit INTEGER,
                audit_election_event_id TEXT,
                public_key TEXT,
                alias TEXT,
                statistics TEXT DEFAULT '{}'
            );
            ",
    )?;

    let mut statement = sqlite_transaction.prepare(
        "INSERT INTO election_event
                (id, created_at, updated_at, labels, annotations, tenant_id, name,
                 description, presentation, bulletin_board_reference, is_archived,
                 voting_channels, status, user_boards, encryption_protocol, is_audit,
                 audit_election_event_id, public_key, alias, statistics)
                VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, $9,
                $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20);
            ",
    )?;

    statement.execute(params![
        election_event.id,
        election_event.created_at.map(|dt| dt.to_rfc3339()),
        election_event.updated_at.map(|dt| dt.to_rfc3339()),
        election_event.labels.as_ref().map(to_string).transpose()?,
        election_event
            .annotations
            .as_ref()
            .map(to_string)
            .transpose()?,
        election_event.tenant_id,
        election_event.name,
        election_event.description,
        election_event
            .presentation
            .as_ref()
            .map(to_string)
            .transpose()?,
        election_event
            .bulletin_board_reference
            .as_ref()
            .map(to_string)
            .transpose()?,
        election_event.is_archived,
        election_event
            .voting_channels
            .as_ref()
            .map(to_string)
            .transpose()?,
        election_event.status.as_ref().map(to_string).transpose()?,
        election_event.user_boards,
        election_event.encryption_protocol,
        election_event.is_audit,
        election_event.audit_election_event_id,
        election_event.public_key,
        election_event.alias,
        election_event
            .statistics
            .as_ref()
            .map(to_string)
            .transpose()?,
    ])?;
    Ok(())
}
