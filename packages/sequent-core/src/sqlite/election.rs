// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::hasura::core::Election;
use anyhow::Result;
use rusqlite::{params, Transaction};
use serde_json::to_string;
use tracing::instrument;

#[instrument(err, skip_all)]
pub async fn create_election_sqlite(
    sqlite_transaction: &Transaction<'_>,
    elections: Vec<Election>,
) -> Result<()> {
    sqlite_transaction.execute_batch(
        "
        CREATE TABLE election (
            id TEXT NOT NULL PRIMARY KEY,
            tenant_id TEXT NOT NULL,
            election_event_id TEXT NOT NULL,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            last_updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
            labels TEXT,
            annotations TEXT,
            name TEXT NOT NULL,
            description TEXT,
            presentation TEXT,
            status TEXT,
            eml TEXT,
            num_allowed_revotes INTEGER,
            is_consolidated_ballot_encoding BOOLEAN,
            spoil_ballot_option BOOLEAN,
            alias TEXT,
            voting_channels TEXT,
            is_kiosk BOOLEAN DEFAULT FALSE,
            image_document_id TEXT,
            statistics TEXT DEFAULT '{}',
            receipts TEXT,
            permission_label TEXT,
            keys_ceremony_id TEXT,
            initialization_report_generated BOOLEAN DEFAULT FALSE
        );",
    )?;

    let mut statement = sqlite_transaction.prepare(
        "INSERT INTO election (
                id, tenant_id, election_event_id, created_at, last_updated_at, labels,
                annotations, name, description, presentation, status, eml,
                num_allowed_revotes, is_consolidated_ballot_encoding, spoil_ballot_option,
                alias, voting_channels, is_kiosk, image_document_id, statistics,
                receipts, permission_label, keys_ceremony_id, initialization_report_generated
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6,
                ?7, ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, ?15,
                ?16, ?17, ?18, ?19, ?20,
                ?21, ?22, ?23, ?24
            )",
    )?;

    for election in elections {
        statement.execute(params![
            // 1
            election.id,
            // 2
            election.tenant_id,
            // 3
            election.election_event_id,
            // 4
            election.created_at.as_ref().map(|dt| dt.to_string()),
            // 5
            election.last_updated_at.as_ref().map(|dt| dt.to_string()),
            // 6
            election.labels.as_ref().and_then(|v| to_string(v).ok()),
            // 7
            election
                .annotations
                .as_ref()
                .and_then(|v| to_string(v).ok()),
            // 8
            election.name,
            // 9
            election.description,
            // 10
            election
                .presentation
                .as_ref()
                .and_then(|v| to_string(v).ok()),
            // 11
            election.status.as_ref().and_then(|v| to_string(v).ok()),
            // 12
            election.eml,
            // 13
            election.num_allowed_revotes,
            // 14
            election.is_consolidated_ballot_encoding,
            // 15
            election.spoil_ballot_option,
            // 16
            election.alias,
            // 17
            election
                .voting_channels
                .as_ref()
                .and_then(|v| to_string(v).ok()),
            // 18
            election.is_kiosk.unwrap_or(false),
            // 19
            election.image_document_id,
            // 20
            election.statistics.as_ref().and_then(|v| to_string(v).ok()),
            // 21
            election.receipts.as_ref().and_then(|v| to_string(v).ok()),
            // 22
            election.permission_label,
            // 23
            election.keys_ceremony_id,
            // 24
            election.initialization_report_generated,
        ])?;
    }
    Ok(())
}
