// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::path::Path;

use anyhow::{anyhow, Context, Result};
use csv::ReaderBuilder;
// use deadpool_postgres::Transaction;
// use futures::{pin_mut, StreamExt};
use rusqlite::{params, Transaction as SqliteTransaction};
// use tempfile::NamedTempFile;
// use tokio::fs::File;
// use tokio::io::AsyncWriteExt;
use tracing::instrument;

#[instrument(err, skip_all)]
pub async fn create_candidate_sqlite(
    // hasura_transaction: &Transaction<'_>,
    sqlite_transaction: &SqliteTransaction<'_>,
    // contest_ids: &Vec<String>,
    // tenant_id: &str,
    // election_event_id: &str,
) -> Result<()> {
    sqlite_transaction.execute_batch(
        "
        CREATE TABLE candidate (
            id TEXT PRIMARY KEY,
            tenant_id TEXT NOT NULL,
            election_event_id TEXT NOT NULL,
            contest_id TEXT,
            created_at TEXT,
            last_updated_at TEXT,
            labels TEXT,
            annotations TEXT,
            name TEXT,
            alias TEXT,
            description TEXT,
            type TEXT,
            presentation TEXT,
            is_public BOOLEAN,
            image_document_id TEXT
        );",
    )?;

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn import_candidate_sqlite(
    sqlite_transaction: &SqliteTransaction<'_>,
    contests_csv: &Path,
    // contest_ids: &Vec<String>,
    // tenant_id: &str,
    // election_event_id: &str,
) -> Result<()> {
    tokio::task::block_in_place(|| -> anyhow::Result<()> {
        let mut insert = sqlite_transaction.prepare(
            "INSERT INTO candidate (
                id, tenant_id, election_event_id, contest_id, created_at, last_updated_at,
                labels, annotations, name, alias, description, type, presentation,
                is_public, image_document_id
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
        )?;

        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_path(&contests_csv)
            .context("opening candidate CSV")?;

        for record in rdr.records() {
            let rec = record.context("CSV parse error")?;

            fn opt(i: &str) -> Option<String> {
                if i.is_empty() {
                    None
                } else {
                    Some(i.to_string())
                }
            }

            let is_public = match rec.get(13).unwrap_or("") {
                "t" | "true" => Some(true),
                "f" | "false" => Some(false),
                _ if rec.get(13).unwrap_or("").is_empty() => None,
                other => {
                    return Err(anyhow!(
                        "Invalid boolean in is_public column: {}",
                        other
                    ));
                }
            };

            insert.execute(params![
                rec.get(0).with_context(|| "Error fetching String record")?,
                rec.get(1).with_context(|| "Error fetching String record")?,
                rec.get(2).with_context(|| "Error fetching String record")?,
                opt(rec
                    .get(3)
                    .with_context(|| "Error fetching String record")?),
                opt(rec
                    .get(4)
                    .with_context(|| "Error fetching String record")?),
                opt(rec
                    .get(5)
                    .with_context(|| "Error fetching String record")?),
                opt(rec
                    .get(6)
                    .with_context(|| "Error fetching String record")?),
                opt(rec
                    .get(7)
                    .with_context(|| "Error fetching String record")?),
                rec.get(8).with_context(|| "Error fetching String record")?,
                opt(rec
                    .get(9)
                    .with_context(|| "Error fetching String record")?),
                opt(rec
                    .get(10)
                    .with_context(|| "Error fetching String record")?),
                rec.get(11)
                    .with_context(|| "Error fetching String record")?,
                opt(rec
                    .get(12)
                    .with_context(|| "Error fetching String record")?),
                is_public,
                opt(rec
                    .get(14)
                    .with_context(|| "Error fetching String record")?),
            ])?;
        }
        Ok(())
    })?;

    Ok(())
}
