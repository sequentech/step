// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::assign_value;
use anyhow::{anyhow, Context, Result};
use immudb_rs::{sql_value::Value, Client, CommittedSqlTx, NamedParam, Row, SqlValue, TxMode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::Display;
use strand::hash::info;
use strum_macros::Display;
use tracing::{info, instrument};

const IMMUDB_DEFAULT_LIMIT: usize = 900;
const IMMUDB_DEFAULT_ENTRIES_TX_LIMIT: usize = 50;
const IMMUDB_DEFAULT_OFFSET: usize = 0;
const ELECTORAL_LOG_TABLE: &'static str = "electoral_log_messages";

#[derive(Debug)]
pub struct BoardClient {
    client: Client,
}

#[derive(Debug, Clone, Display, PartialEq, Eq, Ord, PartialOrd)]
#[strum(serialize_all = "snake_case")]
pub enum ElectoralLogVarCharColumn {
    StatementKind,
    UserId,
    BallotId,
    Username,
    SenderPk,
    ElectionId,
    AreaId,
    Version,
}

#[derive(Display, Debug, Clone)]
pub enum SqlCompOperators {
    #[strum(to_string = "=")]
    Equal,
    #[strum(to_string = "!=")]
    NotEqual,
    #[strum(to_string = ">")]
    GreaterThan,
    #[strum(to_string = "<")]
    LessThan,
    #[strum(to_string = ">=")]
    GreaterThanOrEqual,
    #[strum(to_string = "<=")]
    LessThanOrEqual,
    #[strum(to_string = "LIKE")]
    Like,
    #[strum(to_string = "ILIKE")]
    ILike,
    #[strum(to_string = "IN")]
    In,
    #[strum(to_string = "NOT IN")]
    NotIn,
}

pub type WhereClauseBTreeMap = BTreeMap<ElectoralLogVarCharColumn, (SqlCompOperators, String)>;
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
    pub ballot_id: Option<String>,
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
        let mut ballot_id: Option<String> = None;

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
                "ballod_id" => match value.value.as_ref() {
                    Some(Value::S(inner)) => ballot_id = Some(inner.clone()),
                    Some(Value::Null(_)) => ballot_id = None,
                    None => ballot_id = None,
                    _ => {
                        return Err(anyhow!(
                            "invalid column value for 'ballod_id': {:?}",
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
            ballot_id,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Aggregate {
    pub count: i64,
}

impl TryFrom<&Row> for Aggregate {
    type Error = anyhow::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        let mut count = 0;

        for (column, value) in row.columns.iter().zip(row.values.iter()) {
            match column.as_str() {
                _ => assign_value!(Value::N, value, count),
            }
        }
        Ok(Aggregate { count })
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
            area_id,
            ballot_id,
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

    /// columns_matcher represents the columns that will be used to filter the messages,
    /// The order as defined ElectoralLogVarCharColumn is important for preformance to match the indexes.
    /// BTreeMap ensures the order is preserved no matter the insertion sequence.
    pub async fn get_electoral_log_messages_filtered<K: Display, V: Display>(
        &mut self,
        board_db: &str,
        columns_matcher: Option<WhereClauseBTreeMap>,
        min_ts: Option<i64>,
        max_ts: Option<i64>,
        limit: Option<i64>,
        offset: Option<i64>,
        order_by: Option<HashMap<K, V>>,
    ) -> Result<Vec<ElectoralLogMessage>> {
        self.get_filtered(
            board_db,
            columns_matcher,
            min_ts,
            max_ts,
            limit,
            offset,
            order_by,
        )
        .await
    }

    #[instrument(skip_all, err)]
    async fn get_filtered<K: Display, V: Display>(
        &mut self,
        board_db: &str,
        columns_matcher: Option<WhereClauseBTreeMap>,
        min_ts: Option<i64>,
        max_ts: Option<i64>,
        limit: Option<i64>,
        offset: Option<i64>,
        order_by: Option<HashMap<K, V>>,
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

        let mut params = vec![];
        let mut where_clause = String::from("statement_kind IS NOT NULL ");
        if let Some(columns_matcher) = &columns_matcher {
            for (key, (op, value)) in columns_matcher {
                where_clause.push_str(&format!("AND {key} {op} @{key} "));
                params.push(NamedParam {
                    name: key.to_string(),
                    value: Some(SqlValue {
                        value: Some(Value::S(value.to_owned())),
                    }),
                })
            }
        }

        let order_by_clauses = if let Some(order_by) = order_by {
            order_by
                .iter()
                .map(|(field, direction)| format!("ORDER BY {field} {direction}"))
                .collect::<Vec<String>>()
                .join(", ")
        } else {
            format!("ORDER BY id desc")
        };

        self.client.use_database(board_db).await?;
        let sql = format!(
            r#"
        SELECT
            id,
            username,
            user_id,
            area_id,
            election_id,
            ballot_id,
            created,
            sender_pk,
            statement_timestamp,
            statement_kind,
            message,
            version
        FROM {ELECTORAL_LOG_TABLE}
        WHERE {where_clause}
        {min_clause}
        {max_clause}
        {order_by_clauses}
        LIMIT @limit
        OFFSET @offset;
        "#
        );

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

        params.push(NamedParam {
            name: String::from("limit"),
            value: Some(SqlValue {
                value: Some(Value::N(limit.unwrap_or(IMMUDB_DEFAULT_LIMIT as i64))),
            }),
        });

        params.push(NamedParam {
            name: String::from("offset"),
            value: Some(SqlValue {
                value: Some(Value::N(offset.unwrap_or(IMMUDB_DEFAULT_OFFSET as i64))),
            }),
        });

        info!("SQL query: {}", sql);
        let sql_query_response = self.client.sql_query(&sql, params).await?;
        let messages = sql_query_response
            .get_ref()
            .rows
            .iter()
            .map(ElectoralLogMessage::try_from)
            .collect::<Result<Vec<ElectoralLogMessage>>>()?;

        Ok(messages)
    }

    #[instrument(err)]
    pub async fn count_electoral_log_messages(
        &mut self,
        board_db: &str,
        columns_matcher: Option<WhereClauseBTreeMap>,
    ) -> Result<i64> {
        let mut params = vec![];
        let mut where_clause = String::from("statement_kind IS NOT NULL ");
        if let Some(columns_matcher) = &columns_matcher {
            for (key, (op, value)) in columns_matcher {
                where_clause.push_str(&format!("AND {key} {op} @{key} "));
                params.push(NamedParam {
                    name: key.to_string(),
                    value: Some(SqlValue {
                        value: Some(Value::S(value.to_owned())),
                    }),
                })
            }
        }

        self.client.use_database(board_db).await?;
        let sql = format!(
            r#"
            SELECT COUNT(*)
            FROM {ELECTORAL_LOG_TABLE}
            WHERE {where_clause}
            "#,
        );

        info!("SQL query: {}", sql);
        let sql_query_response = self.client.sql_query(&sql, params).await?;
        let mut rows_iter = sql_query_response
            .get_ref()
            .rows
            .iter()
            .map(Aggregate::try_from);
        let aggregate = rows_iter
            .next()
            .ok_or_else(|| anyhow!("No aggregate found"))??;

        Ok(aggregate.count as i64)
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
                    ballot_id
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
                    @ballot_id
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
                NamedParam {
                    name: String::from("ballot_id"),
                    value: Some(SqlValue {
                        value: match message.ballot_id.clone() {
                            Some(ballot_id) => Some(Value::S(ballot_id)),
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
                    ballot_id
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
                    @ballot_id
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
                NamedParam {
                    name: String::from("ballot_id"),
                    value: Some(SqlValue {
                        value: match message.ballot_id.clone() {
                            Some(ballot_id) => Some(Value::S(ballot_id)),
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
         CREATE TABLE IF NOT EXISTS {ELECTORAL_LOG_TABLE} (
            id INTEGER AUTO_INCREMENT,
            created TIMESTAMP,
            sender_pk VARCHAR,
            statement_timestamp TIMESTAMP,
            statement_kind VARCHAR[64],
            message BLOB,
            version VARCHAR,
            user_id VARCHAR[64],
            username VARCHAR,
            election_id VARCHAR[64],
            area_id VARCHAR[64],
            ballod_id VARCHAR[64],
            PRIMARY KEY id
        );
        "#
        );

        // This is the order of the cols in the where clauses, as defined in ElectoralLogVarCharColumn
        // Note Username cannot be indexed because it is not constrained to 512B, but is not needded since we have user_id
        // StatementKind, UserId, BallotId, Username, SenderPk, ElectionId, AreaId, Version,
        let elog_indexes = vec![
            format!("CREATE INDEX IF NOT EXISTS ON {ELECTORAL_LOG_TABLE} (statement_kind, user_id, ballot_id, election_id, id)"), // To list or count cast_vote_messages and Order by id
            format!("CREATE INDEX IF NOT EXISTS ON {ELECTORAL_LOG_TABLE} (statement_kind, user_id, ballot_id, election_id, statement_timestamp)"), // Order by statement_timestamp
            format!("CREATE INDEX IF NOT EXISTS ON {ELECTORAL_LOG_TABLE} (statement_kind, election_id, id)"), // Order by id
            format!("CREATE INDEX IF NOT EXISTS ON {ELECTORAL_LOG_TABLE} (statement_kind, election_id, statement_timestamp)"), // Order by statement_timestamp
            format!("CREATE INDEX IF NOT EXISTS ON {ELECTORAL_LOG_TABLE} (user_id, election_id, area_id, id)"), // Other posible filters...
            format!("CREATE INDEX IF NOT EXISTS ON {ELECTORAL_LOG_TABLE} (election_id, area_id, id)"),
            format!("CREATE INDEX IF NOT EXISTS ON {ELECTORAL_LOG_TABLE} (area_id, id)"),
        ];

        self.upsert_database(board_dbname, &sql, elog_indexes.as_slice())
            .await
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
    /// the requested tables and indexes if they don't exist.
    async fn upsert_database(
        &mut self,
        database_name: &str,
        tables: &str,
        indexes: &[String],
    ) -> Result<()> {
        // create database if it doesn't exist
        if !self.client.has_database(database_name).await? {
            println!("Database not found, creating..");
            self.client.create_database(database_name).await?;
            info!("Database created!");
        };
        self.client.use_database(database_name).await?;

        // List tables and create them if missing
        if !self.client.has_tables().await? {
            info!("no tables! let's create them");
            self.client.sql_exec(&tables, vec![]).await?;
        }
        for index in indexes {
            info!("Inserting index...");
            self.client.sql_exec(index, vec![]).await?;
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
            ballot_id: None,
        };
        let messages = vec![electoral_log_message];

        b.insert_electoral_log_messages(BOARD_DB, &messages)
            .await
            .unwrap();

        let ret = b.get_electoral_log_messages(BOARD_DB).await.unwrap();
        assert_eq!(messages, ret);

        let cols_match = BTreeMap::from([
            (
                ElectoralLogVarCharColumn::StatementKind,
                (SqlCompOperators::Equal, "".to_string()),
            ),
            (
                ElectoralLogVarCharColumn::SenderPk,
                (SqlCompOperators::Equal, "".to_string()),
            ),
        ]);
        let ret = b
            .get_electoral_log_messages_filtered(
                BOARD_DB,
                Some(cols_match),
                None,
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(messages, ret);
        let ret = b
            .get_electoral_log_messages_filtered(
                BOARD_DB,
                Some(cols_match),
                Some(1i64),
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(messages, ret);
        let ret = b
            .get_electoral_log_messages_filtered(
                BOARD_DB,
                Some(cols_match),
                None,
                Some(556i64),
                None,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(messages, ret);
        let ret = b
            .get_electoral_log_messages_filtered(
                BOARD_DB,
                Some(cols_match),
                Some(1i64),
                Some(556i64),
                None,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(messages, ret);
        let ret = b
            .get_electoral_log_messages_filtered(
                BOARD_DB,
                Some(cols_match),
                Some(556i64),
                Some(666i64),
                None,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(ret.len(), 0);

        tear_down(b).await;
    }
}
