use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use crate::api_types::Message;
use std::env;
use std::path::PathBuf;

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

    // Create boards table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS boards (
            name TEXT PRIMARY KEY,
            created_at INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'active',
            cfg_id TEXT,
            threshold_no INTEGER,
            trustees_no INTEGER,
            last_message_kind TEXT,
            message_count INTEGER DEFAULT 0,
            batch_count INTEGER DEFAULT 0
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // Create messages table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            board_name TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            sender_pk TEXT NOT NULL,
            statement_kind TEXT NOT NULL,
            batch INTEGER NOT NULL DEFAULT 0,
            mix_number INTEGER NOT NULL DEFAULT 0,
            size INTEGER NOT NULL,
            content_type TEXT NOT NULL,
            inline_data BLOB,
            s3_key TEXT,
            version TEXT NOT NULL,
            FOREIGN KEY (board_name) REFERENCES boards(name),
            UNIQUE (board_name, sender_pk, statement_kind, batch, mix_number)
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // Create index for efficient range queries
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

/// Validates board name to prevent path traversal and SQL injection
pub fn validate_board_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("Board name cannot be empty");
    }
    if name.len() > 255 {
        anyhow::bail!("Board name too long (max 255 characters)");
    }
    // Only allow alphanumeric, hyphens, underscores
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
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

pub async fn insert_message(
    pool: &SqlitePool,
    board_name: &str,
    message: &Message,
    inline_data: Option<&[u8]>,
    s3_key: Option<&str>,
    version: &str,
    sender_pk: &str,
    statement_kind: &str,
    batch: i32,
    mix_number: i32,
) -> Result<i64> {
    validate_board_name(board_name)?;
    
    let content_type = match &message.content_type {
        crate::api_types::ContentType::Inline { .. } => "inline",
        crate::api_types::ContentType::S3 { .. } => "s3",
    };

    let result = sqlx::query(
        r#"
        INSERT INTO messages (board_name, timestamp, size, content_type, inline_data, s3_key, version, sender_pk, statement_kind, batch, mix_number)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(board_name)
    .bind(message.timestamp)
    .bind(message.size as i64)
    .bind(content_type)
    .bind(inline_data)
    .bind(s3_key)
    .bind(version)
    .bind(sender_pk)
    .bind(statement_kind)
    .bind(batch)
    .bind(mix_number)
    .execute(pool)
    .await?;

    let message_id = result.last_insert_rowid();

    // Update board statistics
    // We don't care if these fail - they are statistics for monitoring
    let _ = update_board_statistics(pool, board_name, statement_kind).await;

    Ok(message_id)
}

/// Update board statistics after message insertion
/// This is best-effort - failures are logged but don't fail the insertion
async fn update_board_statistics(
    pool: &SqlitePool,
    board_name: &str,
    statement_kind: &str,
) -> Result<()> {
    // Count batches if this is a Ballots message
    let batch_increment = if statement_kind == "Ballots" { 1 } else { 0 };

    // Update statistics in a single query
    sqlx::query(
        r#"
        UPDATE boards 
        SET last_message_kind = ?,
            message_count = (SELECT COUNT(*) FROM messages WHERE board_name = ?),
            batch_count = batch_count + ?
        WHERE name = ?
        "#,
    )
    .bind(statement_kind)
    .bind(board_name)
    .bind(batch_increment)
    .bind(board_name)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_message(pool: &SqlitePool, board_name: &str, id: i64) -> Result<Option<Message>> {
    validate_board_name(board_name)?;
    
    let row = sqlx::query_as::<_, (i64, i64, i64, String, Option<Vec<u8>>, Option<String>, String, String, String, i32, i32)>(
        "SELECT id, timestamp, size, content_type, inline_data, s3_key, version, sender_pk, statement_kind, batch, mix_number FROM messages WHERE board_name = ? AND id = ?",
    )
    .bind(board_name)
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id, timestamp, size, content_type, inline_data, s3_key, version, sender_pk, statement_kind, batch, mix_number)| {
        let content_type = match content_type.as_str() {
            "inline" => crate::api_types::ContentType::Inline {
                data: inline_data.unwrap_or_default(),
            },
            "s3" => crate::api_types::ContentType::S3 {
                key: s3_key.unwrap_or_default(),
            },
            _ => crate::api_types::ContentType::Inline { data: vec![] },
        };

        Message {
            id: id.to_string(),
            timestamp,
            size: size as usize,
            content_type,
            sender_pk,
            statement_kind,
            batch,
            mix_number,
            version,
        }
    }))
}

pub async fn list_messages(pool: &SqlitePool, board_name: &str) -> Result<Vec<Message>> {
    validate_board_name(board_name)?;
    
    let rows = sqlx::query_as::<_, (i64, i64, i64, String, Option<Vec<u8>>, Option<String>, String, String, String, i32, i32)>(
        "SELECT id, timestamp, size, content_type, inline_data, s3_key, version, sender_pk, statement_kind, batch, mix_number FROM messages WHERE board_name = ? ORDER BY id ASC",
    )
    .bind(board_name)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, timestamp, size, content_type, inline_data, s3_key, version, sender_pk, statement_kind, batch, mix_number)| {
            let content_type = match content_type.as_str() {
                "inline" => crate::api_types::ContentType::Inline {
                    data: inline_data.unwrap_or_default(),
                },
                "s3" => crate::api_types::ContentType::S3 {
                    key: s3_key.unwrap_or_default(),
                },
                _ => crate::api_types::ContentType::Inline { data: vec![] },
            };

            Message {
                id: id.to_string(),
                timestamp,
                size: size as usize,
                content_type,
                sender_pk,
                statement_kind,
                batch,
                mix_number,
                version,
            }
        })
        .collect())
}

/// Get messages greater than last_id (for trustee synchronization)
pub async fn get_messages_after(
    pool: &SqlitePool,
    board_name: &str,
    last_id: i64,
    limit: i64,
) -> Result<(Vec<Message>, bool)> {
    validate_board_name(board_name)?;
    
    // Fetch limit + 1 to detect if there are more messages
    let rows = sqlx::query_as::<_, (i64, i64, i64, String, Option<Vec<u8>>, Option<String>, String, String, String, i32, i32)>(
        "SELECT id, timestamp, size, content_type, inline_data, s3_key, version, sender_pk, statement_kind, batch, mix_number FROM messages WHERE board_name = ? AND id > ? ORDER BY id ASC LIMIT ?",
    )
    .bind(board_name)
    .bind(last_id)
    .bind(limit + 1)
    .fetch_all(pool)
    .await?;

    let truncated = rows.len() > limit as usize;
    let messages: Vec<Message> = rows
        .into_iter()
        .take(limit as usize)
        .map(|(id, timestamp, size, content_type, inline_data, s3_key, version, sender_pk, statement_kind, batch, mix_number)| {
            let content_type = match content_type.as_str() {
                "inline" => crate::api_types::ContentType::Inline {
                    data: inline_data.unwrap_or_default(),
                },
                "s3" => crate::api_types::ContentType::S3 {
                    key: s3_key.unwrap_or_default(),
                },
                _ => crate::api_types::ContentType::Inline { data: vec![] },
            };

            Message {
                id: id.to_string(),
                timestamp,
                size: size as usize,
                content_type,
                sender_pk,
                statement_kind,
                batch,
                mix_number,
                version,
            }
        })
        .collect();

    Ok((messages, truncated))
}

/// Update board metadata when Configuration is posted
/// This is called separately from insert_message because it needs to parse the Configuration artifact
pub async fn update_board_config_metadata(
    pool: &SqlitePool,
    board_name: &str,
    cfg_id: &str,
    threshold_no: i32,
    trustees_no: i32,
) -> Result<()> {
    validate_board_name(board_name)?;
    
    sqlx::query(
        r#"UPDATE boards 
           SET cfg_id = ?, threshold_no = ?, trustees_no = ?
           WHERE name = ?"#
    )
    .bind(cfg_id)
    .bind(threshold_no)
    .bind(trustees_no)
    .bind(board_name)
    .execute(pool)
    .await?;
    
    Ok(())
}

