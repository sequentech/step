// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::hasura::core::ElectionEvent;
use anyhow::{anyhow, Context, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::{json, to_string, Value};
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
                statistics TEXT DEFAULT '{}',
                external_id TEXT
            );
            ",
    )?;

    let mut statement = sqlite_transaction.prepare(
        "INSERT INTO election_event
                (id, created_at, updated_at, labels, annotations, tenant_id,
                 description, presentation, bulletin_board_reference, is_archived,
                 voting_channels, status, user_boards, encryption_protocol, is_audit,
                 audit_election_event_id, public_key, statistics, external_id)
                VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, $9,
                $10, $11, $12, $13, $14, $15, $16, $17, $18, $19);
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
        election_event
            .statistics
            .as_ref()
            .map(to_string)
            .transpose()?,
        election_event.external_id,
    ])?;
    Ok(())
}

#[instrument(err, skip_all)]
pub fn replace_election_event_translation_overrides_sqlite(
    sqlite_connection: &Connection,
    election_event_id: &str,
    translation_overrides: Option<&Value>,
) -> Result<()> {
    let stored_presentation = sqlite_connection
        .query_row(
            "SELECT presentation FROM election_event WHERE id = ? LIMIT 1",
            [election_event_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .ok_or_else(|| {
            anyhow!("Election event is missing from tally results SQLite")
        })?;
    let mut presentation = stored_presentation
        .map(|value| serde_json::from_str::<Value>(&value))
        .transpose()
        .context("Invalid election event presentation in tally results SQLite")?
        .unwrap_or_else(|| json!({}));
    let presentation_object = presentation.as_object_mut().ok_or_else(|| {
        anyhow!("Election event presentation in tally results SQLite is not an object")
    })?;

    if let Some(translation_overrides) = translation_overrides {
        presentation_object
            .insert("i18n".to_string(), translation_overrides.clone());
    } else {
        presentation_object.remove("i18n");
    }

    let serialized_presentation = serde_json::to_string(&presentation)?;
    let updated_rows = sqlite_connection.execute(
        "UPDATE election_event SET presentation = ? WHERE id = ?",
        [serialized_presentation.as_str(), election_event_id],
    )?;
    if updated_rows != 1 {
        return Err(anyhow!(
            "Expected one election event row in tally results SQLite, updated {updated_rows}"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_translation_overrides_replace_tally_snapshot_without_changing_other_presentation(
    ) -> Result<()> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            r#"
                CREATE TABLE election_event (id TEXT, presentation TEXT);
                INSERT INTO election_event VALUES
                    ('event-1', '{"i18n":{"en":{"resultsPortal:key":"old"}},"css":"tally-css","unknown":{"kept":true}}'),
                    ('event-2', '{"i18n":{"en":{"resultsPortal:key":"other-event"}}}');
            "#,
        )?;

        let current_overrides = json!({
            "en": {
                "global:resultsPortal.summary.title": "Global override",
                "resultsPortal:resultsPortal.publishedResultsDescription": "Results override"
            }
        });
        replace_election_event_translation_overrides_sqlite(
            &conn,
            "event-1",
            Some(&current_overrides),
        )?;

        let presentation: String = conn.query_row(
            "SELECT presentation FROM election_event WHERE id = 'event-1'",
            [],
            |row| row.get(0),
        )?;
        let presentation: Value = serde_json::from_str(&presentation)?;
        assert_eq!(presentation["i18n"], current_overrides);
        assert_eq!(presentation["css"], "tally-css");
        assert_eq!(presentation["unknown"], json!({"kept": true}));

        let other_presentation: String = conn.query_row(
            "SELECT presentation FROM election_event WHERE id = 'event-2'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(
            serde_json::from_str::<Value>(&other_presentation)?["i18n"]["en"]
                ["resultsPortal:key"],
            "other-event"
        );

        replace_election_event_translation_overrides_sqlite(
            &conn, "event-1", None,
        )?;
        let presentation: String = conn.query_row(
            "SELECT presentation FROM election_event WHERE id = 'event-1'",
            [],
            |row| row.get(0),
        )?;
        let presentation: Value = serde_json::from_str(&presentation)?;
        assert!(presentation.get("i18n").is_none());
        assert_eq!(presentation["css"], "tally-css");
        assert_eq!(presentation["unknown"], json!({"kept": true}));

        Ok(())
    }
}
