use anyhow::{anyhow, Result};
use log::info;
use tracing::{event, Level, instrument};

use immudb_rs::{sql_value::Value, Client, NamedParam, Row, SqlValue, TxMode};
use std::fmt::Debug;

#[derive(Debug)]
pub struct BoardClient {
    client: Client,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardMessage {
    pub id: i64,
    pub created: i64,
    // Base64 encoded spki der representation.
    pub signer_key: String,
    pub statement_timestamp: i64,
    pub statement_kind: String,
    pub message: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Board {
    pub id: i64,
    pub database_name: String,
    pub is_archived: bool,
}

macro_rules! assign_value {
    ($enum_variant:path, $value:expr, $target:ident) => {
        match $value.value.as_ref() {
            Some($enum_variant(inner)) => {
                $target = inner.clone();
            }
            _ => {
                return Err(
                    anyhow!(
                        r#"invalid column value for `$enum_variant`, `$value`, 
                        `$target`"#
                    )
                );
            }
        }
    };
}

impl TryFrom<&Row> for BoardMessage {
    type Error = anyhow::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        let mut id = 0;
        let mut created = 0;
        let mut signer_key = String::from("");
        let mut statement_timestamp = 0;
        let mut statement_kind = String::from("");
        let mut message = vec![];
        for (column, value) in row.columns.iter().zip(row.values.iter()) {
            // FIXME for some reason columns names appear with parentheses
            match column.as_str() {
                "(messages.id)" => assign_value!(Value::N, value, id),
                "(messages.created)" => assign_value!(Value::Ts, value, created),
                "(messages.signer_key)" => assign_value!(Value::S, value, signer_key),
                "(messages.statement_timestamp)" => {
                    assign_value!(Value::Ts, value, statement_timestamp)
                }
                "(messages.statement_kind)" => assign_value!(Value::S, value, statement_kind),
                "(messages.message)" => assign_value!(Value::Bs, value, message),
                _ => return Err(anyhow!("invalid column found '{}'", column.as_str())),
            }
        }
        Ok(BoardMessage {
            id,
            created,
            signer_key,
            statement_timestamp,
            statement_kind,
            message,
        })
    }
}

impl TryFrom<&Row> for Board {
    type Error = anyhow::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        let mut id = 0;
        let mut database_name: String = "".to_string();
        let mut is_archived = false;
        for (column, value) in row.columns.iter().zip(row.values.iter()) {
            // FIXME for some reason columns names appear with parentheses
            match column.as_str() {
                "(bulletin_boards.id)" => assign_value!(Value::N, value, id),
                "(bulletin_boards.database_name)" => assign_value!(Value::S, value, database_name),
                "(bulletin_boards.is_archived)" => assign_value!(Value::B, value, is_archived),
                _ => return Err(anyhow!("invalid column found '{}'", column.as_str())),
            }
        }
        Ok(Board {
            id,
            database_name,
            is_archived,
        })
    }
}

impl BoardClient {
    pub async fn new(
        server_url: &str,
        username: &str,
        password: &str,
    ) -> Result<BoardClient> {
        let client = Client::new(&server_url, username, password).await?;
        Ok(BoardClient { client: client })
    }

    pub async fn login(&mut self) -> Result<()> {
        self.client.login().await
    }

    /// Get all messages whose id is bigger than `last_id`
    pub async fn get_messages(
        &mut self,
        board_db: &str,
        last_id: i64,
    ) -> Result<Vec<BoardMessage>> {
        self.client.open_session(board_db).await?;
        let sql = format!(
            r#"
        SELECT
            id,
            created,
            signer_key,
            statement_timestamp,
            statement_kind,
            message
        FROM messages
        WHERE id > {}
        "#,
            last_id
        );
        let sql_query_response = self.client.sql_query(&sql, vec![]).await?;
        let messages = sql_query_response
            .get_ref()
            .rows
            .iter()
            .map(BoardMessage::try_from)
            .collect::<Result<Vec<BoardMessage>>>()?;
        self.client.close_session().await?;
        Ok(messages)
    }


    /// Get all messages whose id is bigger than `last_id`
    pub async fn get_messages_from_kind(
        &mut self,
        board_db: &str,
        kind: &str,
        signer_key: &str
    ) -> Result<Vec<BoardMessage>> {
        self.client.open_session(board_db).await?;
        let sql = format!(
            r#"
        SELECT
            id,
            created,
            signer_key,
            statement_timestamp,
            statement_kind,
            message
        FROM messages
        WHERE signer_key = {} and statement_kind = {}
        "#,
            signer_key,
            kind,
        );
        let sql_query_response = self.client.sql_query(&sql, vec![]).await?;
        let messages = sql_query_response
            .get_ref()
            .rows
            .iter()
            .map(BoardMessage::try_from)
            .collect::<Result<Vec<BoardMessage>>>()?;
        self.client.close_session().await?;
        Ok(messages)
    }

    pub async fn insert_messages(
        &mut self,
        board_db: &str,
        messages: &Vec<BoardMessage>,
    ) -> Result<()> {
        info!("Insert {} messages..", messages.len());
        self.client.open_session(board_db).await?;
        // Start a new transaction
        let transaction_id = self.client.new_tx(TxMode::ReadWrite).await?;
        for message in messages {
            let message_sql = r#"
                INSERT INTO messages(
                    created,
                    signer_key,
                    statement_kind,
                    statement_timestamp,
                    message
                ) VALUES (
                    @created,
                    @signer_key,
                    @statement_kind,
                    @statement_timestamp,
                    @message
                );
            "#;
            let params = vec![
                NamedParam {
                    name: String::from("created"),
                    value: Some(SqlValue {
                        value: Some(Value::Ts(message.created)),
                    }),
                },
                NamedParam {
                    name: String::from("signer_key"),
                    value: Some(SqlValue {
                        value: Some(Value::S(message.signer_key.clone())),
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
            ];
            self.client
                .tx_sql_exec(&message_sql, &transaction_id, params)
                .await?;
        }
        self.client.commit(&transaction_id).await?;
        self.client.close_session().await?;
        Ok(())
    }

    pub async fn get_boards(&mut self, index_db: &str) -> Result<Vec<Board>> {
        self.client.open_session(index_db).await?;
        let sql = format!(
            r#"
        SELECT
            id,
            database_name,
            is_archived
        FROM bulletin_boards
        WHERE is_archived = {}
        "#,
            false
        );
        let sql_query_response = self.client.sql_query(&sql, vec![]).await?;
        let boards = sql_query_response
            .get_ref()
            .rows
            .iter()
            .map(Board::try_from)
            .collect::<Result<Vec<Board>>>()?;
        self.client.close_session().await?;
        Ok(boards)
    }

    pub async fn get_board(&mut self, index_db: &str, board_db: &str) -> Result<Board> {
        self.client.use_database(index_db).await?;
        let message_sql = r#"
        SELECT
            id,
            database_name,
            is_archived
        FROM bulletin_boards
        WHERE database_name = @database_name;
        "#;
        let params = vec![NamedParam {
            name: String::from("database_name"),
            value: Some(SqlValue {
                value: Some(Value::S(board_db.to_string())),
            }),
        }];
        let sql_query_response = self.client.sql_query(&message_sql, params).await?;
        let boards = sql_query_response
            .get_ref()
            .rows
            .iter()
            .map(Board::try_from)
            .collect::<Result<Vec<Board>>>()?;
        
        if boards.len() > 0 {
            Ok(boards[0].clone())
        }
        else {
            Err(anyhow!("board name '{}' not found", board_db))
        }
        
        
    }

    
    #[instrument(skip(self))]
    pub async fn has_database(&mut self, database_name: &str) -> Result<bool> {
        self.client.has_database(database_name).await
    }

    #[instrument(skip(self))]
    pub async fn create_board(&mut self, index_db: &str, board_db: &str) -> Result<Board> {
        self.client.create_database(board_db).await?;
        event!(Level::INFO, "Database created!");
        self.client.use_database(board_db).await?;
        let tables = r#"
            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER AUTO_INCREMENT,
                created TIMESTAMP,
                signer_key VARCHAR,
                statement_timestamp TIMESTAMP,
                statement_kind VARCHAR,
                message BLOB,
                PRIMARY KEY id
            );
            "#;
        self.client.sql_exec(&tables, vec![]).await?;
        event!(Level::INFO, "Database tables created!");
        self.client.use_database(index_db).await?;

        let message_sql = r#"
            INSERT INTO bulletin_boards(
                database_name,
                is_archived
            ) VALUES (
                @database_name,
                @is_archived
            );
        "#;
        let params = vec![
            NamedParam {
                name: String::from("database_name"),
                value: Some(SqlValue {
                    value: Some(Value::S(board_db.to_string())),
                }),
            },
            NamedParam {
                name: String::from("is_archived"),
                value: Some(SqlValue {
                    value: Some(Value::B(false)),
                }),
            },
        ];
        let _ = self.client.sql_exec(&message_sql, params).await?;

        self.get_board(index_db, board_db).await
    }

    #[instrument(skip(self))]
    pub async fn delete_board(&mut self, index_db: &str, board_db: &str) -> Result<()> {
        self.delete_database(board_db).await?;
        event!(Level::INFO, "Database deleted!");
        self.client.use_database(index_db).await?;

        let message_sql = r#"
            DELETE from bulletin_boards where 
            database_name = @database_name 
            AND
            is_archived = @is_archived;
        "#;
        let params = vec![
            NamedParam {
                name: String::from("database_name"),
                value: Some(SqlValue {
                    value: Some(Value::S(board_db.to_string())),
                }),
            },
            NamedParam {
                name: String::from("is_archived"),
                value: Some(SqlValue {
                    value: Some(Value::B(false)),
                }),
            },
        ];
        let _ = self.client.sql_exec(&message_sql, params).await?;

        Ok(())
    }

    pub async fn upsert_index_db(&mut self, index_dbname: &str) -> Result<()> {
        self.upsert_database(
            index_dbname,
            r#"
            CREATE TABLE IF NOT EXISTS bulletin_boards (
                id INTEGER AUTO_INCREMENT,
                database_name VARCHAR[128],
                is_archived BOOLEAN,
                PRIMARY KEY id
            );
            CREATE UNIQUE INDEX ON bulletin_boards(database_name);
            "#,
        )
        .await
    }

    pub async fn upsert_board_db(&mut self, board_dbname: &str) -> Result<()> {
        self.upsert_database(
            board_dbname,
            r#"
            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER AUTO_INCREMENT,
                created TIMESTAMP,
                signer_key VARCHAR,
                statement_timestamp TIMESTAMP,
                statement_kind VARCHAR,
                message BLOB,
                PRIMARY KEY id
            );
            "#,
        )
        .await
    }

    pub async fn delete_database(&mut self, database_name: &str) -> Result<()> {
        if self.client.has_database(database_name).await? {
            self.client.delete_database(database_name).await?;
        }
        Ok(())
    }

    /// Creates the database, only if it doesn't exist. It also creates
    /// the appropriate tables if they don't exist.
    async fn upsert_database(&mut self, database_name: &str, tables: &str) -> Result<()> {
        // create database if it doesn't exist
        if !self.client.has_database(database_name).await? {
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

    const INDEX_DB: &'static str = "testindexdb";
    const BOARD_DB: &'static str = "testdb";

    async fn set_up() -> BoardClient {
        let mut b = BoardClient::new("http://immudb:3322", "immudb", "immudb").await.unwrap();
        
        b.login().await.unwrap();
        // In case the previous test did not clean up properly
        b.delete_database(INDEX_DB).await.unwrap();
        b.delete_database(BOARD_DB).await.unwrap();
        
        b.upsert_index_db(INDEX_DB).await.unwrap();
        b.upsert_board_db(BOARD_DB).await.unwrap();
        b.create_board(INDEX_DB, BOARD_DB).await.unwrap();

        b
    }

    async fn tear_down(mut b: BoardClient) {
        b.delete_board(INDEX_DB, BOARD_DB).await.unwrap();
        b.delete_database(INDEX_DB).await.unwrap();
    }
    
    #[tokio::test]
    #[ignore]
    #[serial]
    pub async fn test_create_delete() {
        let mut b = set_up().await;
        
        assert!(b.has_database(INDEX_DB).await.unwrap());
        assert!(b.has_database(BOARD_DB).await.unwrap());
        let board = b.get_board(INDEX_DB, BOARD_DB).await.unwrap();
        assert_eq!(board.database_name, BOARD_DB);
        let board = b.get_board(INDEX_DB, "NOT FOUND").await;
        assert!(board.is_err());
        tear_down(b).await;
    }

    #[tokio::test]
    #[ignore]
    #[serial]
    pub async fn test_message_create_retrieve() {
        let mut b = set_up().await;
        let board = b.get_board(INDEX_DB, BOARD_DB).await.unwrap();
        assert_eq!(board.database_name, BOARD_DB);
        let board_message = BoardMessage {
            id: 1,
            created: 0,
            signer_key: "".to_string(),
            statement_timestamp: 0,
            statement_kind: "".to_string(),
            message: vec![],
        };
        let messages = vec![board_message];
        b.insert_messages(BOARD_DB, &messages).await.unwrap();
        let ret = b.get_messages(BOARD_DB, 0).await.unwrap();
        assert_eq!(messages, ret);
        tear_down(b).await;
    }
}