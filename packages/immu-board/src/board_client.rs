// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use tracing::{info, instrument};

use immudb_rs::{sql_value::Value, Client, NamedParam, Row, SqlValue, TxMode};
use std::fmt::Debug;
use tokio::time::{sleep, Duration};

const IMMUDB_DEFAULT_LIMIT: usize = 900;
const IMMUDB_DEFAULT_ENTRIES_TX_LIMIT: usize = 50;
const IMMUDB_DEFAULT_OFFSET: usize = 0;
const ELECTORAL_LOG_TABLE: &'static str = "electoral_log_messages";

#[derive(Debug, Clone)]
enum Table {
    BraidMessages,
    ElectoralLogMessages,
}

impl Table {
    fn as_str(&self) -> &'static str {
        match self {
            Table::BraidMessages => "braid_messages",
            Table::ElectoralLogMessages => "electoral_log_messages",
        }
    }
}

#[derive(Debug)]
pub struct BoardClient {
    client: Client,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElectoralLogMessage {
    pub id: i64,
    pub created: i64,
    pub sender_pk: String,
    pub statement_timestamp: i64,
    pub statement_kind: String,
    pub message: Vec<u8>,
    pub version: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
}

impl TryFrom<&Row> for ElectoralLogMessage {
    type Error = anyhow::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        let mut id = 0;
        let mut created = 0;
        let mut sender_pk = String::from("");
        let mut statement_timestamp = 0;
        let mut statement_kind = String::from("");
        let mut message = vec![];
        let mut version = String::from("");
        let mut user_id: Option<String> = None;
        let mut username: Option<String> = None;

        for (column, value) in row.columns.iter().zip(row.values.iter()) {
            // FIXME for some reason columns names appear with parentheses
            let dot = column
                .find('.')
                .ok_or(anyhow!("invalid column found '{}'", column.as_str()))?;
            let bare_column = &column[dot + 1..column.len() - 1];

            match bare_column {
                "id" => assign_value!(Value::N, value, id),
                "created" => assign_value!(Value::Ts, value, created),
                "sender_pk" => assign_value!(Value::S, value, sender_pk),
                "statement_timestamp" => {
                    assign_value!(Value::Ts, value, statement_timestamp)
                }
                "statement_kind" => assign_value!(Value::S, value, statement_kind),
                "message" => assign_value!(Value::Bs, value, message),
                "version" => assign_value!(Value::S, value, version),
                "user_id" => match value.value.as_ref() {
                    Some(Value::S(inner)) => user_id = Some(inner.clone()),
                    None => user_id = None,
                    _ => return Err(anyhow!("invalid column value for 'userId'")),
                },
                "username" => match value.value.as_ref() {
                    Some(Value::S(inner)) => username = Some(inner.clone()),
                    None => username = None,
                    _ => return Err(anyhow!("invalid column value for 'username'")),
                },
                _ => return Err(anyhow!("invalid column found '{}'", bare_column)),
            }
        }

        Ok(ElectoralLogMessage {
            id,
            created,
            sender_pk,
            statement_timestamp,
            statement_kind,
            message,
            version,
            user_id,
            username,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardMessage {
    pub id: i64,
    pub created: i64,
    // Base64 encoded spki der representation.
    pub sender_pk: String,
    pub statement_timestamp: i64,
    pub statement_kind: String,
    pub message: Vec<u8>,
    pub version: String,
}

impl BoardClient {
    #[instrument(skip(password), level = "trace")]
    pub async fn new(server_url: &str, username: &str, password: &str) -> Result<BoardClient> {
        let mut client = Client::new(&server_url, username, password).await?;
        client.login().await?;

        Ok(BoardClient { client: client })
    }

    /// Get all electoral log messages whose id is bigger than `last_id`
    pub async fn get_electoral_log_messages(
        &mut self,
        board_db: &str,
    ) -> Result<Vec<ElectoralLogMessage>> {
        let mut offset: usize = 0;
        let mut last_batch = self
            .get_electoral_log_messages_from_db(
                board_db,
                0,
                Some(IMMUDB_DEFAULT_LIMIT),
                Some(offset),
            )
            .await?;
        let mut messages = last_batch.clone();
        while IMMUDB_DEFAULT_LIMIT == last_batch.len() {
            offset += last_batch.len();
            last_batch = self
                .get_electoral_log_messages_from_db(
                    board_db,
                    0,
                    Some(IMMUDB_DEFAULT_LIMIT),
                    Some(offset),
                )
                .await?;
            messages.extend(last_batch.clone());
        }
        Ok(messages)
    }

    async fn get_electoral_log_messages_from_db(
        &mut self,
        board_db: &str,
        last_id: i64,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<ElectoralLogMessage>> {
        self.client.use_database(board_db).await?;
        let sql = format!(
            r#"
        SELECT
            id,
            created,
            sender_pk,
            statement_timestamp,
            statement_kind,
            message,
            version,
            user_id
        FROM {}
        WHERE id > @last_id
        ORDER BY id
        LIMIT {}
        OFFSET {};
        "#,
            Table::ElectoralLogMessages.as_str(),
            limit.unwrap_or(IMMUDB_DEFAULT_LIMIT),
            offset.unwrap_or(IMMUDB_DEFAULT_OFFSET),
        );

        let params = vec![NamedParam {
            name: String::from("last_id"),
            value: Some(SqlValue {
                value: Some(Value::N(last_id)),
            }),
        }];

        let sql_query_response = self.client.sql_query(&sql, params).await?;
        let messages = sql_query_response
            .get_ref()
            .rows
            .iter()
            .map(ElectoralLogMessage::try_from)
            .collect::<Result<Vec<ElectoralLogMessage>>>()?;

        Ok(messages)
    }

    pub async fn insert_electoral_log_messages(
        &mut self,
        board_db: &str,
        messages: &Vec<ElectoralLogMessage>,
    ) -> Result<()> {
        info!("Insert {} messages..", messages.len());
        self.client.open_session(board_db).await?;
        // Start a new transaction
        let transaction_id = self.client.new_tx(TxMode::ReadWrite).await;
        if transaction_id.is_err() {
            self.client.close_session().await?;
        }
        let transaction_id = transaction_id?;

        let mut sql_results = vec![];
        for message in messages {
            let message_sql = format!(
                r#"
                INSERT INTO {} (
                    created,
                    sender_pk,
                    statement_kind,
                    statement_timestamp,
                    message,
                    version,
                    user_id
                ) VALUES (
                    @created,
                    @sender_pk,
                    @statement_kind,
                    @statement_timestamp,
                    @message,
                    @version,
                    @user_id
                );
            "#,
                Table::ElectoralLogMessages.as_str()
            );
            let params = vec![
                NamedParam {
                    name: String::from("created"),
                    value: Some(SqlValue {
                        value: Some(Value::Ts(message.created)),
                    }),
                },
                NamedParam {
                    name: String::from("sender_pk"),
                    value: Some(SqlValue {
                        value: Some(Value::S(message.sender_pk.clone())),
                    }),
                },
                NamedParam {
                    name: String::from("statement_timestamp"),
                    value: Some(SqlValue {
                        value: Some(Value::Ts(message.statement_timestamp)),
                    }),
                },
                NamedParam {
                    name: String::from("statement_kind"),
                    value: Some(SqlValue {
                        value: Some(Value::S(message.statement_kind.clone())),
                    }),
                },
                NamedParam {
                    name: String::from("message"),
                    value: Some(SqlValue {
                        value: Some(Value::Bs(message.message.clone())),
                    }),
                },
                NamedParam {
                    name: String::from("version"),
                    value: Some(SqlValue {
                        value: Some(Value::S(message.version.clone())),
                    }),
                },
                NamedParam {
                    name: String::from("user_id"),
                    value: Some(SqlValue {
                        value: match message.user_id.clone() {
                            Some(user_id) => Some(Value::S(user_id)),
                            None => None,
                        },
                    }),
                },
            ];
            let result = self
                .client
                .tx_sql_exec(&message_sql, &transaction_id, params)
                .await;
            sql_results.push(result);
        }

        let commit = self
            .client
            .commit(&transaction_id)
            .await
            .with_context(|| "error commiting to electoral log");
        self.client.close_session().await?;

        // We defer checking on these results until after closing the session
        for result in sql_results {
            result?;
        }
        commit?;

        Ok(())
    }

    #[instrument(skip(self), level = "trace")]
    pub async fn has_database(&mut self, database_name: &str) -> Result<bool> {
        self.client.has_database(database_name).await
    }

    /// Creates the requested board immudb database if it doesnt exist.
    /// Also creates the and the electoral log and braid tables.
    #[instrument(skip(self))]
    pub async fn upsert_electoral_log_db(&mut self, board_dbname: &str) -> Result<()> {
        let sql = format!(
            r#"
        CREATE TABLE IF NOT EXISTS {} (
            id INTEGER AUTO_INCREMENT,
            created TIMESTAMP,
            sender_pk VARCHAR,
            statement_timestamp TIMESTAMP,
            statement_kind VARCHAR,
            message BLOB,
            version VARCHAR,
            PRIMARY KEY id
        );
        "#,
            // Table::BraidMessages.as_str(),
            Table::ElectoralLogMessages.as_str()
        );
        self.upsert_database(board_dbname, &sql).await
    }

    /// Deletes the immudb database.
    #[instrument(skip(self))]
    pub async fn delete_database(&mut self, database_name: &str) -> Result<()> {
        if self.client.has_database(database_name).await? {
            self.client.delete_database(database_name).await?;
        }
        Ok(())
    }

    /// Creates the requested immudb database, only if it doesn't exist. It also creates
    /// the requested tables if they don't exist.
    #[instrument(skip(self))]
    async fn upsert_database(&mut self, database_name: &str, tables: &str) -> Result<()> {
        // create database if it doesn't exist
        if !self.client.has_database(database_name).await? {
            self.client.create_database(database_name).await?;
            info!("Database created!");
        };
        self.client.use_database(database_name).await?;

        // List tables and create them if missing
        if !self.client.has_tables().await? {
            info!("no tables! let's create them");
            self.client.sql_exec(&tables, vec![]).await?;
        }
        Ok(())
    }
}
