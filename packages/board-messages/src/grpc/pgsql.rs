use anyhow::{anyhow, Result};
use bb8_postgres::bb8::Pool;
use bb8_postgres::bb8::PooledConnection;
use bb8_postgres::PostgresConnectionManager;
use std::borrow::BorrowMut;
use std::ops::Deref;
use std::ops::DerefMut;
use std::str::FromStr;
use std::time::SystemTime;
use tokio_postgres::Client;
use tokio_postgres::{config::Config, NoTls, Row};
use tracing::error;
use tracing::instrument;

use crate::braid::message::Message;
use crate::braid::newtypes::Timestamp;
use strand::serialization::{StrandDeserialize, StrandSerialize};

const INDEX_TABLE: &'static str = "BULLETIN_BOARDS";
const PG_DEFAULT_ENTRIES_TX_LIMIT: usize = 50;
const PG_DEFAULT_OFFSET: usize = 0;
const PG_DEFAULT_LIMIT: usize = 2500;

///////////////////////////////////////////////////////////////////////////
// PostgreSql client
//
///////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct PgsqlConnectionParams {
    host: String,
    port: u32,
    username: String,
    password: String,
}
impl PgsqlConnectionParams {
    pub fn new(host: &str, port: u32, username: &str, password: &str) -> PgsqlConnectionParams {
        PgsqlConnectionParams {
            host: host.to_string(),
            port,
            username: username.to_string(),
            password: password.to_string(),
        }
    }
    pub fn connection_string(&self) -> String {
        format!(
            "host={} port={} user={} password={}",
            self.host, self.port, self.username, self.password
        )
    }
    pub fn with_database(&self, db_name: &str) -> PgsqlDbConnectionParams {
        PgsqlDbConnectionParams::new(self, db_name)
    }
}

#[derive(Clone)]
pub struct PgsqlDbConnectionParams {
    connection: PgsqlConnectionParams,
    db_name: String,
}
impl PgsqlDbConnectionParams {
    pub fn new(connection: &PgsqlConnectionParams, db_name: &str) -> PgsqlDbConnectionParams {
        PgsqlDbConnectionParams {
            connection: connection.clone(),
            db_name: db_name.to_string(),
        }
    }
    pub fn connection_string(&self) -> String {
        format!(
            "{} dbname={}",
            self.connection.connection_string(),
            self.db_name
        )
    }
}

///////////////////////////////////////////////////////////////////////////
// Row structs
//
///////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct B3MessageRow {
    pub id: i64,
    pub created: Timestamp,
    // Base64 encoded spki der representation.
    pub sender_pk: String,
    pub statement_timestamp: Timestamp,
    pub statement_kind: String,
    pub message: Vec<u8>,
    pub version: String,
}

impl TryFrom<&Row> for B3MessageRow {
    type Error = anyhow::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        let id = row.get("id");
        let created: SystemTime = row.get("created");
        let sender_pk = row.get("sender_pk");
        let statement_timestamp: SystemTime = row.get("statement_timestamp");
        let statement_kind = row.get("statement_kind");
        let message = row.get("message");
        let version = row.get("version");

        let created = crate::timestamp_from_system_time(&created);
        let statement_timestamp = crate::timestamp_from_system_time(&statement_timestamp);

        Ok(B3MessageRow {
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

impl TryFrom<Message> for B3MessageRow {
    type Error = anyhow::Error;

    fn try_from(message: Message) -> Result<B3MessageRow> {
        Ok(B3MessageRow {
            id: 0,
            created: crate::timestamp(),
            statement_timestamp: message.statement.get_timestamp(),
            statement_kind: message.statement.get_kind().to_string(),
            message: message.strand_serialize()?,
            sender_pk: message.sender.pk.to_der_b64_string()?,
            version: crate::get_schema_version(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct B3IndexRow {
    pub id: i32,
    pub board_name: String,
    pub is_archived: bool,
}

impl TryFrom<&Row> for B3IndexRow {
    type Error = anyhow::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        let id = row.get("id");
        let board_name = row.get("board_name");
        let is_archived = row.get("is_archived");

        Ok(B3IndexRow {
            id,
            board_name,
            is_archived,
        })
    }
}

/// Utility function to create a database (will not pass a database parameter in the connection string).
pub async fn create_database(c: &PgsqlConnectionParams, dbname: &str) -> Result<()> {
    let (client, connection) = tokio_postgres::connect(&c.connection_string(), NoTls)
        .await
        .unwrap();

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    client
        .execute(&format!("CREATE DATABASE {}", dbname), &[])
        .await?;

    Ok(())
}

/// Utility function to drop a database (will not pass a database parameter in the connection string).
pub async fn drop_database(c: &PgsqlConnectionParams, dbname: &str) -> Result<()> {
    let (client, connection) = tokio_postgres::connect(&c.connection_string(), NoTls)
        .await
        .unwrap();

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    client
        .execute(&format!("DROP DATABASE IF EXISTS {}", dbname), &[])
        .await
        .unwrap();

    Ok(())
}

// Run ignored tests with
// cargo test <test_name> -- --include-ignored
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serial_test::serial;

    const PG_DATABASE: &'static str = "protocoldb";
    const PG_HOST: &'static str = "localhost";
    const PG_USER: &'static str = "postgres";
    const PG_PASSW: &'static str = "postgrespw";
    const PG_PORT: u32 = 49153;
    const TEST_BOARD: &'static str = "testboard";

    async fn set_up() -> XPgsqlB3Client {
        let c = PgsqlConnectionParams::new(PG_HOST, PG_PORT, PG_USER, PG_PASSW);
        drop_database(&c, PG_DATABASE).await.unwrap();
        create_database(&c, PG_DATABASE).await.unwrap();

        let c = c.with_database(PG_DATABASE);

        let mut client = XPgsqlB3Client::new(&c).await.unwrap();
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
        assert_eq!(board.unwrap().board_name, TEST_BOARD);
        let board = client.get_board("NOT FOUND").await.unwrap();
        assert!(board.is_none());
        client.delete_board(TEST_BOARD).await.unwrap();
        let board = client.get_board(TEST_BOARD).await.unwrap();
        assert!(board.is_none());
    }

    #[tokio::test]
    #[ignore]
    #[serial]
    pub async fn test_message_create_retrieve() {
        let mut client = set_up().await;
        client.create_board_ine(TEST_BOARD).await.unwrap();
        let board = client.get_board(TEST_BOARD).await.unwrap();
        assert_eq!(board.unwrap().board_name, TEST_BOARD);
        let board_message = B3MessageRow {
            id: 1,
            created: crate::timestamp(),
            sender_pk: "".to_string(),
            statement_timestamp: crate::timestamp(),
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
/*
pub struct PgsqlPooledB3Client {
    pool: Pool<PostgresConnectionManager<NoTls>>,
}

impl PgsqlPooledB3Client {
    /// Creates a new PgsqlB3Client from a bb8 pool
    pub fn new(pool: Pool<PostgresConnectionManager<NoTls>>) -> PgsqlPooledB3Client {
        PgsqlPooledB3Client { pool }
    }

    pub async fn from_params(params: &PgsqlDbConnectionParams) -> Result<PgsqlPooledB3Client> {
        let config = Config::from_str(&params.connection_string())?;
        let manager = PostgresConnectionManager::new(config, NoTls);
        let pool = Pool::builder().build(manager).await?;
        Ok(PgsqlPooledB3Client { pool })
    }

    pub async fn get_client(&self) -> Result<PooledConnection<PostgresConnectionManager<NoTls>>> {
        self.pool
            .get()
            .await
            .map_err(|e| anyhow!("Error retrieving connection from pool {e}"))
    }

    /// Creates the index table if it doesn't exist.
    #[instrument(skip(self))]
    pub async fn create_index_ine(&mut self) -> Result<()> {
        let mut client = self.get_client().await?;
        let transaction = client.transaction().await?;
        transaction
            .execute(
                &format!(
                    r#"
            CREATE TABLE IF NOT EXISTS {} (
                id SERIAL PRIMARY KEY,
                board_name VARCHAR,
                is_archived BOOLEAN
            );
            "#,
                    INDEX_TABLE
                ),
                &[],
            )
            .await?;
        transaction
            .execute(
                &format!(
                    r#"
            CREATE UNIQUE INDEX IF NOT EXISTS BOARD_NAME_IDX ON {}(board_name);
            "#,
                    INDEX_TABLE
                ),
                &[],
            )
            .await?;
        transaction.commit().await?;

        Ok(())
    }

    /// Creates the requested board table and adds it to the index, if it doesn't exist.
    #[instrument(skip(self))]
    pub async fn create_board_ine(&mut self, board: &str) -> Result<()> {
        let mut client = self.get_client().await?;
        let transaction = client.transaction().await?;
        transaction
            .execute(
                &format!(
                    r#"
            CREATE TABLE IF NOT EXISTS {} (
                id BIGSERIAL PRIMARY KEY,
                created TIMESTAMP,
                sender_pk VARCHAR,
                statement_timestamp TIMESTAMP,
                statement_kind VARCHAR,
                message BYTEA,
                version VARCHAR
            );
            "#,
                    board
                ),
                &[],
            )
            .await?;

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

    /// Get all messages whose id is bigger than `last_id`.
    pub async fn get_messages(
        &mut self,
        board_name: &str,
        last_id: i64,
    ) -> Result<Vec<B3MessageRow>> {
        let mut offset: usize = 0;
        let mut last_batch = self
            .get(board_name, last_id, Some(PG_DEFAULT_LIMIT), Some(offset))
            .await?;
        let mut messages = last_batch.clone();
        while PG_DEFAULT_LIMIT == last_batch.len() {
            offset += last_batch.len();
            last_batch = self
                .get(board_name, last_id, Some(PG_DEFAULT_LIMIT), Some(offset))
                .await?;
            messages.extend(last_batch.clone());
        }
        Ok(messages)
    }

    /// Get one messages matching id.
    pub async fn get_one_message(
        &mut self,
        board_name: &str,
        id: i64,
    ) -> Result<Option<B3MessageRow>> {
        self.get_one(board_name, id).await
    }

    pub async fn get_with_kind(
        &mut self,
        board: &str,
        kind: &str,
        sender_pk: &str,
    ) -> Result<Vec<B3MessageRow>> {
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
        WHERE sender_pk = $1 AND statement_kind = $2
        ORDER BY id;
        "#,
            board
        );

        let client = self.get_client().await?;
        let sql_query_response = client.query(&sql, &[&sender_pk, &kind]).await?;
        let messages = sql_query_response
            .iter()
            .map(B3MessageRow::try_from)
            .collect::<Result<Vec<B3MessageRow>>>()?;

        Ok(messages)
    }

    /// Get all boards in the index.
    pub async fn get_boards(&mut self) -> Result<Vec<B3IndexRow>> {
        let sql = format!(
            r#"
        SELECT
            id,
            board_name,
            is_archived
        FROM bulletin_boards
        WHERE is_archived = {}
        ORDER BY board_name
        "#,
            false
        );

        let client = self.get_client().await?;
        let sql_query_response = client.query(&sql, &[]).await?;
        let boards = sql_query_response
            .iter()
            .map(B3IndexRow::try_from)
            .collect::<Result<Vec<B3IndexRow>>>()?;

        Ok(boards)
    }

    /// Gets the requested board from the index.
    pub async fn get_board(&mut self, board_name: &str) -> Result<Option<B3IndexRow>> {
        let message_sql = format!(
            r#"
        SELECT
            id,
            board_name,
            is_archived
        FROM {}
        WHERE board_name = $1;
        "#,
            INDEX_TABLE
        );

        let client = self.get_client().await?;
        let sql_query_response = client.query(&message_sql, &[&board_name]).await?;
        let boards = sql_query_response
            .iter()
            .map(B3IndexRow::try_from)
            .collect::<Result<Vec<B3IndexRow>>>()?;

        if boards.len() > 0 {
            Ok(Some(boards[0].clone()))
        } else {
            Ok(None)
        }
    }

    /// Inserts messages into the requested board table.
    pub async fn insert_messages(
        &mut self,
        board_name: &str,
        messages: &Vec<B3MessageRow>,
    ) -> Result<()> {
        for chunk in messages.chunks(PG_DEFAULT_ENTRIES_TX_LIMIT) {
            let chunk_vec: Vec<B3MessageRow> = chunk.to_vec();
            self.insert(board_name, &chunk_vec).await?;
        }
        Ok(())
    }

    /// Deletes the requested board table and removes it from the index.
    pub async fn delete_board(&mut self, board_name: &str) -> Result<()> {
        let mut client = self.get_client().await?;
        let transaction = client.transaction().await?;
        let message_sql = format!(
            r#"
                DELETE from {} where
                board_name = $1
                AND
                is_archived = $2;
            "#,
            INDEX_TABLE
        );

        transaction
            .execute(&message_sql, &[&board_name, &false])
            .await?;
        transaction
            .execute(&format!("DROP TABLE IF EXISTS {};", board_name), &[])
            .await?;

        transaction.commit().await?;

        Ok(())
    }

    /// Clears all data in the database.
    pub async fn clear_database(&mut self) -> Result<()> {
        let mut client = self.get_client().await?;
        let transaction = client.transaction().await?;
        transaction
            .execute("drop schema if exists public cascade;", &[])
            .await?;
        transaction
            .execute("create schema if not exists public;", &[])
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn get(
        &mut self,
        board: &str,
        last_id: i64,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<B3MessageRow>> {
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

        let client = self.get_client().await?;
        let sql_query_response = client.query(&sql, &[&last_id]).await?;
        let messages = sql_query_response
            .iter()
            .map(B3MessageRow::try_from)
            .collect::<Result<Vec<B3MessageRow>>>()?;

        Ok(messages)
    }

    async fn insert(&mut self, board_name: &str, messages: &Vec<B3MessageRow>) -> Result<()> {
        let mut client = self.get_client().await?;
        let transaction = client.transaction().await?;

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

            let created = crate::system_time_from_timestamp(message.created).ok_or(anyhow!(
                "Could not extract system time from 'created' value"
            ))?;
            let statement_timestamp = crate::system_time_from_timestamp(message.created).ok_or(
                anyhow!("Could not extract system time from 'statement_timestamp' value"),
            )?;

            transaction
                .execute(
                    &message_sql,
                    &[
                        &created,
                        &message.sender_pk,
                        &statement_timestamp,
                        &message.statement_kind,
                        &message.message,
                        &message.version,
                    ],
                )
                .await?;
        }

        transaction.commit().await?;

        Ok(())
    }

    async fn get_one(&mut self, board_name: &str, id: i64) -> Result<Option<B3MessageRow>> {
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

        let client = self.get_client().await?;
        let rows = client.query(&sql, &[&id]).await?;

        if rows.len() > 0 {
            Ok(Some(B3MessageRow::try_from(&rows[0])?))
        } else {
            Ok(None)
        }
    }
}*/

pub struct ZPgsqlB3Client<'a> {
    client: PooledConnection<'a, PostgresConnectionManager<NoTls>>,
}

impl<'a> ZPgsqlB3Client<'a> {
    pub fn new(
        client: PooledConnection<'a, PostgresConnectionManager<NoTls>>,
    ) -> ZPgsqlB3Client<'a> {
        ZPgsqlB3Client { client }
    }

    pub async fn create_index_ine(&mut self) -> Result<()> {
        create_index_ine(self.client.deref_mut()).await
    }

    pub async fn create_board_ine(&mut self, board: &str) -> Result<()> {
        create_board_ine(self.client.deref_mut(), board).await
    }

    pub async fn get_messages(&self, board_name: &str, last_id: i64) -> Result<Vec<B3MessageRow>> {
        get_messages(self.client.deref(), board_name, last_id).await
    }

    pub async fn get_with_kind(
        &self,
        board: &str,
        kind: &str,
        sender_pk: &str,
    ) -> Result<Vec<B3MessageRow>> {
        get_with_kind(self.client.deref(), board, kind, sender_pk).await
    }

    pub async fn get_boards(&self) -> Result<Vec<B3IndexRow>> {
        get_boards(self.client.deref()).await
    }

    pub async fn insert_messages(
        &mut self,
        board_name: &str,
        messages: &Vec<B3MessageRow>,
    ) -> Result<()> {
        insert_messages(self.client.deref_mut(), board_name, messages).await
    }

    pub async fn delete_board(&mut self, board_name: &str) -> Result<()> {
        delete_board(self.client.deref_mut(), board_name).await
    }

    pub async fn clear_database(&mut self) -> Result<()> {
        clear_database(self.client.deref_mut()).await
    }
}

pub struct XPgsqlB3Client {
    client: Client,
}

impl XPgsqlB3Client {
    pub async fn new(params: &PgsqlDbConnectionParams) -> Result<XPgsqlB3Client> {
        let (client, connection) =
            tokio_postgres::connect(&params.connection_string(), NoTls).await?;

        // The connection object performs the actual communication with the database,
        // so spawn it off to run on its own.
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("connection error: {}", e);
            }
        });

        // let ret = PgsqlB3Client { client };
        let ret = XPgsqlB3Client { client };

        Ok(ret)
    }

    pub async fn create_index_ine(&mut self) -> Result<()> {
        create_index_ine(self.client.borrow_mut()).await
    }

    pub async fn create_board_ine(&mut self, board: &str) -> Result<()> {
        create_board_ine(self.client.borrow_mut(), board).await
    }

    pub async fn get_messages(&self, board_name: &str, last_id: i64) -> Result<Vec<B3MessageRow>> {
        get_messages(&self.client, board_name, last_id).await
    }

    pub async fn get_with_kind(
        &self,
        board: &str,
        kind: &str,
        sender_pk: &str,
    ) -> Result<Vec<B3MessageRow>> {
        get_with_kind(&self.client, board, kind, sender_pk).await
    }

    pub async fn get_with_kind_only(&self, board: &str, kind: &str) -> Result<Vec<B3MessageRow>> {
        get_with_kind_only(&self.client, board, kind).await
    }

    pub async fn get_boards(&self) -> Result<Vec<B3IndexRow>> {
        get_boards(&self.client).await
    }

    pub async fn insert_messages(
        &mut self,
        board_name: &str,
        messages: &Vec<B3MessageRow>,
    ) -> Result<()> {
        insert_messages(self.client.borrow_mut(), board_name, messages).await
    }

    pub async fn delete_board(&mut self, board_name: &str) -> Result<()> {
        delete_board(self.client.borrow_mut(), board_name).await
    }

    pub async fn clear_database(&mut self) -> Result<()> {
        clear_database(self.client.borrow_mut()).await
    }

    pub async fn get_board(&self, board_name: &str) -> Result<Option<B3IndexRow>> {
        get_board(&self.client, board_name).await
    }

    pub async fn get_one_message(&self, board_name: &str, id: i64) -> Result<Option<B3MessageRow>> {
        get_one_message(&self.client, board_name, id).await
    }

    pub async fn get_message_count(&self, board_name: &str) -> Result<i64> {
        get_message_count(&self.client, board_name).await
    }
}

/// Creates the index table if it doesn't exist.
#[instrument]
async fn create_index_ine(client: &mut Client) -> Result<()> {
    let transaction = client.transaction().await?;
    transaction
        .execute(
            &format!(
                r#"
        CREATE TABLE IF NOT EXISTS {} (
            id SERIAL PRIMARY KEY,
            board_name VARCHAR,
            is_archived BOOLEAN
        );
        "#,
                INDEX_TABLE
            ),
            &[],
        )
        .await?;
    transaction
        .execute(
            &format!(
                r#"
        CREATE UNIQUE INDEX IF NOT EXISTS BOARD_NAME_IDX ON {}(board_name);
        "#,
                INDEX_TABLE
            ),
            &[],
        )
        .await?;
    transaction.commit().await?;

    Ok(())
}

/// Creates the requested board table and adds it to the index, if it doesn't exist.
#[instrument]
async fn create_board_ine(client: &mut Client, board: &str) -> Result<()> {
    let transaction = client.transaction().await?;
    transaction
        .execute(
            &format!(
                r#"
        CREATE TABLE IF NOT EXISTS {} (
            id BIGSERIAL PRIMARY KEY,
            created TIMESTAMP,
            sender_pk VARCHAR,
            statement_timestamp TIMESTAMP,
            statement_kind VARCHAR,
            message BYTEA,
            version VARCHAR
        );
        "#,
                board
            ),
            &[],
        )
        .await?;

    let message_sql = &format!(
        r#"
        INSERT INTO {} (
            board_name,
            is_archived
        ) VALUES (
            $1,
            $2
        ) ON CONFLICT (board_name) DO NOTHING;
        "#,
        INDEX_TABLE
    );
    transaction.execute(message_sql, &[&board, &false]).await?;
    transaction.commit().await?;
    Ok(())
}

/// Get all messages whose id is bigger than `last_id`.
async fn get_messages(
    client: &Client,
    board_name: &str,
    last_id: i64,
) -> Result<Vec<B3MessageRow>> {
    let mut offset: usize = 0;
    let mut last_batch = get(
        client,
        board_name,
        last_id,
        Some(PG_DEFAULT_LIMIT),
        Some(offset),
    )
    .await?;
    let mut messages = last_batch.clone();
    while PG_DEFAULT_LIMIT == last_batch.len() {
        offset += last_batch.len();
        last_batch = get(
            client,
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

/// Get one messages matching id.
async fn get_message_count(client: &Client, board: &str) -> Result<i64> {
    let sql = format!(
        r#"
    SELECT count(*)
    FROM {}
    "#,
        board
    );

    let sql_query_response = client.query(&sql, &[]).await?;
    let count: i64 = sql_query_response[0].get(0);

    Ok(count)
}

/// Get one messages matching id.
async fn get_one_message(
    client: &Client,
    board_name: &str,
    id: i64,
) -> Result<Option<B3MessageRow>> {
    get_one(client, board_name, id).await
}

async fn get_with_kind(
    client: &Client,
    board: &str,
    kind: &str,
    sender_pk: &str,
) -> Result<Vec<B3MessageRow>> {
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
    WHERE sender_pk = $1 AND statement_kind = $2
    ORDER BY id;
    "#,
        board
    );

    let sql_query_response = client.query(&sql, &[&sender_pk, &kind]).await?;
    let messages = sql_query_response
        .iter()
        .map(B3MessageRow::try_from)
        .collect::<Result<Vec<B3MessageRow>>>()?;

    Ok(messages)
}

async fn get_with_kind_only(client: &Client, board: &str, kind: &str) -> Result<Vec<B3MessageRow>> {
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
    WHERE statement_kind = $1
    ORDER BY id;
    "#,
        board
    );

    let sql_query_response = client.query(&sql, &[&kind]).await?;
    let messages = sql_query_response
        .iter()
        .map(B3MessageRow::try_from)
        .collect::<Result<Vec<B3MessageRow>>>()?;

    Ok(messages)
}

/// Get all boards in the index.
async fn get_boards(client: &Client) -> Result<Vec<B3IndexRow>> {
    let sql = format!(
        r#"
    SELECT
        id,
        board_name,
        is_archived
    FROM {}
    WHERE is_archived = {}
    ORDER BY board_name
    "#,
        INDEX_TABLE, false
    );
    let sql_query_response = client.query(&sql, &[]).await?;
    let boards = sql_query_response
        .iter()
        .map(B3IndexRow::try_from)
        .collect::<Result<Vec<B3IndexRow>>>()?;

    Ok(boards)
}

/// Gets the requested board from the index.
async fn get_board(client: &Client, board_name: &str) -> Result<Option<B3IndexRow>> {
    let message_sql = format!(
        r#"
    SELECT
        id,
        board_name,
        is_archived
    FROM {}
    WHERE board_name = $1;
    "#,
        INDEX_TABLE
    );

    let sql_query_response = client.query(&message_sql, &[&board_name]).await?;
    let boards = sql_query_response
        .iter()
        .map(B3IndexRow::try_from)
        .collect::<Result<Vec<B3IndexRow>>>()?;

    if boards.len() > 0 {
        Ok(Some(boards[0].clone()))
    } else {
        Ok(None)
    }
}

use crate::braid::artifact::Configuration;
use strand::{context::Ctx, elgamal::Ciphertext};

async fn update_index<C: Ctx>(
    client: &mut Client,
    board_name: &str,
    configuration: &Configuration<C>,
) -> Result<()> {
    let transaction = client.transaction().await?;
    let message_sql = format!(
        r#"
            UPDATE {}
            set identifier = $X, threshold = $X, total_trustees = $Y
            where board_name = $1
            AND
            is_archived = $2;
        "#,
        INDEX_TABLE
    );

    transaction
        .execute(&message_sql, &[&board_name, &false])
        .await?;
    transaction
        .execute(&format!("DROP TABLE IF EXISTS {};", board_name), &[])
        .await?;

    transaction.commit().await?;

    Ok(())
}

/// Inserts the configuration into the requested board table, and updates the index.
async fn insert_configuration<C: Ctx>(
    client: &mut Client,
    board_name: &str,
    message: Message,
) -> Result<()> {
    let bytes = message
        .artifact
        .clone()
        .ok_or(anyhow!("Expected configuration message to have artifact"))?;
    let cfg = Configuration::<C>::strand_deserialize(&bytes)?;

    let created = crate::timestamp();

    let rows = vec![B3MessageRow {
        id: 0,
        created,
        statement_timestamp: message.statement.get_timestamp(),
        statement_kind: message.statement.get_kind().to_string(),
        message: message.strand_serialize()?,
        sender_pk: message.sender.pk.to_der_b64_string()?,
        version: crate::get_schema_version(),
    }];

    insert(client, board_name, &rows).await?;
    update_index(client, board_name, &cfg).await
}

/// Inserts messages into the requested board table.
async fn insert_messages(
    client: &mut Client,
    board_name: &str,
    messages: &Vec<B3MessageRow>,
) -> Result<()> {
    for chunk in messages.chunks(PG_DEFAULT_ENTRIES_TX_LIMIT) {
        let chunk_vec: Vec<B3MessageRow> = chunk.to_vec();
        insert(client, board_name, &chunk_vec).await?;
    }
    Ok(())
}

/// Deletes the requested board table and removes it from the index.
async fn delete_board(client: &mut Client, board_name: &str) -> Result<()> {
    let transaction = client.transaction().await?;
    let message_sql = format!(
        r#"
            DELETE from {} where
            board_name = $1
            AND
            is_archived = $2;
        "#,
        INDEX_TABLE
    );

    transaction
        .execute(&message_sql, &[&board_name, &false])
        .await?;
    transaction
        .execute(&format!("DROP TABLE IF EXISTS {};", board_name), &[])
        .await?;

    transaction.commit().await?;

    Ok(())
}

/// Clears all data in the database.
async fn clear_database(client: &mut Client) -> Result<()> {
    let transaction = client.transaction().await?;
    transaction
        .execute("drop schema if exists public cascade;", &[])
        .await?;
    transaction
        .execute("create schema if not exists public;", &[])
        .await?;
    transaction.commit().await?;
    Ok(())
}

async fn get(
    client: &Client,
    board: &str,
    last_id: i64,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Vec<B3MessageRow>> {
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

    let sql_query_response = client.query(&sql, &[&last_id]).await?;
    let messages = sql_query_response
        .iter()
        .map(B3MessageRow::try_from)
        .collect::<Result<Vec<B3MessageRow>>>()?;

    Ok(messages)
}

async fn insert(client: &mut Client, board_name: &str, messages: &Vec<B3MessageRow>) -> Result<()> {
    // Start a new transaction
    let transaction = client.transaction().await?;

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

        let created = crate::system_time_from_timestamp(message.created).ok_or(anyhow!(
            "Could not extract system time from 'created' value"
        ))?;
        let statement_timestamp = crate::system_time_from_timestamp(message.created).ok_or(
            anyhow!("Could not extract system time from 'statement_timestamp' value"),
        )?;

        transaction
            .execute(
                &message_sql,
                &[
                    &created,
                    &message.sender_pk,
                    &statement_timestamp,
                    &message.statement_kind,
                    &message.message,
                    &message.version,
                ],
            )
            .await?;
    }

    transaction.commit().await?;

    Ok(())
}

async fn get_one(client: &Client, board_name: &str, id: i64) -> Result<Option<B3MessageRow>> {
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

    let rows = client.query(&sql, &[&id]).await?;

    if rows.len() > 0 {
        Ok(Some(B3MessageRow::try_from(&rows[0])?))
    } else {
        Ok(None)
    }
}

/*
enum ClientSource<'a> {
    Direct(tokio_postgres::Client),
    Pooled(PooledConnection<'a, PostgresConnectionManager<NoTls>>),
}
impl<'a> ClientSource<'a> {
    fn get(&mut self) -> &mut tokio_postgres::Client {
        let ret = match self {
            ClientSource::Direct(client) => client.borrow_mut(),

            ClientSource::Pooled(client) => client.deref_mut(),
        };

        ret
    }
}

pub struct PgsqlB3Client<'a> {
    // client: tokio_postgres::Client,
    cs: ClientSource<'a>,
}

impl<'a> PgsqlB3Client<'a> {
    /// Creates a new PgsqlB3Client using a direct db connection. The underlying connection will be closed when the client is dropped.
    pub async fn new(params: &PgsqlDbConnectionParams) -> Result<PgsqlB3Client<'a>> {
        let (client, connection) =
            tokio_postgres::connect(&params.connection_string(), NoTls).await?;

        // The connection object performs the actual communication with the database,
        // so spawn it off to run on its own.
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("connection error: {}", e);
            }
        });

        // let ret = PgsqlB3Client { client };
        let ret = PgsqlB3Client {
            cs: ClientSource::Direct(client),
        };

        Ok(ret)
    }

    /// Creates a new PgsqlB3Client using a direct db connection. The underlying connection will be closed when the client is dropped.
    pub fn from_pooled(
        client: PooledConnection<'a, PostgresConnectionManager<NoTls>>,
    ) -> PgsqlB3Client {
        let cs = ClientSource::Pooled(client);

        PgsqlB3Client { cs }
    }

    /// Creates the index table if it doesn't exist.
    #[instrument(skip(self))]
    pub async fn create_index_ine(&mut self) -> Result<()> {
        let transaction = self.cs.get().transaction().await?;
        transaction
            .execute(
                &format!(
                    r#"
            CREATE TABLE IF NOT EXISTS {} (
                id SERIAL PRIMARY KEY,
                board_name VARCHAR,
                is_archived BOOLEAN
            );
            "#,
                    INDEX_TABLE
                ),
                &[],
            )
            .await?;
        transaction
            .execute(
                &format!(
                    r#"
            CREATE UNIQUE INDEX IF NOT EXISTS BOARD_NAME_IDX ON {}(board_name);
            "#,
                    INDEX_TABLE
                ),
                &[],
            )
            .await?;
        transaction.commit().await?;

        Ok(())
    }

    /// Creates the requested board table and adds it to the index, if it doesn't exist.
    #[instrument(skip(self))]
    pub async fn create_board_ine(&mut self, board: &str) -> Result<()> {
        let transaction = self.cs.get().transaction().await?;
        transaction
            .execute(
                &format!(
                    r#"
            CREATE TABLE IF NOT EXISTS {} (
                id BIGSERIAL PRIMARY KEY,
                created TIMESTAMP,
                sender_pk VARCHAR,
                statement_timestamp TIMESTAMP,
                statement_kind VARCHAR,
                message BYTEA,
                version VARCHAR
            );
            "#,
                    board
                ),
                &[],
            )
            .await?;

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

    /// Get all messages whose id is bigger than `last_id`.
    pub async fn get_messages(
        &mut self,
        board_name: &str,
        last_id: i64,
    ) -> Result<Vec<B3MessageRow>> {
        let mut offset: usize = 0;
        let mut last_batch = self
            .get(board_name, last_id, Some(PG_DEFAULT_LIMIT), Some(offset))
            .await?;
        let mut messages = last_batch.clone();
        while PG_DEFAULT_LIMIT == last_batch.len() {
            offset += last_batch.len();
            last_batch = self
                .get(board_name, last_id, Some(PG_DEFAULT_LIMIT), Some(offset))
                .await?;
            messages.extend(last_batch.clone());
        }
        Ok(messages)
    }

    /// Get one messages matching id.
    pub async fn get_one_message(
        &mut self,
        board_name: &str,
        id: i64,
    ) -> Result<Option<B3MessageRow>> {
        self.get_one(board_name, id).await
    }

    pub async fn get_with_kind(
        &mut self,
        board: &str,
        kind: &str,
        sender_pk: &str,
    ) -> Result<Vec<B3MessageRow>> {
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
        WHERE sender_pk = $1 AND statement_kind = $2
        ORDER BY id;
        "#,
            board
        );

        let sql_query_response = self.cs.get().query(&sql, &[&sender_pk, &kind]).await?;
        let messages = sql_query_response
            .iter()
            .map(B3MessageRow::try_from)
            .collect::<Result<Vec<B3MessageRow>>>()?;

        Ok(messages)
    }

    /// Get all boards in the index.
    pub async fn get_boards(&mut self) -> Result<Vec<B3IndexRow>> {
        let sql = format!(
            r#"
        SELECT
            id,
            board_name,
            is_archived
        FROM bulletin_boards
        WHERE is_archived = {}
        ORDER BY board_name
        "#,
            false
        );
        let sql_query_response = self.cs.get().query(&sql, &[]).await?;
        let boards = sql_query_response
            .iter()
            .map(B3IndexRow::try_from)
            .collect::<Result<Vec<B3IndexRow>>>()?;

        Ok(boards)
    }

    /// Gets the requested board from the index.
    pub async fn get_board(&mut self, board_name: &str) -> Result<Option<B3IndexRow>> {
        let message_sql = format!(
            r#"
        SELECT
            id,
            board_name,
            is_archived
        FROM {}
        WHERE board_name = $1;
        "#,
            INDEX_TABLE
        );

        let sql_query_response = self.cs.get().query(&message_sql, &[&board_name]).await?;
        let boards = sql_query_response
            .iter()
            .map(B3IndexRow::try_from)
            .collect::<Result<Vec<B3IndexRow>>>()?;

        if boards.len() > 0 {
            Ok(Some(boards[0].clone()))
        } else {
            Ok(None)
        }
    }

    /// Inserts messages into the requested board table.
    pub async fn insert_messages(
        &mut self,
        board_name: &str,
        messages: &Vec<B3MessageRow>,
    ) -> Result<()> {
        for chunk in messages.chunks(PG_DEFAULT_ENTRIES_TX_LIMIT) {
            let chunk_vec: Vec<B3MessageRow> = chunk.to_vec();
            self.insert(board_name, &chunk_vec).await?;
        }
        Ok(())
    }

    /// Deletes the requested board table and removes it from the index.
    pub async fn delete_board(&mut self, board_name: &str) -> Result<()> {
        let transaction = self.cs.get().transaction().await?;
        let message_sql = format!(
            r#"
                DELETE from {} where
                board_name = $1
                AND
                is_archived = $2;
            "#,
            INDEX_TABLE
        );

        transaction
            .execute(&message_sql, &[&board_name, &false])
            .await?;
        transaction
            .execute(&format!("DROP TABLE IF EXISTS {};", board_name), &[])
            .await?;

        transaction.commit().await?;

        Ok(())
    }

    /// Clears all data in the database.
    pub async fn clear_database(&mut self) -> Result<()> {
        let transaction = self.cs.get().transaction().await?;
        transaction
            .execute("drop schema if exists public cascade;", &[])
            .await?;
        transaction
            .execute("create schema if not exists public;", &[])
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn get(
        &mut self,
        board: &str,
        last_id: i64,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<B3MessageRow>> {
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

        let sql_query_response = self.cs.get().query(&sql, &[&last_id]).await?;
        let messages = sql_query_response
            .iter()
            .map(B3MessageRow::try_from)
            .collect::<Result<Vec<B3MessageRow>>>()?;

        Ok(messages)
    }

    async fn insert(&mut self, board_name: &str, messages: &Vec<B3MessageRow>) -> Result<()> {
        // Start a new transaction
        let transaction = self.cs.get().transaction().await?;

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

            let created = crate::system_time_from_timestamp(message.created).ok_or(anyhow!(
                "Could not extract system time from 'created' value"
            ))?;
            let statement_timestamp = crate::system_time_from_timestamp(message.created).ok_or(
                anyhow!("Could not extract system time from 'statement_timestamp' value"),
            )?;

            transaction
                .execute(
                    &message_sql,
                    &[
                        &created,
                        &message.sender_pk,
                        &statement_timestamp,
                        &message.statement_kind,
                        &message.message,
                        &message.version,
                    ],
                )
                .await?;
        }

        transaction.commit().await?;

        Ok(())
    }

    async fn get_one(&mut self, board_name: &str, id: i64) -> Result<Option<B3MessageRow>> {
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

        let rows = self.cs.get().query(&sql, &[&id]).await?;

        if rows.len() > 0 {
            Ok(Some(B3MessageRow::try_from(&rows[0])?))
        } else {
            Ok(None)
        }
    }
}
*/
