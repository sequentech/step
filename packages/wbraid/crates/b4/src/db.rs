// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! b4 storage: a dumb, board-agnostic blob store (§8 of
//! `crates/braid/v0.6_spec.md`).
//!
//! b4 stores each message as **opaque bytes** — it does NOT interpret contents.
//! There is no parent/child lineage (the union is a client concern, §8.2), no
//! slot `UNIQUE` and no protocol metadata (`sender_pk`/`statement_kind`/... are
//! gone; the slot lives only in datalog `collides()`, §5). Boards are independent;
//! messages are ordered per board by the autoincrement `id`. The `version` string
//! is retained for the exact-match boundary check (§10.1); the `inline_data`/
//! `s3_key` split is a pure transport detail (§8.1).

use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::env;
use std::path::PathBuf;

use crate::api_types::{ContentType, MessageBlob};

#[derive(Debug, Clone)]
pub struct Board {
    pub name: String,
    pub created_at: i64,
    pub status: String,
}

pub async fn init_db() -> Result<SqlitePool> {
    // Use DATABASE_URL env var if set, otherwise default to b4.db in current directory
    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        let mut path = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        path.push("b4.db");
        // Add mode=rwc to create the file if it doesn't exist
        format!("sqlite:{}?mode=rwc", path.display())
    });

    tracing::info!("Connecting to database: {}", db_url);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // Boards are independent (no lineage, §8.2): just a name + creation time.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS boards (
            name TEXT PRIMARY KEY,
            created_at INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'active'
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // Messages are opaque blobs keyed by (board, autoincrement id). No slot
    // UNIQUE, no protocol metadata (§8.1) — b4 never interprets contents.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            board_name TEXT NOT NULL,
            content_type TEXT NOT NULL,
            inline_data BLOB,
            s3_key TEXT,
            version TEXT NOT NULL,
            FOREIGN KEY (board_name) REFERENCES boards(name)
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // Index for efficient per-board ordered scans / range queries.
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_messages_board_id
        ON messages(board_name, id)
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

/// Validates board name to prevent path traversal and SQL injection.
pub fn validate_board_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("Board name cannot be empty");
    }
    if name.len() > 255 {
        anyhow::bail!("Board name too long (max 255 characters)");
    }
    // Only allow alphanumeric, hyphens, underscores
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        anyhow::bail!("Board name contains invalid characters (only alphanumeric, -, _ allowed)");
    }
    Ok(())
}

pub async fn create_board(pool: &SqlitePool, name: &str) -> Result<Board> {
    validate_board_name(name)?;

    let created_at = chrono::Utc::now().timestamp();

    sqlx::query(
        r#"
        INSERT INTO boards (name, created_at, status)
        VALUES (?, ?, 'active')
        "#,
    )
    .bind(name)
    .bind(created_at)
    .execute(pool)
    .await?;

    Ok(Board {
        name: name.to_string(),
        created_at,
        status: "active".to_string(),
    })
}

pub async fn get_board(pool: &SqlitePool, name: &str) -> Result<Option<Board>> {
    let row = sqlx::query_as::<_, (String, i64, String)>(
        "SELECT name, created_at, status FROM boards WHERE name = ?",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(name, created_at, status)| Board {
        name,
        created_at,
        status,
    }))
}

pub async fn list_boards(pool: &SqlitePool) -> Result<Vec<Board>> {
    let rows = sqlx::query_as::<_, (String, i64, String)>(
        "SELECT name, created_at, status FROM boards ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(name, created_at, status)| Board {
            name,
            created_at,
            status,
        })
        .collect())
}

/// Insert an opaque message. Exactly one of `inline_data` / `s3_key` is set; b4
/// records which without interpreting the bytes.
pub async fn insert_message(
    pool: &SqlitePool,
    board_name: &str,
    inline_data: Option<&[u8]>,
    s3_key: Option<&str>,
    version: &str,
) -> Result<i64> {
    validate_board_name(board_name)?;

    let content_type = if inline_data.is_some() {
        "inline"
    } else {
        "s3"
    };

    let result = sqlx::query(
        r#"
        INSERT INTO messages (board_name, content_type, inline_data, s3_key, version)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(board_name)
    .bind(content_type)
    .bind(inline_data)
    .bind(s3_key)
    .bind(version)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

/// Build the API `MessageBlob` from a stored row.
fn row_to_message(
    id: i64,
    content_type: String,
    inline_data: Option<Vec<u8>>,
    s3_key: Option<String>,
    version: String,
) -> MessageBlob {
    let content_type = match content_type.as_str() {
        "s3" => ContentType::S3 {
            key: s3_key.unwrap_or_default(),
        },
        // "inline" (and any unexpected value) => inline
        _ => ContentType::Inline {
            data: inline_data.unwrap_or_default(),
        },
    };

    MessageBlob {
        id: id.to_string(),
        content_type,
        version,
    }
}

pub async fn get_message(
    pool: &SqlitePool,
    board_name: &str,
    id: i64,
) -> Result<Option<MessageBlob>> {
    validate_board_name(board_name)?;

    let row = sqlx::query_as::<_, (i64, String, Option<Vec<u8>>, Option<String>, String)>(
        "SELECT id, content_type, inline_data, s3_key, version FROM messages WHERE board_name = ? AND id = ?",
    )
    .bind(board_name)
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id, content_type, inline_data, s3_key, version)| {
        row_to_message(id, content_type, inline_data, s3_key, version)
    }))
}

/// All messages on a board, in insertion (`id`) order.
pub async fn list_messages(pool: &SqlitePool, board_name: &str) -> Result<Vec<MessageBlob>> {
    validate_board_name(board_name)?;

    let rows = sqlx::query_as::<_, (i64, String, Option<Vec<u8>>, Option<String>, String)>(
        "SELECT id, content_type, inline_data, s3_key, version FROM messages WHERE board_name = ? ORDER BY id ASC",
    )
    .bind(board_name)
    .fetch_all(pool)
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
    pool: &SqlitePool,
    board_name: &str,
    last_id: i64,
    limit: i64,
) -> Result<(Vec<MessageBlob>, bool)> {
    validate_board_name(board_name)?;

    // Fetch limit + 1 to detect if there are more messages.
    let rows = sqlx::query_as::<_, (i64, String, Option<Vec<u8>>, Option<String>, String)>(
        "SELECT id, content_type, inline_data, s3_key, version FROM messages WHERE board_name = ? AND id > ? ORDER BY id ASC LIMIT ?",
    )
    .bind(board_name)
    .bind(last_id)
    .bind(limit + 1)
    .fetch_all(pool)
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
