// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use csv::ReaderBuilder;
use deadpool_postgres::Transaction;
use futures::{pin_mut, StreamExt};
use rusqlite::{params, Transaction as SqliteTransaction};
use sequent_core::types::hasura::core::Candidate;
use serde_json::to_string;
use tempfile::{NamedTempFile, TempPath};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::instrument;

#[instrument(err, skip_all)]
pub async fn create_candidate_table(
    hasura_transaction: &Transaction<'_>,
    sqlite_transaction: &SqliteTransaction<'_>,
    contest_ids: &Vec<String>,
    tenant_id: &str,
    election_event_id: &str,
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

    let contests_csv = contest_ids
        .iter()
        .map(|id| format!("\"{}\"", id))
        .collect::<Vec<_>>()
        .join(",");

    let copy_sql = format!(
        r#"COPY (
            SELECT
                id::text,
                tenant_id,
                election_event_id::text,
                contest_id::text,
                created_at::text,
                last_updated_at::text,
                labels::text,
                annotations::text,
                name,
                alias,
                description,
                type,
                presentation::text,
                is_public::text,
                image_document_id::text
            FROM sequent_backend.candidate
            WHERE
                tenant_id = '{}'
                AND election_event_id = '{}'
                AND contest_id = ANY('{{{}}}')
        ) TO STDOUT WITH CSV HEADER"#,
        tenant_id, election_event_id, contests_csv
    );

    let mut tmp = NamedTempFile::new().context("creating temp CSV file")?;
    let mut file = File::from_std(tmp.reopen()?);

    let mut stream = hasura_transaction
        .copy_out(&copy_sql)
        .await
        .map_err(|e| anyhow!("COPY OUT failed: {}", e))?;
    pin_mut!(stream);

    while let Some(chunk) = stream.next().await {
        let data = chunk.context("Error reading COPY OUT stream")?;
        file.write_all(&data)
            .await
            .context("Error writing CSV data to temp file")?;
    }
    file.flush().await?;
    drop(file);

    let tmp_path = tmp.into_temp_path();
    tokio::task::block_in_place(|| -> anyhow::Result<()> {
        let mut insert = sqlite_transaction.prepare(
            "INSERT INTO candidate (
                id, tenant_id, election_event_id, contest_id, created_at, last_updated_at,
                labels, annotations, name, alias, description, type, presentation,
                is_public, image_document_id
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
        )?;

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(&tmp_path)
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
                    return Err(anyhow!("Invalid boolean in is_public column: {}", other));
                }
            };

            insert.execute(params![
                rec.get(0).unwrap(),
                rec.get(1).unwrap(),
                rec.get(2).unwrap(),
                opt(rec.get(3).unwrap()),
                opt(rec.get(4).unwrap()),
                opt(rec.get(5).unwrap()),
                opt(rec.get(6).unwrap()),
                opt(rec.get(7).unwrap()),
                rec.get(8).unwrap(),
                opt(rec.get(9).unwrap()),
                opt(rec.get(10).unwrap()),
                rec.get(11).unwrap(),
                opt(rec.get(12).unwrap()),
                is_public,
                opt(rec.get(14).unwrap()),
            ])?;
        }
        Ok(())
    })?;

    Ok(())
}
