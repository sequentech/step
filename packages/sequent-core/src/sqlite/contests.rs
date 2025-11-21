// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::hasura::core::Contest;
use anyhow::Result;
use rusqlite::{params, Transaction};
use serde_json::to_string;
use tracing::instrument;

#[instrument(err, skip_all)]
pub async fn create_contest_sqlite(
    sqlite_transaction: &Transaction<'_>,
    contests: Vec<Contest>,
) -> Result<()> {
    sqlite_transaction.execute_batch(
        "
CREATE TABLE contest (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    election_event_id TEXT NOT NULL,
    election_id TEXT NOT NULL,
    created_at TEXT,
    last_updated_at TEXT,
    labels TEXT,
    annotations TEXT,
    is_acclaimed BOOLEAN,
    is_active BOOLEAN,
    name TEXT,
    description TEXT,
    presentation TEXT,
    min_votes INTEGER,
    max_votes INTEGER,
    voting_type TEXT,
    counting_algorithm TEXT,
    is_encrypted BOOLEAN,
    tally_configuration TEXT,
    conditions TEXT,
    winning_candidates_num INTEGER,
    image_document_id TEXT,
    alias TEXT
        );",
    )?;

    let mut statement = sqlite_transaction.prepare(
        "INSERT INTO contest (
            id,
            tenant_id, election_event_id, election_id, created_at, last_updated_at,
            labels, annotations, is_acclaimed, is_active, name, description,
            presentation, min_votes, max_votes, voting_type, counting_algorithm,
            is_encrypted, tally_configuration, conditions, winning_candidates_num,
            image_document_id, alias
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5,
            ?6, ?7, ?8, ?9, ?10,
            ?11, ?12, ?13, ?14, ?15,
            ?16, ?17, ?18, ?19, ?20,
            ?21, ?22, ?23
        )",
    )?;

    for contest in contests {
        statement.execute(params![
            contest.id,
            contest.tenant_id,
            contest.election_event_id,
            contest.election_id,
            contest.created_at.as_ref().map(|dt| dt.to_string()),
            contest.last_updated_at.as_ref().map(|dt| dt.to_string()),
            contest.labels.as_ref().and_then(|v| to_string(v).ok()),
            contest.annotations.as_ref().and_then(|v| to_string(v).ok()),
            contest.is_acclaimed,
            contest.is_active,
            contest.name,
            contest.description,
            contest
                .presentation
                .as_ref()
                .and_then(|v| to_string(v).ok()),
            contest.min_votes,
            contest.max_votes,
            contest.voting_type,
            contest.counting_algorithm,
            contest.is_encrypted,
            contest
                .tally_configuration
                .as_ref()
                .and_then(|v| to_string(v).ok()),
            contest.conditions.as_ref().and_then(|v| to_string(v).ok()),
            contest.winning_candidates_num,
            contest.image_document_id,
            contest.alias
        ])?;
    }

    Ok(())
}
