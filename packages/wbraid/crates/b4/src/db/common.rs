// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::utils::dispatch_db;
use crate::api_types::{ContentType, MessageBlob};
use anyhow::{anyhow, bail, Context, Result};
use sqlx::Database;

#[derive(Debug, Clone)]
pub struct Board {
    pub name: String,
    pub created_at: i64,
    pub status: String,
}

#[derive(Clone)]
pub enum BoardDb {
    #[cfg(feature = "sqlite")]
    Sqlite(super::sqlite::SqliteBackend),
    #[cfg(feature = "postgres")]
    Postgres(super::postgres::PostgresBackend),
}

impl BoardDb {
    /// Open the matching board db, init'ed and ready to use.
    pub async fn open_url(url: &str) -> Result<Self> {
        let scheme = url
            .split_once(':')
            .map(|(scheme, _)| scheme)
            .ok_or_else(|| anyhow!("missing scheme in database url"))?;
        tracing::debug!("Opening database url '{url}'");

        #[cfg(feature = "sqlite")]
        {
            if sqlx::Sqlite::URL_SCHEMES.contains(&scheme) {
                let backend = super::sqlite::SqliteBackend::open(url).await?;
                return Ok(BoardDb::Sqlite(backend));
            }
        }
        #[cfg(feature = "postgres")]
        {
            if sqlx::Postgres::URL_SCHEMES.contains(&scheme) {
                let backend = super::postgres::PostgresBackend::open(url).await?;
                return Ok(BoardDb::Postgres(backend));
            }
        }

        bail!("unsupported database url scheme: {scheme}")
    }

    pub async fn create_board(&self, name: &str) -> Result<Board> {
        dispatch_db!(self, create_board(name))
    }

    pub async fn get_board(&self, name: &str) -> Result<Option<Board>> {
        dispatch_db!(self, get_board(name))
    }

    pub async fn list_boards(&self) -> Result<Vec<Board>> {
        dispatch_db!(self, list_boards())
    }

    pub async fn insert_message(
        &self,
        board_name: &str,
        inline_data: Option<&[u8]>,
        s3_key: Option<&str>,
        version: &str,
    ) -> Result<i64> {
        dispatch_db!(
            self,
            insert_message(board_name, inline_data, s3_key, version)
        )
    }

    pub async fn get_message(&self, board_name: &str, id: i64) -> Result<Option<MessageBlob>> {
        dispatch_db!(self, get_message(board_name, id))
    }

    pub async fn list_messages(&self, board_name: &str) -> Result<Vec<MessageBlob>> {
        dispatch_db!(self, list_messages(board_name))
    }

    pub async fn get_messages_after(
        &self,
        board_name: &str,
        last_id: i64,
        limit: i64,
    ) -> Result<(Vec<MessageBlob>, bool)> {
        dispatch_db!(self, get_messages_after(board_name, last_id, limit))
    }
}

pub async fn open_from_env() -> Result<BoardDb> {
    let url = std::env::var("DATABASE_URL").context("No DATABASE_URL set")?;
    BoardDb::open_url(&url).await
}

/// Build the API `MessageBlob` from a stored row.
pub(super) fn row_to_message(
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

/// Validates board name to prevent path traversal and SQL injection.
pub(super) fn validate_board_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("Board name cannot be empty");
    }
    if name.len() > 255 {
        bail!("Board name too long (max 255 characters)");
    }
    // Only allow alphanumeric, hyphens, underscores
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        bail!("Board name contains invalid characters (only alphanumeric, -, _ allowed)");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
