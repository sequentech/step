use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use wbraid_shared::Message;
use std::env;
use std::path::PathBuf;

pub async fn init_db() -> Result<SqlitePool> {
    // Use DATABASE_URL env var if set, otherwise default to wbraid.db in current directory
    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        let mut path = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        path.push("wbraid.db");
        // Add mode=rwc to create the file if it doesn't exist
        format!("sqlite:{}?mode=rwc", path.display())
    });
    
    tracing::info!("Connecting to database: {}", db_url);
    
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // Create messages table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            timestamp INTEGER NOT NULL,
            size INTEGER NOT NULL,
            content_type TEXT NOT NULL,
            inline_data BLOB,
            s3_key TEXT
        )
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

pub async fn insert_message(
    pool: &SqlitePool,
    message: &Message,
    inline_data: Option<&[u8]>,
    s3_key: Option<&str>,
) -> Result<()> {
    let content_type = match &message.content_type {
        wbraid_shared::ContentType::Inline { .. } => "inline",
        wbraid_shared::ContentType::S3 { .. } => "s3",
    };

    sqlx::query(
        r#"
        INSERT INTO messages (id, timestamp, size, content_type, inline_data, s3_key)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&message.id)
    .bind(message.timestamp)
    .bind(message.size as i64)
    .bind(content_type)
    .bind(inline_data)
    .bind(s3_key)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_message(pool: &SqlitePool, id: &str) -> Result<Option<Message>> {
    let row = sqlx::query_as::<_, (String, i64, i64, String, Option<Vec<u8>>, Option<String>)>(
        "SELECT id, timestamp, size, content_type, inline_data, s3_key FROM messages WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id, timestamp, size, content_type, inline_data, s3_key)| {
        let content_type = match content_type.as_str() {
            "inline" => wbraid_shared::ContentType::Inline {
                data: inline_data.unwrap_or_default(),
            },
            "s3" => wbraid_shared::ContentType::S3 {
                key: s3_key.unwrap_or_default(),
            },
            _ => wbraid_shared::ContentType::Inline { data: vec![] },
        };

        Message {
            id,
            timestamp,
            size: size as usize,
            content_type,
        }
    }))
}

pub async fn list_messages(pool: &SqlitePool) -> Result<Vec<Message>> {
    let rows = sqlx::query_as::<_, (String, i64, i64, String, Option<Vec<u8>>, Option<String>)>(
        "SELECT id, timestamp, size, content_type, inline_data, s3_key FROM messages ORDER BY timestamp DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, timestamp, size, content_type, inline_data, s3_key)| {
            let content_type = match content_type.as_str() {
                "inline" => wbraid_shared::ContentType::Inline {
                    data: inline_data.unwrap_or_default(),
                },
                "s3" => wbraid_shared::ContentType::S3 {
                    key: s3_key.unwrap_or_default(),
                },
                _ => wbraid_shared::ContentType::Inline { data: vec![] },
            };

            Message {
                id,
                timestamp,
                size: size as usize,
                content_type,
            }
        })
        .collect())
}
