// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{bail, Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use std::str::FromStr;
use std::time::Duration;

use super::common::{row_to_message, validate_board_name, Board};
use crate::api_types::MessageBlob;

/// How long a connection waits for SQLite's single write lock before the
/// statement fails with `SQLITE_BUSY`.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_CONNECTIONS: u32 = 5;

#[derive(Clone)]
pub struct SqliteBackend {
    pool: sqlx::SqlitePool,
}

impl SqliteBackend {
    pub async fn open(url: &str) -> Result<Self> {
        tracing::info!("Connecting to sqlite database: {}", url);
        // - **WAL** (§8): readers and the writer do not block each other, so the
        //   trustees' fetches proceed while a confirm is being written; under the
        //   default rollback journal the pool's connections serialize on one lock.
        // - **`synchronous=FULL`**: a confirm is fsynced before b4 acknowledges it.
        //   The trustee mailbox marks a message sent on that acknowledgement (§6.4),
        //   so a row lost to a power cut after a 200 would stall the protocol; one
        //   fsync per confirm is nothing at a board's message rate.
        // - **busy timeout**: concurrent confirms queue on the write lock instead of
        //   failing at once.
        // - **foreign keys on** — sqlx's default too: a message can
        //   never reference a board that does not exist.
        // - **create if missing**: b4 owns its file.
        let options = SqliteConnectOptions::from_str(url)
            .with_context(|| format!("invalid sqlite database url {url:?}"))?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Full)
            .busy_timeout(BUSY_TIMEOUT)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(MAX_CONNECTIONS)
            .connect_with(options)
            .await?;
        let new_obj = Self { pool };
        new_obj.ensure_init().await.context("failed to init db")?;

        Ok(new_obj)
    }

    async fn ensure_init(&self) -> Result<()> {
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
        .execute(&self.pool)
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

impl SqliteBackend {
    pub async fn create_board(&self, name: &str) -> Result<Board> {
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
        .execute(&self.pool)
        .await?;

        Ok(Board {
            name: name.to_string(),
            created_at,
            status: "active".to_string(),
        })
    }

    pub async fn get_board(&self, name: &str) -> Result<Option<Board>> {
        validate_board_name(name)?;
        let row = sqlx::query_as::<_, (String, i64, String)>(
            "SELECT name, created_at, status FROM boards WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(name, created_at, status)| Board {
            name,
            created_at,
            status,
        }))
    }

    pub async fn list_boards(&self) -> Result<Vec<Board>> {
        let rows = sqlx::query_as::<_, (String, i64, String)>(
            "SELECT name, created_at, status FROM boards ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
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
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn get_message(&self, board_name: &str, id: i64) -> Result<Option<MessageBlob>> {
        validate_board_name(board_name)?;

        let row = sqlx::query_as::<_, (i64, String, Option<Vec<u8>>, Option<String>, String)>(
        "SELECT id, content_type, inline_data, s3_key, version FROM messages WHERE board_name = ? AND id = ?",
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
        "SELECT id, content_type, inline_data, s3_key, version FROM messages WHERE board_name = ? ORDER BY id ASC",
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
        "SELECT id, content_type, inline_data, s3_key, version FROM messages WHERE board_name = ? AND id > ? ORDER BY id ASC LIMIT ?",
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

#[cfg(test)]
mod tests {
    use super::SqliteBackend;
    use crate::api_types::ContentType;
    use sqlx::SqlitePool;
    use std::path::PathBuf;
    use uuid::Uuid;

    /// A fresh database file per test, in the OS temp dir, removed on drop. A
    /// file rather than `:memory:`: in-memory databases report
    /// `journal_mode=memory`, so the WAL check needs a real one.
    struct TempDb {
        backend: SqliteBackend,
        path: PathBuf,
    }

    impl TempDb {
        async fn new() -> Self {
            let path = std::env::temp_dir().join(format!("b4-db-test-{}.db", Uuid::new_v4()));
            let url = format!("sqlite:{}", path.display());
            let backend = SqliteBackend::open(&url)
                .await
                .expect("failed to open temp db");
            Self { backend, path }
        }

        async fn close(self) {
            self.backend.pool.close().await;
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
        let db = TempDb::new().await;
        assert_eq!(pragma_text(&db.backend.pool, "journal_mode").await, "wal");
        assert_eq!(pragma_int(&db.backend.pool, "foreign_keys").await, 1);
        // 2 = FULL
        assert_eq!(pragma_int(&db.backend.pool, "synchronous").await, 2);
        db.close().await;
    }

    #[tokio::test]
    async fn a_message_cannot_reference_a_missing_board() {
        let db = TempDb::new().await;

        let err = db
            .backend
            .insert_message("ghost", Some(b"x"), None, "1")
            .await
            .unwrap_err();

        assert!(
            err.to_string().to_lowercase().contains("foreign key"),
            "{err}"
        );

        db.close().await;
    }

    #[tokio::test]
    async fn boards_and_messages_round_trip_in_id_order() {
        let db = TempDb::new().await;

        assert!(db.backend.get_board("dkg1").await.unwrap().is_none());
        let board = db.backend.create_board("dkg1").await.unwrap();
        assert_eq!(board.status, "active");
        assert_eq!(
            db.backend.get_board("dkg1").await.unwrap().unwrap().name,
            "dkg1"
        );
        assert_eq!(db.backend.list_boards().await.unwrap().len(), 1);

        let first = db
            .backend
            .insert_message("dkg1", Some(&[1, 2, 3]), None, "1")
            .await
            .unwrap();
        let second = db
            .backend
            .insert_message("dkg1", None, Some("dkg1/messages/abc"), "1")
            .await
            .unwrap();
        assert!(second > first);

        let all = db.backend.list_messages("dkg1").await.unwrap();
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

        let one = db
            .backend
            .get_message("dkg1", second)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(one.id, second.to_string());
        assert!(db
            .backend
            .get_message("dkg1", second + 1)
            .await
            .unwrap()
            .is_none());

        // The incremental cursor (§8.5): everything after `first`, and the
        // truncation flag when the limit cuts the page short.
        let (after, truncated) = db
            .backend
            .get_messages_after("dkg1", first, 10)
            .await
            .unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].id, second.to_string());
        assert!(!truncated);

        let (page, truncated) = db.backend.get_messages_after("dkg1", 0, 1).await.unwrap();
        assert_eq!(page.len(), 1);
        assert!(truncated);

        db.close().await;
    }
}
