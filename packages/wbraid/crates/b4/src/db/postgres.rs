// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{bail, Context, Result};

use super::common::{row_to_message, validate_board_name, Board};
use crate::api_types::MessageBlob;

#[derive(Clone)]
pub struct PostgresBackend {
    pool: sqlx::PgPool,
}

impl PostgresBackend {
    pub async fn open(url: &str) -> Result<Self> {
        tracing::info!("Connecting to postgres database: {}", url);

        let pool = {
            let mut options = sqlx::postgres::PgPoolOptions::new();
            if let Ok(val) = std::env::var("PG_MAX_CONNECTIONS") {
                options = options.max_connections(
                    val.trim()
                        .parse::<u32>()
                        .context("can't parse PG_MAX_CONNECTIONS as u32")?,
                );
            }

            options
        }
        .connect(url)
        .await
        .context("failed to connect to postgres db")?;

        let new_obj = Self { pool };
        new_obj.ensure_init().await.context("failed to init db")?;
        Ok(new_obj)
    }

    async fn ensure_init(&self) -> Result<()> {
        // Boards are independent: just a name + creation time + lifecycle status.
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS boards (
                name TEXT PRIMARY KEY,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                status TEXT NOT NULL DEFAULT 'active'
            )
        "#,
        )
        .execute(&self.pool)
        .await?;

        // Messages are opaque blobs keyed by (board, autoincrement id). No slot
        // UNIQUE, no protocol metadata (§8.1) — b4 never interprets contents.
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS messages (
                id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
                board_name TEXT NOT NULL,
                content_type TEXT NOT NULL,
                inline_data BYTEA,
                s3_key TEXT,
                version TEXT NOT NULL,
                CONSTRAINT messages_board_fk
                    FOREIGN KEY (board_name) REFERENCES boards(name),
                CONSTRAINT messages_storage_check
                    CHECK (
                        (content_type = 'inline' AND inline_data IS NOT NULL AND s3_key IS NULL)
                        OR
                        (content_type = 's3' AND inline_data IS NULL AND s3_key IS NOT NULL)
                    )
            )
        "#,
        )
        .execute(&self.pool)
        .await?;

        // Index for efficient per-board ordered scans / range queries.
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_messages_board_id
            ON messages(board_name, id)
        "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

impl PostgresBackend {
    pub async fn create_board(&self, name: &str) -> Result<Board> {
        validate_board_name(name)?;

        let (name, created_at, status) =
            sqlx::query_as::<_, (_, chrono::DateTime<chrono::Utc>, _)>(
                r#"
            INSERT INTO boards (name)
            VALUES ($1)
            RETURNING name, created_at, status
            "#,
            )
            .bind(name)
            .fetch_one(&self.pool)
            .await?;

        Ok(Board {
            name,
            created_at: created_at.timestamp(),
            status,
        })
    }

    pub async fn get_board(&self, name: &str) -> Result<Option<Board>> {
        validate_board_name(name)?;
        let row = sqlx::query_as::<_, (_, chrono::DateTime<chrono::Utc>, _)>(
            "SELECT name, created_at, status FROM boards WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(name, created_at, status)| Board {
            name,
            created_at: created_at.timestamp(),
            status,
        }))
    }

    pub async fn list_boards(&self) -> Result<Vec<Board>> {
        let rows = sqlx::query_as::<_, (_, chrono::DateTime<chrono::Utc>, _)>(
            "SELECT name, created_at, status FROM boards ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(name, created_at, status)| Board {
                name,
                created_at: created_at.timestamp(),
                status,
            })
            .collect())
    }

    /// Insert an opaque message. Exactly one of `inline_data` / `s3_key` is set; b4
    /// records which without interpreting the bytes.
    pub async fn insert_message(
        &self,
        board_name: &str,
        inline_data: Option<&[u8]>,
        s3_key: Option<&str>,
        version: &str,
    ) -> Result<i64> {
        validate_board_name(board_name)?;

        let content_type = match (inline_data, s3_key) {
            (Some(_), None) => "inline",
            (None, Some(_)) => "s3",
            _ => bail!("exactly one of inline_data or s3_key must be provided"),
        };

        let id = sqlx::query_scalar(
            r#"
        INSERT INTO messages (board_name, content_type, inline_data, s3_key, version)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
        )
        .bind(board_name)
        .bind(content_type)
        .bind(inline_data)
        .bind(s3_key)
        .bind(version)
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    pub async fn get_message(&self, board_name: &str, id: i64) -> Result<Option<MessageBlob>> {
        validate_board_name(board_name)?;

        let row = sqlx::query_as::<_, (i64, String, Option<Vec<u8>>, Option<String>, String)>(
        "SELECT id, content_type, inline_data, s3_key, version FROM messages WHERE board_name = $1 AND id = $2",
        )
        .bind(board_name)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(id, content_type, inline_data, s3_key, version)| {
            row_to_message(id, content_type, inline_data, s3_key, version)
        }))
    }

    /// All messages on a board, in insertion (`id`) order.
    pub async fn list_messages(&self, board_name: &str) -> Result<Vec<MessageBlob>> {
        validate_board_name(board_name)?;

        let rows = sqlx::query_as::<_, (i64, String, Option<Vec<u8>>, Option<String>, String)>(
        "SELECT id, content_type, inline_data, s3_key, version FROM messages WHERE board_name = $1 ORDER BY id ASC",
        )
        .bind(board_name)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, content_type, inline_data, s3_key, version)| {
                row_to_message(id, content_type, inline_data, s3_key, version)
            })
            .collect())
    }

    /// Messages with `id > last_id`, in `id` order (optional incremental-fetch
    /// capability, §8.5/§12). The v0.6 client does a full re-fetch (`last_id = 0`);
    /// this is a harmless server-side convenience, never relied upon.
    pub async fn get_messages_after(
        &self,
        board_name: &str,
        last_id: i64,
        limit: i64,
    ) -> Result<(Vec<MessageBlob>, bool)> {
        validate_board_name(board_name)?;

        // Fetch limit + 1 to detect if there are more messages.
        let rows = sqlx::query_as::<_, (i64, String, Option<Vec<u8>>, Option<String>, String)>(
        "SELECT id, content_type, inline_data, s3_key, version FROM messages WHERE board_name = $1 AND id > $2 ORDER BY id ASC LIMIT $3",
        )
        .bind(board_name)
        .bind(last_id)
        .bind(limit + 1)
        .fetch_all(&self.pool)
        .await?;

        let truncated = rows.len() > limit as usize;
        let messages: Vec<MessageBlob> = rows
            .into_iter()
            .take(limit as usize)
            .map(|(id, content_type, inline_data, s3_key, version)| {
                row_to_message(id, content_type, inline_data, s3_key, version)
            })
            .collect();

        Ok((messages, truncated))
    }
}
