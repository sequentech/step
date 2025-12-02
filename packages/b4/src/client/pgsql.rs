// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use bb8_postgres::bb8::PooledConnection;
use bb8_postgres::PostgresConnectionManager;
use std::borrow::BorrowMut;
use std::ops::Deref;
use std::ops::DerefMut;
use std::time::Instant;
use std::time::SystemTime;
use strand::signature::StrandSignaturePk;
use tokio_postgres::Client;
use tokio_postgres::{NoTls, Row};
use tracing::error;
use tracing::instrument;

use crate::messages::artifact::Configuration;
use crate::messages::message::Message;
use crate::messages::newtypes::Timestamp;
use crate::messages::statement::StatementType;
use strand::context::Ctx;
use strand::serialization::{StrandDeserialize, StrandSerialize};

const INDEX_TABLE: &'static str = "INDEX";
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
/*
The authoritative source of all message data is
in the serialized bytes in the message row.
All other rows are extracted from these bytes
for the purposes of validation at the database level
and convenient inspection. Note that validation of messages
does not depend on the bulletin board, and is performed
additionally by each trustee.
*/
pub struct B3MessageRow {
    pub id: i64,
    pub created: Timestamp,
    // Base64 encoded spki der representation.
    pub sender_pk: String,
    pub statement_timestamp: Timestamp,
    pub statement_kind: String,
    pub batch: i32,
    // When signing mixes, specifies which mix in the chain is being signed.
    // This allows creating a unique index for which otherwise there would be duplicate
    // mix signature messages
    pub mix_number: i32,
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
        let batch = row.get("batch");
        let mix_number = row.get("mix_number");
        let created = crate::timestamp_from_system_time(&created);
        let statement_timestamp = crate::timestamp_from_system_time(&statement_timestamp);

        Ok(B3MessageRow {
            id,
            created,
            sender_pk,
            statement_timestamp,
            statement_kind,
            batch,
            mix_number,
            message,
            version,
        })
    }
}

impl TryFrom<Message> for B3MessageRow {
    type Error = anyhow::Error;

    fn try_from(message: Message) -> Result<B3MessageRow> {
        let batch: i32 = message.statement.get_batch_number().try_into()?;
        let mix_number: i32 = message.statement.get_mix_number().try_into()?;

        Ok(B3MessageRow {
            id: 0,
            created: crate::timestamp(),
            statement_timestamp: message.statement.get_timestamp(),
            statement_kind: message.statement.get_kind().to_string(),
            message: message.strand_serialize()?,
            batch,
            mix_number,
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
    pub cfg_id: String,
    pub threshold_no: i32,
    pub trustees_no: i32,
    pub last_message_kind: String,
    pub last_updated: Timestamp,
    pub message_count: i32,
    pub batch_count: i32,
}

impl TryFrom<&Row> for B3IndexRow {
    type Error = anyhow::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        let id = row.get("id");
        let board_name = row.get("board_name");
        let is_archived = row.get("is_archived");
        let cfg_id = row.try_get("cfg_id").unwrap_or("".to_string());
        let threshold_no = row.try_get("threshold_no").unwrap_or(0);
        let trustees_no = row.try_get("trustees_no").unwrap_or(0);
        let last_message_kind = row.try_get("last_message_kind").unwrap_or("".to_string());
        let last_updated = row
            .try_get("last_updated")
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let message_count = row.try_get("message_count").unwrap_or(0);
        let batch_count = row.try_get("batch_count").unwrap_or(0);

        let last_updated = crate::timestamp_from_system_time(&last_updated);

        Ok(B3IndexRow {
            id,
            board_name,
            is_archived,
            cfg_id,
            threshold_no,
            trustees_no,
            last_message_kind,
            last_updated,
            message_count,
            batch_count,
        })
    }
}

/// Utility function to create a database (will not pass a database parameter in the connection string).
#[instrument(err, skip(c))]
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
#[instrument(err, skip(c))]
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

/// Version using database connection pool.
pub struct PooledPgsqlB3Client<'a> {
    client: PooledConnection<'a, PostgresConnectionManager<NoTls>>,
}

impl<'a> PooledPgsqlB3Client<'a> {
    pub fn new(
        client: PooledConnection<'a, PostgresConnectionManager<NoTls>>,
    ) -> PooledPgsqlB3Client<'a> {
        PooledPgsqlB3Client { client }
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
        kind: &StatementType,
        sender_pk: &StrandSignaturePk,
    ) -> Result<Vec<B3MessageRow>> {
        get_with_kind(
            self.client.deref(),
            board,
            &kind.to_string(),
            &sender_pk.to_der_b64_string()?,
        )
        .await
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

// Non-pool version.
pub struct PgsqlB3Client {
    client: Client,
}

impl PgsqlB3Client {
    pub async fn new(params: &PgsqlDbConnectionParams) -> Result<PgsqlB3Client> {
        let (client, connection) =
            tokio_postgres::connect(&params.connection_string(), NoTls).await?;

        // The connection object performs the actual communication with the database,
        // so spawn it off to run on its own.
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("connection error: {}", e);
            }
        });

        let ret = PgsqlB3Client { client };

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
        kind: StatementType,
        sender_pk: &StrandSignaturePk,
    ) -> Result<Vec<B3MessageRow>> {
        get_with_kind(
            &self.client,
            board,
            &kind.to_string(),
            &sender_pk.to_der_b64_string()?,
        )
        .await
    }

    pub async fn get_with_kind_only(
        &self,
        board: &str,
        kind: StatementType,
    ) -> Result<Vec<B3MessageRow>> {
        get_with_kind_only(&self.client, board, &kind.to_string()).await
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

    pub async fn insert_configuration<C: Ctx>(
        &mut self,
        board_name: &str,
        configuration: Message,
    ) -> Result<()> {
        insert_configuration::<C>(self.client.borrow_mut(), board_name, configuration).await
    }

    pub async fn insert_ballots<C: Ctx>(
        &mut self,
        board_name: &str,
        ballots: Message,
    ) -> Result<()> {
        insert_ballots::<C>(self.client.borrow_mut(), board_name, ballots).await
    }
}

/// Creates the index table if it doesn't exist.
#[instrument(err, skip(client))]
async fn create_index_ine(client: &mut Client) -> Result<()> {
    let transaction = client.transaction().await?;
    transaction
        .execute(
            &format!(
                r#"
        CREATE TABLE IF NOT EXISTS {} (
            id SERIAL PRIMARY KEY,
            board_name VARCHAR UNIQUE,
            is_archived BOOLEAN,
            cfg_id VARCHAR,
            threshold_no INT,
            trustees_no INT,
            last_message_kind VARCHAR,
            last_updated TIMESTAMP,
            message_count INT,
            batch_count INT DEFAULT 0,
            UNIQUE(board_name)
        );
        "#,
                INDEX_TABLE
            ),
            &[],
        )
        .await?;

    /*transaction
    .execute(
        &format!(
            r#"
    CREATE UNIQUE INDEX IF NOT EXISTS BOARD_NAME_IDX ON {}(board_name);
    "#,
            INDEX_TABLE
        ),
        &[],
    )
    .await?;*/
    transaction.commit().await?;

    Ok(())
}

/// Creates the requested board table and adds it to the index, if it doesn't exist.
#[instrument(err, skip(client))]
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
            batch INT,
            mix_number INT,
            message BYTEA,
            version VARCHAR,
            UNIQUE (sender_pk, statement_kind, batch, mix_number)
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
#[instrument(err, skip(client))]
async fn get_messages(
    client: &Client,
    board_name: &str,
    last_id: i64,
) -> Result<Vec<B3MessageRow>> {
    let now = Instant::now();

    let mut offset: usize = 0;
    let mut start = get(
        client,
        board_name,
        last_id,
        Some(PG_DEFAULT_LIMIT),
        Some(offset),
    )
    .await?;
    // let mut messages = last_batch.clone();
    let mut last_batch_len = start.len();
    // while PG_DEFAULT_LIMIT == last_batch.len() {
    while PG_DEFAULT_LIMIT == last_batch_len {
        offset += last_batch_len;
        let next = get(
            client,
            board_name,
            last_id,
            Some(PG_DEFAULT_LIMIT),
            Some(offset),
        )
        .await?;
        last_batch_len = next.len();
        // messages.extend(last_batch.clone());
        start.extend(next);
    }

    tracing::trace!(
        "pgsql::get_messages: query time: {}ms",
        now.elapsed().as_millis()
    );

    Ok(start)
}

#[instrument(err, skip(client))]
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
#[instrument(err, skip(client))]
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
        batch,
        mix_number,
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

#[instrument(err, skip(client))]
async fn get_with_kind_only(client: &Client, board: &str, kind: &str) -> Result<Vec<B3MessageRow>> {
    let sql = format!(
        r#"
    SELECT
        id,
        created,
        sender_pk,
        statement_timestamp,
        statement_kind,
        batch,
        mix_number,
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
#[instrument(err, skip(client))]
async fn get_boards(client: &Client) -> Result<Vec<B3IndexRow>> {
    let sql = format!(
        r#"
    SELECT
        id,
        board_name,
        is_archived,
        cfg_id,
        threshold_no,
        trustees_no,
        last_message_kind,
        last_updated,
        message_count,
        batch_count
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
#[instrument(err, skip(client))]
async fn get_board(client: &Client, board_name: &str) -> Result<Option<B3IndexRow>> {
    let message_sql = format!(
        r#"
        SELECT
            id,
            board_name,
            is_archived,
            cfg_id,
            threshold_no,
            trustees_no,
            last_message_kind,
            last_updated,
            message_count,
            batch_count
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

#[instrument(err, skip(client))]
async fn update_index<C: Ctx>(
    client: &mut Client,
    board_name: &str,
    configuration: &Configuration<C>,
) -> Result<()> {
    let transaction = client.transaction().await?;
    let message_sql = format!(
        r#"
            UPDATE {}
            set cfg_id = $1, threshold_no = $2, trustees_no = $3
            where board_name = $4
            AND
            is_archived = $5;
        "#,
        INDEX_TABLE
    );

    transaction
        .execute(
            &message_sql,
            &[
                &configuration.id.to_string(),
                &(configuration.threshold as i32),
                &(configuration.trustees.len() as i32),
                &board_name,
                &false,
            ],
        )
        .await?;

    transaction.commit().await?;

    Ok(())
}

/// Inserts the configuration into the requested board table, and updates the index.
#[instrument(err, skip(client))]
async fn insert_configuration<C: Ctx>(
    client: &mut Client,
    board_name: &str,
    configuration: Message,
) -> Result<()> {
    if configuration.statement.get_kind() != StatementType::Configuration {
        return Err(anyhow!("Expected message to be a configuration"));
    }

    let bytes = configuration
        .artifact
        .clone()
        .ok_or(anyhow!("Expected configuration message to have artifact"))?;
    let cfg = Configuration::<C>::strand_deserialize(&bytes)?;

    let created = crate::timestamp();

    // cfg batch and mix_number is always zero
    let batch: i32 = 0;
    let mix_number: i32 = 0;

    let rows = vec![B3MessageRow {
        id: 0,
        created,
        statement_timestamp: configuration.statement.get_timestamp(),
        statement_kind: configuration.statement.get_kind().to_string(),
        message: configuration.strand_serialize()?,
        batch,
        mix_number,
        sender_pk: configuration.sender.pk.to_der_b64_string()?,
        version: crate::get_schema_version(),
    }];

    insert(client, board_name, &rows).await?;

    update_index(client, board_name, &cfg).await
}

/// Inserts the ballots into the requested board table.
#[instrument(err, skip(client))]
async fn insert_ballots<C: Ctx>(
    client: &mut Client,
    board_name: &str,
    ballots: Message,
) -> Result<()> {
    if ballots.statement.get_kind() != StatementType::Ballots {
        return Err(anyhow!("Expected message to be Ballots"));
    }

    if ballots.artifact.is_none() {
        return Err(anyhow!("Expected ballots message to have artifact"));
    }

    let created = crate::timestamp();

    let batch: i32 = ballots.statement.get_batch_number().try_into()?;
    // ballots mix_number is always zero
    let mix_number: i32 = 0;

    let rows = vec![B3MessageRow {
        id: 0,
        created,
        statement_timestamp: ballots.statement.get_timestamp(),
        statement_kind: ballots.statement.get_kind().to_string(),
        message: ballots.strand_serialize()?,
        batch,
        mix_number,
        sender_pk: ballots.sender.pk.to_der_b64_string()?,
        version: crate::get_schema_version(),
    }];

    insert(client, board_name, &rows).await
}

/// Inserts messages into the requested board table.
#[instrument(err, skip(client, messages))]
async fn insert_messages(
    client: &mut Client,
    board_name: &str,
    messages: &Vec<B3MessageRow>,
) -> Result<()> {
    for chunk in messages.chunks(PG_DEFAULT_ENTRIES_TX_LIMIT) {
        cfg_if::cfg_if! { if #[cfg(feature = "sqlcopy")] {
            insert_copy(client, board_name, chunk).await?;
        }
        else {
            insert(client, board_name, chunk).await?;
        }}
    }
    Ok(())
}

/// Deletes the requested board table and removes it from the index.
#[instrument(err, skip(client))]
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
#[instrument(err, skip(client))]
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

#[instrument(err, skip(client))]
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
        batch,
        mix_number,
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

#[instrument(err, skip(client, messages))]
async fn insert(client: &mut Client, board_name: &str, messages: &[B3MessageRow]) -> Result<()> {
    // Start a new transaction
    let transaction = client.transaction().await?;
    // http://disq.us/p/2ficy6c
    // https://stackoverflow.com/questions/52432459/postgresql-serialized-inserts-interleaving-sequence-numbers
    let lock = format!("select pg_advisory_xact_lock(hashtext($1))");
    transaction.execute(&lock, &[&board_name]).await?;
    // let lock = format!("select pg_advisory_xact_lock(id) from {}", board_name);
    // transaction.execute(&lock, &[]).await?;
    let mut batches: i32 = 0;

    for message in messages {
        if message.statement_kind == StatementType::Ballots.to_string() {
            batches = batches + 1;
        }

        let message_sql = format!(
            r#"
            INSERT INTO {} (
                created,
                sender_pk,
                statement_timestamp,
                statement_kind,
                batch,
                mix_number,
                message,
                version
            ) VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                $7,
                $8
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
                    &message.batch,
                    &message.mix_number,
                    &message.message,
                    &message.version,
                ],
            )
            .await?;
    }

    transaction.commit().await?;

    // We do not care if any of these operations fail, they are statistics
    if let Some(last) = messages.last() {
        let Ok(transaction) = client.transaction().await else {
            return Ok(());
        };

        let message_sql = format!(
            r#"
           UPDATE {}
           SET
           last_message_kind = $1,
           message_count = (SELECT COUNT(*) FROM {}),
           batch_count = batch_count + $2,
           last_updated = localtimestamp
           WHERE board_name = $3
        "#,
            INDEX_TABLE, board_name,
        );

        let Ok(_) = transaction
            .execute(&message_sql, &[&last.statement_kind, &batches, &board_name])
            .await
        else {
            return Ok(());
        };

        let _ = transaction.commit().await;
    }

    Ok(())
}

#[instrument(err, skip(client))]
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

cfg_if::cfg_if! { if #[cfg(feature = "sqlcopy")] {

    use futures::pin_mut;
    use tokio_postgres::binary_copy::BinaryCopyInWriter;
    use tokio_postgres::types::{ToSql, Type};

    // Uses the COPY postgresql command
    async fn insert_copy(
        client: &mut Client,
        board_name: &str,
        messages: &[B3MessageRow],
    ) -> Result<()> {
        // Start a new transaction
        let transaction = client.transaction().await?;
        let types: Vec<Type> = vec![
            Type::TIMESTAMP,
            Type::VARCHAR,
            Type::TIMESTAMP,
            Type::VARCHAR,
            Type::INT4,
            Type::INT4,
            Type::BYTEA,
            Type::VARCHAR,
        ];
        let stmt = format!("COPY {} (created, sender_pk, statement_timestamp, statement_kind, batch, mix_number, message, version) FROM STDIN BINARY", board_name);

        // http://disq.us/p/2ficy6c
        // https://stackoverflow.com/questions/52432459/postgresql-serialized-inserts-interleaving-sequence-numbers
        let lock = format!("select pg_advisory_xact_lock(hashtext($1))");
        transaction.execute(&lock, &[&board_name]).await?;
        let sink = transaction.copy_in(&stmt).await?;
        let writer = BinaryCopyInWriter::new(sink, &types);
        let batches = _write(writer, &messages).await?;
        transaction.commit().await?;

        // We do not care if any of these operations fail, they are statistics
        if let Some(last) = messages.last() {
            let Ok(transaction) = client.transaction().await else {
                return Ok(());
            };

            let message_sql = format!(
                r#"
            UPDATE {}
            SET
            last_message_kind = $1,
            message_count = (SELECT COUNT(*) FROM {}),
            batch_count = batch_count + $2,
            last_updated = localtimestamp
            WHERE board_name = $3
            "#,
                INDEX_TABLE, board_name,
            );

            let Ok(_) = transaction
                .execute(&message_sql, &[&last.statement_kind, &batches, &board_name])
                .await
            else {
                return Ok(());
            };

            let _ = transaction.commit().await;
        }

        Ok(())
    }

    async fn _write(writer: BinaryCopyInWriter, messages: &[B3MessageRow]) -> Result<i32> {
        pin_mut!(writer);

        let mut row: Vec<&'_ (dyn ToSql + Sync)> = vec![];
        let mut ts: Vec<(SystemTime, SystemTime)> = vec![];
        let mut batches = 0;

        for message in messages {
            if message.statement_kind == StatementType::Ballots.to_string() {
                batches = batches + 1;
            }

            let created = crate::system_time_from_timestamp(message.created).ok_or(anyhow!(
                "Could not extract system time from 'created' value"
            ))?;
            let statement_timestamp = crate::system_time_from_timestamp(message.created).ok_or(
                anyhow!("Could not extract system time from 'statement_timestamp' value"),
            )?;

            ts.push((created, statement_timestamp));
        }
        for (i, message) in messages.iter().enumerate() {
            row.clear();
            row.push(&ts[i].0);
            row.push(&message.sender_pk);
            row.push(&ts[i].1);
            row.push(&message.statement_kind);
            row.push(&message.batch);
            row.push(&message.mix_number);
            row.push(&message.message);
            row.push(&message.version);

            writer.as_mut().write(&row).await?;
        }

        writer.finish().await?;

        Ok(batches)
    }
}}

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

    async fn set_up() -> PgsqlB3Client {
        let c = PgsqlConnectionParams::new(PG_HOST, PG_PORT, PG_USER, PG_PASSW);
        drop_database(&c, PG_DATABASE).await.unwrap();
        create_database(&c, PG_DATABASE).await.unwrap();

        let c = c.with_database(PG_DATABASE);

        let mut client = PgsqlB3Client::new(&c).await.unwrap();
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
            batch: 0,
            mix_number: 0,
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
