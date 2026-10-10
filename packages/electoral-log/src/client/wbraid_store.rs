// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Audit deliveries persisted as atomic opaque wbraid board messages. The query
//! index is recoverable: a committed blob is indexed before the next delivery
//! is checked, including when the previous writer lost its response.

use super::audit_query::AuditQuery;
use super::audit_types::*;
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Row};
use std::collections::HashMap;
use wbraid_b4::{api_types::ContentType, db::BoardDb};

pub type WbraidAuditStore = BoardClient;

const BLOB_VERSION: &str = "sequent-audit-v1";
const PAGE_SIZE: i64 = 256;
const MAX_BATCH_RECORDS: usize = 10_000;

pub struct BoardClient {
    board: BoardDb,
    pool: PgPool,
}
impl std::fmt::Debug for BoardClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("WbraidAuditStore")
    }
}

#[derive(Serialize, Deserialize)]
struct Delivery {
    id: String,
    payload_hash: String,
    messages: Vec<ElectoralLogMessage>,
}
#[derive(Serialize, Deserialize)]
struct Batch {
    deliveries: Vec<Delivery>,
}

impl BoardClient {
    pub async fn new(url: &str) -> Result<Self> {
        let board = BoardDb::open_url(url).await?;
        let pool = board
            .postgres_pool()
            .context("audit indexing requires PostgreSQL")?
            .clone();
        sqlx::raw_sql(r#"
            CREATE SCHEMA IF NOT EXISTS sequent_audit;
            CREATE TABLE IF NOT EXISTS sequent_audit.checkpoints (
                board_name TEXT PRIMARY KEY REFERENCES boards(name),
                last_blob_id BIGINT NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS sequent_audit.deliveries (
                board_name TEXT NOT NULL REFERENCES boards(name),
                delivery_id TEXT NOT NULL, payload_hash TEXT NOT NULL,
                blob_id BIGINT NOT NULL REFERENCES messages(id),
                PRIMARY KEY(board_name, delivery_id)
            );
            CREATE TABLE IF NOT EXISTS sequent_audit.records (
                board_name TEXT NOT NULL REFERENCES boards(name),
                blob_id BIGINT NOT NULL REFERENCES messages(id),
                id BIGINT NOT NULL, created BIGINT NOT NULL,
                sender_pk TEXT NOT NULL, statement_timestamp BIGINT NOT NULL,
                statement_kind TEXT NOT NULL, message BYTEA NOT NULL, version TEXT NOT NULL,
                user_id TEXT, username TEXT, election_id TEXT, area_id TEXT, ballot_id TEXT,
                PRIMARY KEY(board_name, id)
            );
            CREATE INDEX IF NOT EXISTS audit_records_actor ON sequent_audit.records(board_name, user_id, id);
            CREATE INDEX IF NOT EXISTS audit_records_kind_time ON sequent_audit.records(board_name, statement_kind, statement_timestamp, id);
            CREATE INDEX IF NOT EXISTS audit_records_box ON sequent_audit.records(board_name, election_id, area_id, id);
        "#).execute(&pool).await?;
        Ok(Self { board, pool })
    }

    pub async fn upsert_electoral_log_db(&mut self, name: &str) -> Result<()> {
        if self.board.get_board(name).await?.is_none() {
            if let Err(error) = self.board.create_board(name).await {
                if self.board.get_board(name).await?.is_none() {
                    return Err(error);
                }
            }
        }
        ensure!(
            self.board
                .get_board(name)
                .await?
                .is_some_and(|board| board.status == "active"),
            "audit board is archived"
        );
        Ok(())
    }

    pub async fn has_database(&mut self, name: &str) -> Result<bool> {
        Ok(self
            .board
            .get_board(name)
            .await?
            .is_some_and(|board| board.status == "active"))
    }

    /// Retain immutable messages when an application event is removed.
    pub async fn archive_board(&mut self, name: &str) -> Result<()> {
        sqlx::query("UPDATE boards SET status='archived' WHERE name=$1")
            .bind(name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn refresh_index(&self, name: &str) -> Result<()> {
        let tx = self.locked_index(name, false).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn locked_index(
        &self,
        name: &str,
        writable: bool,
    ) -> Result<sqlx::Transaction<'_, Postgres>> {
        let mut tx = self.pool.begin().await?;
        // Transaction-scoped locks release on cancellation and rollback.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
            .bind(name)
            .execute(&mut *tx)
            .await?;
        let board = self
            .board
            .get_board(name)
            .await?
            .context("audit board not found")?;
        ensure!(
            !writable || board.status == "active",
            "audit board is archived"
        );
        self.sync_index(name, &mut tx).await?;
        Ok(tx)
    }

    async fn sync_index(&self, name: &str, tx: &mut sqlx::Transaction<'_, Postgres>) -> Result<()> {
        sqlx::query(
            "INSERT INTO sequent_audit.checkpoints(board_name) VALUES($1) ON CONFLICT DO NOTHING",
        )
        .bind(name)
        .execute(&mut **tx)
        .await?;
        let mut after: i64 = sqlx::query_scalar(
            "SELECT last_blob_id FROM sequent_audit.checkpoints WHERE board_name=$1",
        )
        .bind(name)
        .fetch_one(&mut **tx)
        .await?;
        loop {
            let (blobs, more) = self
                .board
                .get_messages_after(name, after, PAGE_SIZE)
                .await?;
            for blob in blobs {
                ensure!(
                    blob.version == BLOB_VERSION,
                    "unsupported audit batch version"
                );
                let ContentType::Inline { data } = blob.content_type else {
                    anyhow::bail!("audit batch must be inline");
                };
                let blob_id: i64 = blob.id.parse().context("invalid board message ID")?;
                let batch: Batch = serde_json::from_slice(&data).context("invalid audit batch")?;
                for delivery in batch.deliveries {
                    validate_delivery(&delivery.id, &delivery.payload_hash, &delivery.messages)?;
                    sqlx::query("INSERT INTO sequent_audit.deliveries(board_name,delivery_id,payload_hash,blob_id) VALUES($1,$2,$3,$4)")
                        .bind(name).bind(&delivery.id).bind(&delivery.payload_hash).bind(blob_id)
                        .execute(&mut **tx).await?;
                    for row in delivery.messages {
                        sqlx::query(r#"INSERT INTO sequent_audit.records
                            (board_name,blob_id,id,created,sender_pk,statement_timestamp,statement_kind,message,version,user_id,username,election_id,area_id,ballot_id)
                            VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)"#)
                            .bind(name).bind(blob_id).bind(row.id).bind(row.created).bind(row.sender_pk)
                            .bind(row.statement_timestamp).bind(row.statement_kind).bind(row.message)
                            .bind(row.version).bind(row.user_id).bind(row.username).bind(row.election_id)
                            .bind(row.area_id).bind(row.ballot_id).execute(&mut **tx).await?;
                    }
                }
                after = blob_id;
            }
            sqlx::query("UPDATE sequent_audit.checkpoints SET last_blob_id=$2 WHERE board_name=$1")
                .bind(name)
                .bind(after)
                .execute(&mut **tx)
                .await?;
            if !more {
                return Ok(());
            }
        }
    }

    /// One board append contains the entire new batch and all its receipts.
    /// If indexing or commit fails afterwards, the next attempt recovers from
    /// that blob before comparing delivery IDs, so it never republishes it.
    pub async fn append_deliveries(
        &mut self,
        name: &str,
        deliveries: &[(&str, &str, &[ElectoralLogMessage])],
    ) -> Result<Vec<bool>> {
        let mut tx = self.locked_index(name, true).await?;
        let mut next_id: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(id),0) FROM sequent_audit.records WHERE board_name=$1",
        )
        .bind(name)
        .fetch_one(&mut *tx)
        .await?;
        let mut pending = Vec::new();
        let mut results = Vec::new();
        let mut seen = HashMap::new();
        for (id, hash, rows) in deliveries {
            validate_delivery(id, hash, rows)?;
            if let Some(previous) = seen.insert(*id, *hash) {
                ensure!(
                    previous == *hash,
                    "audit delivery ID reused with different input"
                );
                results.push(false);
                continue;
            }
            let existing: Option<String> = sqlx::query_scalar("SELECT payload_hash FROM sequent_audit.deliveries WHERE board_name=$1 AND delivery_id=$2")
                .bind(name).bind(id).fetch_optional(&mut *tx).await?;
            if let Some(existing) = existing {
                ensure!(
                    existing == *hash,
                    "audit delivery ID reused with different input"
                );
                results.push(false);
                continue;
            }
            let mut messages = rows.to_vec();
            for row in &mut messages {
                next_id = next_id.checked_add(1).context("audit row ID exhausted")?;
                row.id = next_id;
            }
            pending.push(Delivery {
                id: id.to_string(),
                payload_hash: hash.to_string(),
                messages,
            });
            results.push(true);
        }
        if !pending.is_empty() {
            ensure!(
                pending
                    .iter()
                    .map(|delivery| delivery.messages.len())
                    .sum::<usize>()
                    <= MAX_BATCH_RECORDS,
                "audit batch is too large"
            );
            let bytes = serde_json::to_vec(&Batch {
                deliveries: pending,
            })?;
            self.board
                .insert_message(name, Some(&bytes), None, BLOB_VERSION)
                .await?;
            self.sync_index(name, &mut tx).await?;
        }
        tx.commit().await?;
        Ok(results)
    }

    pub async fn append_delivery(
        &mut self,
        name: &str,
        id: &str,
        hash: &str,
        messages: &[ElectoralLogMessage],
    ) -> Result<bool> {
        let results = self
            .append_deliveries(name, &[(id, hash, messages)])
            .await?;
        results
            .first()
            .copied()
            .context("audit delivery result missing")
    }

    pub async fn count_electoral_log_messages(
        &mut self,
        name: &str,
        filter: Option<&AuditQuery>,
    ) -> Result<i64> {
        self.refresh_index(name).await?;
        let default = AuditQuery::default();
        let mut query = filter.unwrap_or(&default).build(name, true)?;
        Ok(query.build_query_scalar().fetch_one(&self.pool).await?)
    }

    pub async fn query_messages(
        &mut self,
        name: &str,
        filter: &AuditQuery,
    ) -> Result<Vec<ElectoralLogMessage>> {
        self.refresh_index(name).await?;
        let mut query = filter.build(name, false)?;
        Ok(query.build_query_as().fetch_all(&self.pool).await?)
    }

    pub async fn get_electoral_log_messages_batch(
        &mut self,
        name: &str,
        limit: i64,
        after_id: i64,
    ) -> Result<Vec<ElectoralLogMessage>> {
        ensure!(limit >= 0, "negative audit page size");
        self.refresh_index(name).await?;
        let rows = sqlx::query("SELECT * FROM sequent_audit.records WHERE board_name=$1 AND id>$2 ORDER BY id LIMIT $3")
            .bind(name).bind(after_id).bind(limit).fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                Ok(ElectoralLogMessage {
                    id: row.try_get("id")?,
                    created: row.try_get("created")?,
                    sender_pk: row.try_get("sender_pk")?,
                    statement_timestamp: row.try_get("statement_timestamp")?,
                    statement_kind: row.try_get("statement_kind")?,
                    message: row.try_get("message")?,
                    version: row.try_get("version")?,
                    user_id: row.try_get("user_id")?,
                    username: row.try_get("username")?,
                    election_id: row.try_get("election_id")?,
                    area_id: row.try_get("area_id")?,
                    ballot_id: row.try_get("ballot_id")?,
                })
            })
            .collect()
    }
}

fn validate_delivery(id: &str, hash: &str, messages: &[ElectoralLogMessage]) -> Result<()> {
    ensure!(
        [id, hash]
            .iter()
            .all(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())),
        "audit delivery identities must be SHA-256 digests"
    );
    ensure!(!messages.is_empty(), "audit delivery has no messages");
    Ok(())
}
