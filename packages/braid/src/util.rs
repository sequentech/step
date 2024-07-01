// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};

use std::fs;
use std::path::PathBuf;
use log::info;
use tracing::{event, instrument, Level};
use std::fmt::Debug;
use std::time::SystemTime;
use tokio_postgres::{NoTls, Row};
use thiserror::Error;

use strand::hash::Hash;
use strand::util::StrandError;
use board_messages::braid::statement::StatementType;

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("{0}")]
    DatalogError(String),
    #[error("{0}")]
    MissingArtifact(StatementType),
    #[error("{0}")]
    MismatchedArtifactHash(StatementType),
    #[error("{0}")]
    MessageConfigurationMismatch(String),
    #[error("{0}")]
    StrandError(#[from] strand::util::StrandError),
    #[error("{0}: {1}")]
    WrappedError(String, Box<ProtocolError>),
    #[error("{0}")]
    VerificationError(String),
    #[error("{0}")]
    SignatureVerificationError(String),
    #[error("{0}")]
    InvalidTrusteeSelection(String),
    #[error("{0}")]
    InvalidConfiguration(String),
    #[error("{0}")]
    BootstrapError(String),
    #[error("{0}")]
    BoardError(String),
    #[error("{0}")]
    BoardOverwriteAttempt(String),
    #[error("{0}")]
    InternalError(String),
}
pub trait ProtocolContext<T> {
    fn add_context(self, context: &str) -> Result<T, ProtocolError>;
}
impl<T> ProtocolContext<T> for Result<T, ProtocolError> {
    fn add_context(self, context: &str) -> Result<T, ProtocolError> {
        if let Err(e) = self {
            Err(ProtocolError::WrappedError(
                context.to_string(),
                Box::new(e),
            ))
        } else {
            self
        }
    }
}
impl<T> ProtocolContext<T> for Result<T, StrandError> {
    fn add_context(self, context: &str) -> Result<T, ProtocolError> {
        if let Err(e) = self {
            Err(ProtocolError::WrappedError(
                context.to_string(),
                Box::new(e.into()),
            ))
        } else {
            Ok(self?)
        }
    }
}

pub(crate) fn dbg_hash(h: &[u8; 64]) -> String {
    hex::encode(h)[0..10].to_string()
}

/*pub(crate) fn dbg_hashes<const N: usize>(hs: &[[u8; 64]; N]) -> String {
    hs.map(|h| hex::encode(h)[0..10].to_string()).join(" ")
}*/

pub fn hash_from_vec(bytes: &[u8]) -> Result<Hash, StrandError> {
    strand::util::to_hash_array(bytes)
}

pub fn decode_base64(s: &String) -> Result<Vec<u8>> {
    general_purpose::STANDARD_NO_PAD
        .decode(&s)
        .map_err(|error| anyhow!(error))
}

pub fn assert_folder(folder: PathBuf) -> Result<()> {
    let path = folder.as_path();
    if path.exists() {
        if path.is_dir() {
            Ok(())
        } else {
            Err(anyhow!("Path is not a folder: {}", path.display()))
        }
    } else {
        fs::create_dir(path).map_err(|err| anyhow!(err))
    }
}



#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardMessage {
    pub id: i32,
    pub created: SystemTime,
    // Base64 encoded spki der representation.
    pub sender_pk: String,
    pub statement_timestamp: SystemTime,
    pub statement_kind: String,
    pub message: Vec<u8>,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct Board {
    pub id: i32,
    pub board_name: String,
    pub is_archived: bool,
}

impl TryFrom<&Row> for BoardMessage {
    type Error = anyhow::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        let id = row.get("id");
        let created = row.get("created");
        let sender_pk = row.get("sender_pk");
        let statement_timestamp = row.get("statement_timestamp");
        let statement_kind = row.get("statement_kind");
        let message = row.get("message");
        let version = row.get("version");

        Ok(BoardMessage {
            id,
            created,
            sender_pk,
            statement_timestamp,
            statement_kind,
            message,
            version,
        })
    }
}

impl TryFrom<&Row> for Board {
    type Error = anyhow::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        let id = row.get("id");
        let board_name = row.get("board_name");
        let is_archived = row.get("is_archived");

        Ok(Board {
            id,
            board_name,
            is_archived,
        })
    }
}
// Run ignored tests with
// cargo test <test_name> -- --include-ignored
#[cfg(test)]
pub(crate) mod tests {
    use std::alloc::System;

    use super::*;
    use serial_test::serial;

    const PG_DATABASE: &'static str = "protocoldb";
    const PG_HOST: &'static str = "postgres";
    const PG_USER: &'static str = "postgres";
    const PG_PASSW: &'static str = "postgrespassword";
    const PG_PORT: u32 = 5432;
    const TEST_BOARD: &'static str = "testboard";
    
    
    // We cannot use create_database_and_index because we additionally drop the database here
    async fn set_up() -> BoardClient {
        let connection_string = format!("host={} port={} user={} password={}", PG_HOST, PG_PORT, PG_USER, PG_PASSW);
        let (client, connection) =
        tokio_postgres::connect(&connection_string, NoTls).await.unwrap();

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });
        
        client.execute(&format!("DROP DATABASE IF EXISTS {}", PG_DATABASE), &[]).await.unwrap();
        client.execute(&format!("CREATE DATABASE {}", PG_DATABASE), &[]).await.unwrap();

        drop(client);

        let mut client = BoardClient::new(PG_HOST, PG_PORT, PG_USER, PG_PASSW, PG_DATABASE).await.unwrap();
        client.create_index_ine().await.unwrap();

        client
    }
    
    #[tokio::test]
    #[ignore]
    #[serial]
    async fn test_create_delete_board() {
        // let mut client = BoardClient::new(PG_HOST, PG_PORT, PG_USER, PG_PASSW, PG_DATABASE).await.unwrap();
        let mut client = set_up().await;
        client.create_board_ine(TEST_BOARD).await.unwrap();
        let board = client.get_board(TEST_BOARD).await.unwrap();
        assert_eq!(board.board_name, TEST_BOARD);
        let board = client.get_board("NOT FOUND").await;
        assert!(board.is_err());
        client.delete_board(TEST_BOARD).await.unwrap();
        let board = client.get_board(TEST_BOARD).await;
        assert!(board.is_err());

    }

    #[tokio::test]
    #[ignore]
    #[serial]
    pub async fn test_message_create_retrieve() {
        let mut client = set_up().await;
        client.create_board_ine(TEST_BOARD).await.unwrap();
        let board = client.get_board(TEST_BOARD).await.unwrap();
        assert_eq!(board.board_name, TEST_BOARD);
        let board_message = BoardMessage {
            id: 1,
            created: SystemTime::now(),
            sender_pk: "".to_string(),
            statement_timestamp: SystemTime::now(),
            statement_kind: "".to_string(),
            message: vec![],
            version: "".to_string(),
        };
        let messages = vec![board_message.clone()];
        client.insert_messages(TEST_BOARD, &messages).await.unwrap();
        
        let ret = client.get_messages(TEST_BOARD, 0).await.unwrap();
        assert_eq!(messages.len(), 1);
        let msg = ret.get(0).unwrap();
        // id is autogenerated by postgres
        // timestamps will not match due to less precision on postgres side
        assert_eq!(msg.sender_pk, board_message.sender_pk);
        assert_eq!(msg.statement_kind, board_message.statement_kind);
        assert_eq!(msg.message, board_message.message);
        assert_eq!(msg.version, board_message.version);
    }
}



const INDEX_TABLE: &'static str = "BULLETIN_BOARDS";
const PG_DEFAULT_ENTRIES_TX_LIMIT: usize = 50;
const PG_DEFAULT_OFFSET: usize = 0;
const PG_DEFAULT_LIMIT: usize = 2500;


pub struct BoardClient {
    client: tokio_postgres::Client,
}

impl BoardClient {
    /// Creates a new BoardClient. The underlying connection will be closed when the client is dropped.
    pub async fn new(host: &str, port: u32, username: &str, password: &str, dbname: &str) -> Result<BoardClient> {
        let connection_string = format!("host={} port={} user={} password={} dbname={}", host, port, username, password, dbname);
        let (client, connection) =
        tokio_postgres::connect(&connection_string, NoTls).await?;

        // The connection object performs the actual communication with the database,
        // so spawn it off to run on its own.
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        let ret = BoardClient {
            client
        };

        Ok(ret)
    }

    /// Creates the index table if it doesn't exist.
    #[instrument(skip(self))]
    pub async fn create_index_ine(&mut self) -> Result<()> {
        let transaction = self.client.transaction().await?;
        transaction.execute(
            &format!(r#"
            CREATE TABLE IF NOT EXISTS {} (
                id SERIAL PRIMARY KEY,
                board_name VARCHAR,
                is_archived BOOLEAN
            );
            "#,
            INDEX_TABLE),
            &[]
        ).await?;
        transaction.execute(
            &format!(
            r#"
            CREATE UNIQUE INDEX IF NOT EXISTS BOARD_NAME_IDX ON {}(board_name);
            "#,
            INDEX_TABLE),
            &[]
        )
        .await?;
        transaction.commit().await?;

        Ok(())
    }

    /// Creates the requested board table and adds it to the index, if it doesn't exist.
    #[instrument(skip(self))]
    pub async fn create_board_ine(&mut self, board: &str) -> Result<()> {
        let transaction = self.client.transaction().await?;
        transaction.execute(
            &format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                id SERIAL PRIMARY KEY,
                created TIMESTAMP,
                sender_pk VARCHAR,
                statement_timestamp TIMESTAMP,
                statement_kind VARCHAR,
                message BYTEA,
                version VARCHAR
            );
            "#,
            board),
            &[]
        ).await?;

        let message_sql = r#"
            INSERT INTO bulletin_boards(
                board_name,
                is_archived
            ) VALUES (
                $1,
                $2
            ) ON CONFLICT (board_name) DO NOTHING;
        "#;
        transaction.execute(message_sql, &[&board, &false]).await?;
        transaction.commit().await?;
        Ok(())
        
    }

    /// Gets the requested board from the index.
    pub async fn get_board(&mut self, board_name: &str) -> Result<Board> {
        let message_sql = format!(r#"
        SELECT
            id,
            board_name,
            is_archived
        FROM {}
        WHERE board_name = $1;
        "#,
        INDEX_TABLE);
        
        let sql_query_response = self.client.query(&message_sql, &[&board_name]).await?;
        let boards = sql_query_response
            .iter()
            .map(Board::try_from)
            .collect::<Result<Vec<Board>>>()?;

        if boards.len() > 0 {
            Ok(boards[0].clone())
        } else {
            Err(anyhow!("board name '{}' not found", board_name))
        }
    }

    /// Get all messages whose id is bigger than `last_id`.
    pub async fn get_messages(
        &mut self,
        board_name: &str,
        last_id: i32,
    ) -> Result<Vec<BoardMessage>> {
        let mut offset: usize = 0;
        let mut last_batch = self
            .get(
                board_name,
                last_id,
                Some(PG_DEFAULT_LIMIT),
                Some(offset),
            )
            .await?;
        let mut messages = last_batch.clone();
        while PG_DEFAULT_LIMIT == last_batch.len() {
            offset += last_batch.len();
            last_batch = self
                .get(
                    board_name,
                    last_id,
                    Some(PG_DEFAULT_LIMIT),
                    Some(offset),
                )
                .await?;
            messages.extend(last_batch.clone());
        }
        Ok(messages)
    }

    async fn get(
        &mut self,
        board: &str,
        last_id: i32,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<BoardMessage>> {
    
        let sql = format!(
            r#"
        SELECT
            id,
            created,
            sender_pk,
            statement_timestamp,
            statement_kind,
            message,
            version
        FROM {}
        WHERE id > $1
        ORDER BY id
        LIMIT {}
        OFFSET {};
        "#,
            board,
            limit.unwrap_or(PG_DEFAULT_LIMIT),
            offset.unwrap_or(PG_DEFAULT_OFFSET),
        );

        let sql_query_response = self.client.query(&sql, &[&last_id]).await?;
        let messages = sql_query_response
            .iter()
            .map(BoardMessage::try_from)
            .collect::<Result<Vec<BoardMessage>>>()?;

        Ok(messages)
    }

    /// Get all boards in the index
    pub async fn get_boards(&mut self) -> Result<Vec<Board>> {
        let sql = format!(
            r#"
        SELECT
            id,
            board_name,
            is_archived
        FROM bulletin_boards
        WHERE is_archived = {}
        "#,
            false
        );
        let sql_query_response = self.client.query(&sql, &[]).await?;
        let boards = sql_query_response
            .iter()
            .map(Board::try_from)
            .collect::<Result<Vec<Board>>>()?;

        Ok(boards)
    }

    /// Inserts messages into the requested board table.
    pub async fn insert_messages(
        &mut self,
        board_name: &str,
        messages: &Vec<BoardMessage>,
    ) -> Result<()> {
        for chunk in messages.chunks(PG_DEFAULT_ENTRIES_TX_LIMIT) {
            let chunk_vec: Vec<BoardMessage> = chunk.to_vec();
            self.insert(board_name, &chunk_vec)
                .await?;
        }
        Ok(())
    }

    async fn insert(
        &mut self,
        board_name: &str,
        messages: &Vec<BoardMessage>,
    ) -> Result<()> {
        info!("Insert {} messages..", messages.len());
        
        // Start a new transaction
        let transaction = self.client.transaction().await?;

        for message in messages {
            let message_sql = format!(
                r#"
                INSERT INTO {} (
                    created,
                    sender_pk,
                    statement_timestamp,
                    statement_kind,
                    message,
                    version
                ) VALUES (
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    $6
                );
            "#,
                board_name
            );
            
            transaction.execute(&message_sql, 
                &[
                    &message.created, 
                    &message.sender_pk,
                    &message.statement_timestamp,
                    &message.statement_kind,
                    &message.message,
                    &message.version
                ]
            )
            .await?;
            
        }

        transaction.commit().await?;

        Ok(())
    }

    /// Get one messages matching id.
    pub async fn get_one_message(
        &mut self,
        board_name: &str,
        id: i64,
    ) -> Result<Option<BoardMessage>> {
        self.get_one(board_name, id).await
    }

    async fn get_one(
        &mut self,
        board_name: &str,
        id: i64,
    ) -> Result<Option<BoardMessage>> {
        let sql = format!(
            r#"
        SELECT
            id,
            created,
            sender_pk,
            statement_timestamp,
            statement_kind,
            message,
            version
        FROM {}
        WHERE id = @id
        "#,
            board_name
        );

        let rows = self.client.query(&sql, &[&id]).await?;

        if rows.len() > 0 {
            Ok(Some(BoardMessage::try_from(&rows[0])?))
        } else {
            Ok(None)
        }
    }

    /// Deletes the requested board table and removes it from the index.
    #[instrument(skip(self))]
    pub async fn delete_board(&mut self, board_name: &str) -> Result<()> {
        let transaction = self.client.transaction().await?;
        let message_sql = format!(r#"
            DELETE from {} where 
            board_name = $1
            AND
            is_archived = $2;
        "#,
        INDEX_TABLE);

        transaction.execute(&message_sql, &[&board_name, &false]).await?;
        transaction.execute(&format!("DROP TABLE IF EXISTS {};", board_name), &[]).await?;

        transaction.commit().await?;

        Ok(())
    }

    /// Clears all data in the database.
    pub async fn clear_database(&mut self) -> Result<()> {
        let transaction = self.client.transaction().await?;
        transaction.execute("drop schema if exists public cascade;", &[]).await?;
        transaction.execute("create schema if not exists public;", &[]).await?;
        transaction.commit().await?;
        Ok(())
    }
    
    pub async fn create_database_and_index(&mut self, host: &str, port: u32, username: &str, password: &str, dbname: &str) -> Result<BoardClient> {
        let connection_string = format!("host={} port={} user={} password={}", host, port, username, password);
        let (client, connection) =
        tokio_postgres::connect(&connection_string, NoTls).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });
        
        client.execute(&format!("CREATE DATABASE {}", dbname), &[]).await?;
        drop(client);

        let mut client = BoardClient::new(host, port, username, password, dbname).await?;
        client.create_index_ine().await?;

        Ok(client)
    }
}