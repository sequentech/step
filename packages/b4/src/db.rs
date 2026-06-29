// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::api_types::Message;
use anyhow::{anyhow, Context, Result};
use bb8_postgres::{bb8::Pool, PostgresConnectionManager};
use std::env;
use tokio_postgres::NoTls;

/// PostgreSQL connection pool type alias
pub type DbPool = Pool<PostgresConnectionManager<NoTls>>;

#[derive(Debug, Clone)]
pub struct Board {
    pub name: String,
    pub created_at: i64,
    pub status: String,
}

/// PostgreSQL connection parameters
#[derive(Clone)]
pub struct PgConnectionParams {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
}

impl PgConnectionParams {
    pub fn new(host: &str, port: u16, username: &str, password: &str, database: &str) -> Self {
        Self {
            host: host.to_string(),
            port,
            username: username.to_string(),
            password: password.to_string(),
            database: database.to_string(),
        }
    }

    pub fn from_env() -> Result<Self> {
        let host = env::var("B4_PG_HOST").context("B4_PG_HOST must be set")?;
        let port: u16 = env::var("B4_PG_PORT")
            .context("B4_PG_PORT must be set")?
            .parse()
            .context("B4_PG_PORT must be a valid port number")?;
        let username = env::var("B4_PG_USER").context("B4_PG_USER must be set")?;
        let password = env::var("B4_PG_PASSWORD").context("B4_PG_PASSWORD must be set")?;
        let database = env::var("B4_PG_DATABASE").context("B4_PG_DATABASE must be set")?;

        Ok(Self {
            host,
            port,
            username,
            password,
            database,
        })
    }

    pub fn connection_string(&self) -> String {
        format!(
            "host={} port={} user={} password={} dbname={}",
            self.host, self.port, self.username, self.password, self.database
        )
    }
}

pub async fn init_db() -> Result<DbPool> {
    let params = PgConnectionParams::from_env()?;
    init_db_with_params(&params).await
}

pub async fn init_db_with_params(params: &PgConnectionParams) -> Result<DbPool> {
    tracing::info!(
        "Connecting to PostgreSQL database at {}:{}",
        params.host,
        params.port
    );

    let manager =
        PostgresConnectionManager::new_from_stringlike(params.connection_string(), NoTls)?;

    let pool = Pool::builder().max_size(5).build(manager).await?;

    // Create tables in a scoped block so connection is dropped before returning pool
    {
        let conn = pool.get().await?;

        // Create boards table (Superset of INDEX and boards)
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS boards (
                id SERIAL UNIQUE,
                board_name VARCHAR PRIMARY KEY,
                created_at TIMESTAMP DEFAULT NOW(),
                is_archived BOOLEAN DEFAULT FALSE,
                status VARCHAR DEFAULT 'active',
                cfg_id VARCHAR,
                threshold_no INTEGER,
                trustees_no INTEGER,
                last_message_kind VARCHAR,
                last_updated TIMESTAMP,
                message_count INTEGER DEFAULT 0,
                batch_count INTEGER DEFAULT 0
            )
            "#,
            &[],
        )
        .await?;

        // Create messages table (Superset + Partitioned)
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS messages (
                id BIGSERIAL,
                board_name VARCHAR NOT NULL,
                -- b4/db.rs specific columns
                timestamp BIGINT, 
                size BIGINT,
                content_type VARCHAR,
                inline_data BYTEA,
                s3_key VARCHAR,
                -- pgsql.rs specific columns
                created TIMESTAMP,
                statement_timestamp TIMESTAMP,
                message BYTEA,
                -- Common columns
                sender_pk VARCHAR NOT NULL,
                statement_kind VARCHAR NOT NULL,
                batch INTEGER NOT NULL DEFAULT 0,
                mix_number INTEGER NOT NULL DEFAULT 0,
                version VARCHAR NOT NULL,
                
                PRIMARY KEY (board_name, id),
                UNIQUE (board_name, sender_pk, statement_kind, batch, mix_number)
            ) PARTITION BY LIST (board_name);
            "#,
            &[],
        )
        .await?;

        // Create index for efficient range queries
        conn.execute(
            r#"
            CREATE INDEX IF NOT EXISTS idx_messages_board_id 
            ON messages(board_name, id)
            "#,
            &[],
        )
        .await?;
    }

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
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        anyhow::bail!("Board name contains invalid characters (only alphanumeric, -, _ allowed)");
    }
    Ok(())
}

pub async fn create_board(pool: &DbPool, name: &str) -> Result<Board> {
    validate_board_name(name)?;

    let now_chrono = chrono::Utc::now();
    let created_at = now_chrono.timestamp();
    // tokio-postgres requires SystemTime (chrono feature not strictly enabled)
    let created_at_sql: std::time::SystemTime = now_chrono.into();

    let mut conn = pool.get().await?;

    // Create transaction to insert board and create partition
    let transaction = conn.transaction().await?;

    transaction
        .execute(
            r#"
        INSERT INTO boards (board_name, created_at, status, last_updated)
        VALUES ($1, $2, 'active', $2)
        "#,
            &[&name, &created_at_sql],
        )
        .await?;

    // Create partition
    let partition_name = format!("messages_{}", name);
    transaction
        .execute(
            &format!(
                "CREATE TABLE IF NOT EXISTS \"{}\" PARTITION OF messages FOR VALUES IN ('{}')",
                partition_name, name
            ),
            &[],
        )
        .await?;

    transaction.commit().await?;

    Ok(Board {
        name: name.to_string(),
        created_at,
        status: "active".to_string(),
    })
}

pub async fn get_board(pool: &DbPool, name: &str) -> Result<Option<Board>> {
    let conn = pool.get().await?;
    let row = conn
        .query_opt(
            "SELECT board_name, EXTRACT(EPOCH FROM created_at)::BIGINT, status FROM boards WHERE board_name = $1",
            &[&name],
        )
        .await?;

    Ok(row.map(|r| Board {
        name: r.get(0),
        created_at: r.get(1),
        status: r.get(2),
    }))
}

pub async fn list_boards(pool: &DbPool) -> Result<Vec<Board>> {
    let conn = pool.get().await?;
    let rows = conn
        .query(
            "SELECT board_name, EXTRACT(EPOCH FROM created_at)::BIGINT, status FROM boards ORDER BY created_at DESC",
            &[],
        )
        .await?;

    Ok(rows
        .into_iter()
        .map(|r| Board {
            name: r.get(0),
            created_at: r.get(1),
            status: r.get(2),
        })
        .collect())
}

pub async fn insert_message(
    pool: &DbPool,
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

    let conn = pool.get().await?;
    let row = conn.query_one(
        r#"
        INSERT INTO messages (
            board_name, timestamp, size, content_type, inline_data, s3_key, version, sender_pk, statement_kind, batch, mix_number,
            created, statement_timestamp, message
        )
        VALUES (
            $1, $2::BIGINT, $3, $4, $5, $6, $7, $8, $9, $10, $11,
            TO_TIMESTAMP($2::DOUBLE PRECISION), -- created
            TO_TIMESTAMP($2::DOUBLE PRECISION), -- statement_timestamp
            $5 -- message (copy of inline_data)
        )
        RETURNING id
        "#,
        &[
            &board_name,
            &message.timestamp,
            &(message.size as i64),
            &content_type,
            &inline_data,
            &s3_key,
            &version,
            &sender_pk,
            &statement_kind,
            &batch,
            &mix_number,
        ],
    )
    .await?;

    let message_id: i64 = row.get(0);

    // Update board statistics (similar to b3's insert() function)
    // We don't care if these fail - they are statistics for monitoring
    let _ = update_board_statistics(pool, board_name, statement_kind).await;

    Ok(message_id)
}

/// Update board statistics after message insertion (like b3's INDEX table updates)
/// This is best-effort - failures are logged but don't fail the insertion
async fn update_board_statistics(
    pool: &DbPool,
    board_name: &str,
    statement_kind: &str,
) -> Result<()> {
    // Count batches if this is a Ballots message
    let batch_increment = if statement_kind == "Ballots" { 1 } else { 0 };

    let conn = pool.get().await?;
    // Update statistics in a single query
    conn.execute(
        r#"
        UPDATE boards 
        SET last_message_kind = $1,
            message_count = (SELECT COUNT(*) FROM messages WHERE board_name = $2),
            batch_count = batch_count + $3
        WHERE board_name = $2
        "#,
        &[&statement_kind, &board_name, &batch_increment],
    )
    .await?;

    Ok(())
}

pub async fn get_message(pool: &DbPool, board_name: &str, id: i64) -> Result<Option<Message>> {
    validate_board_name(board_name)?;

    let conn = pool.get().await?;
    let row = conn
        .query_opt(
            "SELECT id, timestamp, size, content_type, inline_data, s3_key, version, sender_pk, statement_kind, batch, mix_number, message FROM messages WHERE board_name = $1 AND id = $2",
            &[&board_name, &id],
        )
        .await?;

    Ok(row.map(|r| {
        let id: i64 = r.get(0);
        let timestamp: i64 = r.try_get(1).unwrap_or_default();
        let _version: String = r.try_get(6).unwrap_or_default();
        let sender_pk: String = r.get(7);
        let statement_kind: String = r.get(8);
        let batch: i32 = r.get(9);
        let mix_number: i32 = r.get(10);
        let inline_data: Option<Vec<u8>> = r.get(4);
        let s3_key: Option<String> = r.get(5);
        let pg_message: Option<Vec<u8>> = r.try_get(11).unwrap_or_default();

        let size: i64 = match r.try_get::<_, Option<i64>>(2) {
            Ok(Some(s)) if s > 0 => s,
            _ => {
                if let Some(ref d) = inline_data {
                    d.len() as i64
                } else if let Some(ref m) = pg_message {
                    m.len() as i64
                } else {
                    0
                }
            }
        };

        let content_type_str: Option<String> = r.try_get(3).unwrap_or_default();
        let content_type = match content_type_str.as_deref() {
            Some("inline") => crate::api_types::ContentType::Inline {
                data: inline_data.unwrap_or_default(),
            },
            Some("s3") => crate::api_types::ContentType::S3 {
                key: s3_key.unwrap_or_default(),
            },
            _ => crate::api_types::ContentType::Inline {
                data: pg_message.unwrap_or_else(|| inline_data.unwrap_or_default()),
            },
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
        }
    }))
}

pub async fn list_messages(pool: &DbPool, board_name: &str) -> Result<Vec<Message>> {
    validate_board_name(board_name)?;

    let conn = pool.get().await?;
    let rows = conn
        .query(
            "SELECT id, timestamp, size, content_type, inline_data, s3_key, version, sender_pk, statement_kind, batch, mix_number, message FROM messages WHERE board_name = $1 ORDER BY id ASC",
            &[&board_name],
        )
        .await?;

    Ok(rows
        .into_iter()
        .map(|r| {
            let id: i64 = r.get(0);
            let timestamp: i64 = r.try_get(1).unwrap_or_default();
            let _version: String = r.try_get(6).unwrap_or_default();
            let sender_pk: String = r.get(7);
            let statement_kind: String = r.get(8);
            let batch: i32 = r.get(9);
            let mix_number: i32 = r.get(10);
            let inline_data: Option<Vec<u8>> = r.get(4);
            let s3_key: Option<String> = r.get(5);
            let pg_message: Option<Vec<u8>> = r.try_get(11).unwrap_or_default();

            let size: i64 = match r.try_get::<_, Option<i64>>(2) {
                Ok(Some(s)) if s > 0 => s,
                _ => {
                    if let Some(ref d) = inline_data {
                        d.len() as i64
                    } else if let Some(ref m) = pg_message {
                        m.len() as i64
                    } else {
                        0
                    }
                }
            };

            let content_type_str: Option<String> = r.try_get(3).unwrap_or_default();
            let content_type = match content_type_str.as_deref() {
                Some("inline") => crate::api_types::ContentType::Inline {
                    data: inline_data.unwrap_or_default(),
                },
                Some("s3") => crate::api_types::ContentType::S3 {
                    key: s3_key.unwrap_or_default(),
                },
                _ => crate::api_types::ContentType::Inline {
                    data: pg_message.unwrap_or_else(|| inline_data.unwrap_or_default()),
                },
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
            }
        })
        .collect())
}

/// Get messages greater than last_id (for trustee synchronization)
pub async fn get_messages_after(
    pool: &DbPool,
    board_name: &str,
    last_id: i64,
    limit: i64,
) -> Result<(Vec<Message>, bool)> {
    validate_board_name(board_name)?;

    let conn = pool.get().await?;
    // Fetch limit + 1 to detect if there are more messages
    let rows = conn.query(
        "SELECT id, timestamp, size, content_type, inline_data, s3_key, version, sender_pk, statement_kind, batch, mix_number, message FROM messages WHERE board_name = $1 AND id > $2 ORDER BY id ASC LIMIT $3",
        &[&board_name, &last_id, &(limit + 1)],
    )
    .await?;

    let truncated = rows.len() > limit as usize;
    let messages: Vec<Message> = rows
        .into_iter()
        .take(limit as usize)
        .map(|r| {
            let id: i64 = r.get(0);
            let timestamp: i64 = r.try_get(1).unwrap_or_default();
            let _version: String = r.try_get(6).unwrap_or_default();
            let sender_pk: String = r.get(7);
            let statement_kind: String = r.get(8);
            let batch: i32 = r.get(9);
            let mix_number: i32 = r.get(10);
            let inline_data: Option<Vec<u8>> = r.get(4);
            let s3_key: Option<String> = r.get(5);
            let pg_message: Option<Vec<u8>> = r.try_get(11).unwrap_or_default();

            let size: i64 = match r.try_get::<_, Option<i64>>(2) {
                Ok(Some(s)) if s > 0 => s,
                _ => {
                    if let Some(ref d) = inline_data {
                        d.len() as i64
                    } else if let Some(ref m) = pg_message {
                        m.len() as i64
                    } else {
                        0
                    }
                }
            };

            let content_type_str: Option<String> = r.try_get(3).unwrap_or_default();
            let content_type = match content_type_str.as_deref() {
                Some("inline") => crate::api_types::ContentType::Inline {
                    data: inline_data.unwrap_or_default(),
                },
                Some("s3") => crate::api_types::ContentType::S3 {
                    key: s3_key.unwrap_or_default(),
                },
                _ => crate::api_types::ContentType::Inline {
                    data: pg_message.unwrap_or_else(|| inline_data.unwrap_or_default()),
                },
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
            }
        })
        .collect();

    Ok((messages, truncated))
}

/// Update board metadata when Configuration is posted (similar to b3's update_index)
/// This is called separately from insert_message because it needs to parse the Configuration artifact
pub async fn update_board_config_metadata(
    pool: &DbPool,
    board_name: &str,
    cfg_id: &str,
    threshold_no: i32,
    trustees_no: i32,
) -> Result<()> {
    validate_board_name(board_name)?;

    let conn = pool.get().await?;
    conn.execute(
        r#"UPDATE boards 
           SET cfg_id = $1, threshold_no = $2, trustees_no = $3
           WHERE board_name = $4"#,
        &[&cfg_id, &threshold_no, &trustees_no, &board_name],
    )
    .await?;

    Ok(())
}
