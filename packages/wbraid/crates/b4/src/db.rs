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

use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;
use std::env;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use crate::api_types::{ContentType, MessageBlob};

/// Names the database as a sqlx SQLite URL (`sqlite:<path>?mode=rwc`). Unset,
/// b4 uses `b4.db` in the current directory.
pub const DATABASE_URL_ENV: &str = "DATABASE_URL";
const DEFAULT_DATABASE_FILE: &str = "b4.db";

/// How long a connection waits for SQLite's single write lock before the
/// statement fails with `SQLITE_BUSY`.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_CONNECTIONS: u32 = 5;

#[derive(Debug, Clone)]
pub struct Board {
    pub name: String,
    pub created_at: i64,
    pub status: String,
}

/// The database URL from [`DATABASE_URL_ENV`], or the default file.
pub fn database_url_from_env() -> String {
    env::var(DATABASE_URL_ENV).unwrap_or_else(|_| {
        let mut path = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        path.push(DEFAULT_DATABASE_FILE);
        // mode=rwc creates the file if it doesn't exist
        format!("sqlite:{}?mode=rwc", path.display())
    })
}

/// The options every connection in the pool is opened with. Left to sqlx 0.8,
/// a connection gets `foreign_keys=ON` and a 5 s busy timeout and nothing
/// else — sqlx stopped setting a journal mode so as not to switch an existing
/// database into or out of WAL — so the rest is spelled out here:
///
/// - **WAL** (§8): readers and the writer do not block each other, so the
///   trustees' fetches proceed while a confirm is being written; under the
///   default rollback journal the pool's connections serialize on one lock.
/// - **`synchronous=FULL`**: a confirm is fsynced before b4 acknowledges it.
///   The trustee mailbox marks a message sent on that acknowledgement (§6.4),
///   so a row lost to a power cut after a 200 would stall the protocol; one
///   fsync per confirm is nothing at a board's message rate.
/// - **busy timeout**: concurrent confirms queue on the write lock instead of
///   failing at once.
/// - **foreign keys on** — sqlx's default too: a message can
///   never reference a board that does not exist.
/// - **create if missing**: b4 owns its file.
pub fn connect_options(db_url: &str) -> Result<SqliteConnectOptions> {
    let options = SqliteConnectOptions::from_str(db_url)
        .with_context(|| format!("invalid {DATABASE_URL_ENV} {db_url:?}"))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Full)
        .busy_timeout(BUSY_TIMEOUT)
        .foreign_keys(true);
    Ok(options)
}

pub async fn init_db() -> Result<SqlitePool> {
    init_db_at(&database_url_from_env()).await
}

/// Open the database at `db_url`, creating it if needed, and ensure the schema.
pub async fn init_db_at(db_url: &str) -> Result<SqlitePool> {
    tracing::info!("Connecting to database: {}", db_url);

    let pool = SqlitePoolOptions::new()
        .max_connections(MAX_CONNECTIONS)
        .connect_with(connect_options(db_url)?)
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

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    /// A fresh database file per test, in the OS temp dir, removed on drop. A
    /// file rather than `:memory:`: in-memory databases report
    /// `journal_mode=memory`, so the WAL check needs a real one.
    struct TempDb {
        path: PathBuf,
    }

    impl TempDb {
        fn new() -> Self {
            let path = env::temp_dir().join(format!("b4-db-test-{}.db", Uuid::new_v4()));
            Self { path }
        }

        fn url(&self) -> String {
            format!("sqlite:{}?mode=rwc", self.path.display())
        }
    }

    impl Drop for TempDb {
        fn drop(&mut self) {
            for suffix in ["", "-wal", "-shm"] {
                let _ = std::fs::remove_file(format!("{}{}", self.path.display(), suffix));
            }
        }
    }

    async fn pragma_text(pool: &SqlitePool, name: &str) -> String {
        let sql = format!("PRAGMA {name}");
        sqlx::query_scalar(&sql).fetch_one(pool).await.unwrap()
    }

    async fn pragma_int(pool: &SqlitePool, name: &str) -> i64 {
        let sql = format!("PRAGMA {name}");
        sqlx::query_scalar(&sql).fetch_one(pool).await.unwrap()
    }

    #[tokio::test]
    async fn connections_run_in_wal_with_foreign_keys_and_full_sync() {
        let db = TempDb::new();
        let pool = init_db_at(&db.url()).await.unwrap();

        assert_eq!(pragma_text(&pool, "journal_mode").await, "wal");
        assert_eq!(pragma_int(&pool, "foreign_keys").await, 1);
        // 2 = FULL
        assert_eq!(pragma_int(&pool, "synchronous").await, 2);

        pool.close().await;
    }

    #[tokio::test]
    async fn a_message_cannot_reference_a_missing_board() {
        let db = TempDb::new();
        let pool = init_db_at(&db.url()).await.unwrap();

        let err = insert_message(&pool, "ghost", Some(b"x"), None, "1")
            .await
            .unwrap_err();

        assert!(
            err.to_string().to_lowercase().contains("foreign key"),
            "{err}"
        );
        pool.close().await;
    }

    #[tokio::test]
    async fn boards_and_messages_round_trip_in_id_order() {
        let db = TempDb::new();
        let pool = init_db_at(&db.url()).await.unwrap();

        assert!(get_board(&pool, "dkg1").await.unwrap().is_none());
        let board = create_board(&pool, "dkg1").await.unwrap();
        assert_eq!(board.status, "active");
        assert_eq!(
            get_board(&pool, "dkg1").await.unwrap().unwrap().name,
            "dkg1"
        );
        assert_eq!(list_boards(&pool).await.unwrap().len(), 1);

        let first = insert_message(&pool, "dkg1", Some(&[1, 2, 3]), None, "1")
            .await
            .unwrap();
        let second = insert_message(&pool, "dkg1", None, Some("dkg1/messages/abc"), "1")
            .await
            .unwrap();
        assert!(second > first);

        let all = list_messages(&pool, "dkg1").await.unwrap();
        let ids: Vec<&str> = all.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, [first.to_string(), second.to_string()]);
        match &all[0].content_type {
            ContentType::Inline { data } => assert_eq!(data, &[1, 2, 3]),
            other => panic!("expected inline content, got {other:?}"),
        }
        match &all[1].content_type {
            ContentType::S3 { key } => assert_eq!(key, "dkg1/messages/abc"),
            other => panic!("expected an s3 key, got {other:?}"),
        }

        let one = get_message(&pool, "dkg1", second).await.unwrap().unwrap();
        assert_eq!(one.id, second.to_string());
        assert!(get_message(&pool, "dkg1", second + 1)
            .await
            .unwrap()
            .is_none());

        // The incremental cursor (§8.5): everything after `first`, and the
        // truncation flag when the limit cuts the page short.
        let (after, truncated) = get_messages_after(&pool, "dkg1", first, 10).await.unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].id, second.to_string());
        assert!(!truncated);

        let (page, truncated) = get_messages_after(&pool, "dkg1", 0, 1).await.unwrap();
        assert_eq!(page.len(), 1);
        assert!(truncated);

        pool.close().await;
    }

    #[test]
    fn board_names_are_restricted_to_a_safe_alphabet() {
        assert!(validate_board_name("dkg-1_tally").is_ok());
        assert!(validate_board_name("").is_err());
        assert!(validate_board_name(&"a".repeat(256)).is_err());
        for bad in ["a/b", "a b", "a.b", "../x"] {
            assert!(validate_board_name(bad).is_err(), "{bad}");
        }
    }
}
