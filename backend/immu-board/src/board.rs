use anyhow::{anyhow, Result};
use tracing::{instrument};

use std::fmt::Debug;
use immudb_rs::{Client, Row, sql_value::Value};

#[derive(Debug)]
pub struct Board {
    client: Client,
    index_dbname: String,
    board_dbname: String,
}

#[derive(Debug, Clone)]
pub struct BoardMessage {
    id: i64,
    created: i64,
    signer_key: Vec<u8>,
    statement_timestamp: i64,
    statement_kind: String,
    message: Vec<u8>
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
        let mut signer_key = vec![];
        let mut statement_timestamp = 0;
        let mut statement_kind = String::from("");
        let mut message = vec![];
        for (column, value) in row.columns.iter().zip(row.values.iter()) {
            match column.as_str() {
                "id" => assign_value!(Value::N, value, id),
                "created" => assign_value!(Value::N, value, created),
                "signer_key" => assign_value!(Value::Bs, value, signer_key),
                "statement_timestamp" => assign_value!(Value::N, value, statement_timestamp),
                "statement_kind" => assign_value!(Value::S, value, statement_kind),
                "message" => assign_value!(Value::Bs, value, message),
                _ => return Err(anyhow!("invalid column found")),
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

impl Board {
    #[instrument]
    pub async fn new(
        server_url: &str,
        index_dbname: String,
        board_dbname: String,
    ) -> Result<Board> {
        Ok(Board {
            client: Client::new(&server_url).await?,
            index_dbname: index_dbname,
            board_dbname: board_dbname,
        })
    }

    /// Get all messages whose id is bigger than `last_id`
    pub async fn get_messages(
        &mut self, last_id: i64
    ) -> Result<Vec<BoardMessage>>
    {
        let sql = format!(r#"
        SELECT
            id,
            created,
            signer_key,
            statement_timestamp,
            statement_kind,
            message
        FROM messages
        WHERE id > {}
        "#, last_id);
        let sql_query_response = self.client.sql_query(&sql, vec![]).await?;
        let messages = sql_query_response
            .get_ref()
            .rows
            .iter()
            .map(BoardMessage::try_from)
            .collect::<Result<Vec<BoardMessage>>>()?;
        Ok(messages)
    }

    pub async fn insert_messages(
        &mut self, messages: &Vec<BoardMessage>
    ) -> Result<()>
    {
        let sql: String = messages
            .iter()
            .try_fold(
                String::new(),
                |mut acc_sql: String, message: &BoardMessage| -> Result<String>
                {
                    let message_sql = format!(r#"
                        INSERT INTO messages(
                            id,
                            created,
                            signer_key,
                            statement_timestamp,
                            statement_kind,
                            message)
                        VALUES ({}, {}, {:?}, {}, {}, {:?});
                        "#,
                        message.id,
                        message.created,
                        message.signer_key,
                        message.statement_timestamp,
                        message.statement_kind,
                        message.message
                    );
                    acc_sql.push_str(&message_sql);
                    Ok(acc_sql)
                }
            )?;
        self.client.sql_exec(&sql, vec![]).await?;
        Ok(())
    }
}
