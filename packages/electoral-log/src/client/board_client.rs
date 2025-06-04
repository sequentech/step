// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use crate::assign_value;
use immudb_rs::{sql_value::Value, Client, CommittedSqlTx, NamedParam, Row, SqlValue, TxMode};
use std::fmt::Debug;
use tokio::time::{sleep, Duration};
use tokio_stream::StreamExt; // Added for streaming
use tracing::{debug, error, info, warn};

const IMMUDB_DEFAULT_LIMIT: usize = 900;
const IMMUDB_DEFAULT_ENTRIES_TX_LIMIT: usize = 50;
const IMMUDB_DEFAULT_OFFSET: usize = 0;
const ELECTORAL_LOG_TABLE: &'static str = "electoral_log_messages";

#[derive(Debug)]
pub struct BoardClient {
    client: Client,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    pub election_id: Option<String>,
    pub area_id: Option<String>,
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
        let mut election_id: Option<String> = None;
        let mut area_id: Option<String> = None;

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
                    Some(Value::Null(_)) => user_id = None,
                    None => user_id = None,
                    _ => {
                        return Err(anyhow!(
                            "invalid column value for 'user_id': {:?}",
                            value.value.as_ref()
                        ))
                    }
                },
                "username" => match value.value.as_ref() {
                    Some(Value::S(inner)) => username = Some(inner.clone()),
                    Some(Value::Null(_)) => username = None,
                    None => username = None,
                    _ => {
                        return Err(anyhow!(
                            "invalid column value for 'username': {:?}",
                            value.value.as_ref()
                        ))
                    }
                },
                "election_id" => match value.value.as_ref() {
                    Some(Value::S(inner)) => election_id = Some(inner.clone()),
                    Some(Value::Null(_)) => election_id = None,
                    None => election_id = None,
                    _ => {
                        return Err(anyhow!(
                            "invalid column value for 'election_id': {:?}",
                            value.value.as_ref()
                        ))
                    }
                },
                "area_id" => match value.value.as_ref() {
                    Some(Value::S(inner)) => area_id = Some(inner.clone()),
                    Some(Value::Null(_)) => area_id = None,
                    None => area_id = None,
                    _ => {
                        return Err(anyhow!(
                            "invalid column value for 'area_id': {:?}",
                            value.value.as_ref()
                        ))
                    }
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
            election_id,
            area_id,
        })
    }
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
            user_id,
            username
        FROM {}
        WHERE id > @last_id
        ORDER BY id
        LIMIT {}
        OFFSET {};
        "#,
            ELECTORAL_LOG_TABLE,
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

    pub async fn get_electoral_log_messages_filtered(
        &mut self,
        board_db: &str,
        kind: &str,
        sender_pk: Option<&str>,
        min_ts: Option<i64>,
        max_ts: Option<i64>,
    ) -> Result<Vec<ElectoralLogMessage>> {
        self.get_filtered(board_db, kind, sender_pk, min_ts, max_ts)
            .await
    }

    async fn get_filtered(
        &mut self,
        board_db: &str,
        kind: &str,
        sender_pk: Option<&str>,
        min_ts: Option<i64>,
        max_ts: Option<i64>,
    ) -> Result<Vec<ElectoralLogMessage>> {
        let (min_clause, min_clause_value) = if let Some(min_ts) = min_ts {
            ("AND created >= @min_ts", min_ts)
        } else {
            ("", 0)
        };

        let (max_clause, max_clause_value) = if let Some(max_ts) = max_ts {
            ("AND created <= @max_ts", max_ts)
        } else {
            ("", 0)
        };

        let (sender_pk_clause, sender_pk_value) = if let Some(sender_pk) = sender_pk {
            ("AND sender_pk = @sender_pk", sender_pk)
        } else {
            ("", "")
        };

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
            version
        FROM {}
        WHERE statement_kind = @statement_kind
        {}
        {}
        {}
        ORDER BY id;
        "#,
            ELECTORAL_LOG_TABLE, min_clause, max_clause, sender_pk_clause
        );

        let mut params = vec![NamedParam {
            name: String::from("statement_kind"),
            value: Some(SqlValue {
                value: Some(Value::S(kind.to_string())),
            }),
        }];
        if min_clause_value != 0 {
            params.push(NamedParam {
                name: String::from("min_ts"),
                value: Some(SqlValue {
                    value: Some(Value::Ts(min_clause_value)),
                }),
            })
        }
        if max_clause_value != 0 {
            params.push(NamedParam {
                name: String::from("max_ts"),
                value: Some(SqlValue {
                    value: Some(Value::Ts(max_clause_value)),
                }),
            })
        }
        if !sender_pk_value.is_empty() {
            params.push(NamedParam {
                name: String::from("sender_pk"),
                value: Some(SqlValue {
                    value: Some(Value::S(sender_pk_value.to_string())),
                }),
            })
        }

        let response_stream = self.client.streaming_sql_query(&sql, params)
            .await
            .with_context(|| "Failed to execute streaming_sql_query using immudb-rs v0.1.0. This version streams batches (SqlQueryResult).")?;
        let mut stream = response_stream.into_inner(); // Get the tonic::Streaming<SqlQueryResult>

        let mut messages: Vec<ElectoralLogMessage> = vec![];
        let mut total_rows_fetched = 0;

        while let Some(batch_result) = stream.next().await {
            // Iterates over SqlQueryResult batches
            match batch_result {
                Ok(sql_query_result_batch) => {
                    for individual_row in &sql_query_result_batch.rows {
                        total_rows_fetched += 1;
                        if total_rows_fetched % 1000 == 0 {
                            info!(total_rows_fetched, "Processed rows from stream...");
                        }

                        let elog_row = match ElectoralLogMessage::try_from(individual_row) {
                            Ok(elog_row) => elog_row,
                            Err(e) => {
                                warn!(error = %e, "Failed to parse ImmudbRow into ElectoralLogRow from stream batch.");
                                continue;
                            }
                        };
                        messages.push(elog_row);
                    }
                }
                Err(e) => {
                    error!(error = %e, "Error receiving batch from Immudb stream.");
                    // Depending on the error, you might want to break or continue.
                    // For now, we'll log and break for stream errors to avoid infinite loops on persistent errors.
                    break;
                }
            }
        }

        Ok(messages)
    }

    pub async fn open_session(&mut self, database_name: &str) -> Result<()> {
        self.client.open_session(database_name).await
    }

    pub async fn close_session(&mut self) -> Result<()> {
        self.client.close_session().await
    }

    pub async fn new_tx(&mut self, mode: TxMode) -> Result<String> {
        self.client.new_tx(mode).await
    }

    pub async fn commit(&mut self, transaction_id: &String) -> Result<CommittedSqlTx> {
        self.client.commit(transaction_id).await
    }

    // Insert messages in batch using an existing session/transaction
    pub async fn insert_electoral_log_messages_batch(
        &mut self,
        transaction_id: &String,
        messages: &[ElectoralLogMessage],
    ) -> Result<()> {
        info!("Insert {} messages in batch..", messages.len());
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
                    user_id,
                    username,
                    election_id,
                    area_id
                ) VALUES (
                    @created,
                    @sender_pk,
                    @statement_kind,
                    @statement_timestamp,
                    @message,
                    @version,
                    @user_id,
                    @username,
                    @election_id,
                    @area_id
                );
            "#,
                ELECTORAL_LOG_TABLE
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
                NamedParam {
                    name: String::from("username"),
                    value: Some(SqlValue {
                        value: match message.username.clone() {
                            Some(username) => Some(Value::S(username)),
                            None => None,
                        },
                    }),
                },
                NamedParam {
                    name: String::from("election_id"),
                    value: Some(SqlValue {
                        value: match message.election_id.clone() {
                            Some(election_id) => Some(Value::S(election_id)),
                            None => None,
                        },
                    }),
                },
                NamedParam {
                    name: String::from("area_id"),
                    value: Some(SqlValue {
                        value: match message.area_id.clone() {
                            Some(area_id) => Some(Value::S(area_id)),
                            None => None,
                        },
                    }),
                },
            ];
            let result = self
                .client
                .tx_sql_exec(&message_sql, transaction_id, params)
                .await?;
            sql_results.push(result);
        }
        Ok(())
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
                    user_id,
                    username,
                    election_id,
                    area_id
                ) VALUES (
                    @created,
                    @sender_pk,
                    @statement_kind,
                    @statement_timestamp,
                    @message,
                    @version,
                    @user_id,
                    @username,
                    @election_id,
                    @area_id
                );
            "#,
                ELECTORAL_LOG_TABLE
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
                NamedParam {
                    name: String::from("username"),
                    value: Some(SqlValue {
                        value: match message.username.clone() {
                            Some(username) => Some(Value::S(username)),
                            None => None,
                        },
                    }),
                },
                NamedParam {
                    name: String::from("election_id"),
                    value: Some(SqlValue {
                        value: match message.election_id.clone() {
                            Some(election_id) => Some(Value::S(election_id)),
                            None => None,
                        },
                    }),
                },
                NamedParam {
                    name: String::from("area_id"),
                    value: Some(SqlValue {
                        value: match message.area_id.clone() {
                            Some(area_id) => Some(Value::S(area_id)),
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

    /// Creates the requested immudb database if it doesnt exist.
    /// Also creates the electoral log table.
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
            user_id VARCHAR,
            username VARCHAR,
            election_id VARCHAR,
            area_id VARCHAR,
            PRIMARY KEY id
        );
        "#,
            ELECTORAL_LOG_TABLE
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
    async fn upsert_database(&mut self, database_name: &str, tables: &str) -> Result<()> {
        // create database if it doesn't exist
        if !self.client.has_database(database_name).await? {
            println!("Database not found, creating..");
            self.client.create_database(database_name).await?;
            event!(Level::INFO, "Database created!");
        };
        self.client.use_database(database_name).await?;

        // List tables and create them if missing
        if !self.client.has_tables().await? {
            event!(Level::INFO, "no tables! let's create them");
            self.client.sql_exec(&tables, vec![]).await?;
        }
        Ok(())
    }
}

// Run ignored tests with
// cargo test <test_name> -- --include-ignored
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serial_test::serial;

    const BOARD_DB: &'static str = "testdb";

    async fn set_up() -> BoardClient {
        let mut b = BoardClient::new("http://localhost:3322", "immudb", "immudb")
            .await
            .unwrap();

        // In case the previous test did not clean up properly
        b.delete_database(BOARD_DB).await.unwrap();
        b.upsert_electoral_log_db(BOARD_DB).await.unwrap();

        b
    }

    async fn tear_down(mut b: BoardClient) {
        b.delete_database(BOARD_DB).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    #[serial]
    pub async fn test_message_create_retrieve() {
        let mut b = set_up().await;
        let electoral_log_message = ElectoralLogMessage {
            id: 1,
            created: 555,
            sender_pk: "".to_string(),
            statement_timestamp: 0,
            statement_kind: "".to_string(),
            message: vec![],
            version: "".to_string(),
            user_id: None,
            username: None,
            election_id: None,
            area_id: None,
        };
        let messages = vec![electoral_log_message];

        b.insert_electoral_log_messages(BOARD_DB, &messages)
            .await
            .unwrap();

        let ret = b.get_electoral_log_messages(BOARD_DB).await.unwrap();
        assert_eq!(messages, ret);
        let ret = b
            .get_electoral_log_messages_filtered(BOARD_DB, "", Some(""), None, None)
            .await
            .unwrap();
        assert_eq!(messages, ret);
        let ret = b
            .get_electoral_log_messages_filtered(BOARD_DB, "", Some(""), Some(1i64), None)
            .await
            .unwrap();
        assert_eq!(messages, ret);
        let ret = b
            .get_electoral_log_messages_filtered(BOARD_DB, "", Some(""), None, Some(556i64))
            .await
            .unwrap();
        assert_eq!(messages, ret);
        let ret = b
            .get_electoral_log_messages_filtered(BOARD_DB, "", Some(""), Some(1i64), Some(556i64))
            .await
            .unwrap();
        assert_eq!(messages, ret);
        let ret = b
            .get_electoral_log_messages_filtered(BOARD_DB, "", Some(""), Some(556i64), Some(666i64))
            .await
            .unwrap();
        assert_eq!(ret.len(), 0);

        tear_down(b).await;
    }
}
