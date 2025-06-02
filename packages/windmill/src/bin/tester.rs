// src/main.rs

// Assuming board_client.rs and schema.rs are in the src directory
// REMOVED: use windmill::services::protocol_manager; // Not used in this snippet
use anyhow::{anyhow, Context, Result};
// REMOVED: use tokio_stream::{Stream, StreamExt}; // StreamExt from futures_util is usually preferred with tokio
use tokio_stream::{Stream, StreamExt};
use tracing::Level;
// CORRECTED: Using SqlValue directly for clarity where the enum type is needed.
// Value is often a type alias for sql_value::Value or similar, but we'll use SqlValue.
use immudb_rs::{Client, NamedParam, Row, SqlValue, TxMode, sql_value::Value}; // SqlValue is the key enum type here
use windmill::types::resources::{Aggregate, DataList, OrderDirection, TotalAggregate};

// --- Application Configuration ---
const SERVER_URL: &str = "http://immudb:3322"; // Default immudb gRPC port
const USERNAME: &str = "immudb";
const PASSWORD: &str = "immudb";
const TARGET_DB_NAME: &str = "app_streaming_db"; // Database for the application
const TARGET_TABLE_NAME: &str = "app_stream_data";
const NUM_ROWS_TO_PERSIST: usize = 200_000;
const INSERT_BATCH_SIZE: usize = 1000; // Number of rows per transaction batch

fn setup_tracing() {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO) // Default to INFO
        .with_env_filter(format!("info,{}=debug", env!("CARGO_PKG_NAME"))) // module specific
        .init();
}

async fn get_table_row_count(client: &mut Client, table_name: &str) -> Result<Option<usize>> {
    let count_sql = format!("SELECT COUNT(*) FROM {}", table_name);


    let sql_query_response = client.sql_query(&count_sql, vec![]).await?;
    let mut rows_iter = sql_query_response
        .get_ref()
        .rows
        .iter()
        .map(Aggregate::try_from);

    let aggregate = rows_iter
        // get the first item
        .next()
        // unwrap the Result and Option
        .ok_or(anyhow!("No aggregate found"))??;

    let count = aggregate.count as usize;
    tracing::debug!("Table '{}' has {} rows.", table_name, count);
    return Ok(Some(count));
}

async fn ensure_table_and_data(client: &mut Client) -> Result<()> {
    tracing::info!(
        "Checking table '{}' for {} rows...",
        TARGET_TABLE_NAME,
        NUM_ROWS_TO_PERSIST
    );

    let current_row_count_opt = get_table_row_count(client, TARGET_TABLE_NAME).await?;

    match current_row_count_opt {
        Some(count) if count == NUM_ROWS_TO_PERSIST => {
            tracing::info!(
                "Table '{}' exists and has the correct number of rows ({}). No action needed.",
                TARGET_TABLE_NAME,
                count
            );
            return Ok(());
        }
        Some(count) => {
            tracing::info!(
                "Table '{}' exists but has {} rows. Expected {}. Clearing and re-populating.",
                TARGET_TABLE_NAME,
                count,
                NUM_ROWS_TO_PERSIST
            );
            let delete_sql = format!("DELETE FROM {}", TARGET_TABLE_NAME);
            client
                .sql_exec(&delete_sql, vec![])
                .await
                .context(format!("Failed to delete rows from table '{}'", TARGET_TABLE_NAME))?;
            tracing::info!("Cleared existing rows from table '{}'.", TARGET_TABLE_NAME);
        }
        None => {
            tracing::info!("Table '{}' does not exist. Creating and populating.", TARGET_TABLE_NAME);
            let create_table_sql = format!(
                "CREATE TABLE {} (id INTEGER, message VARCHAR, PRIMARY KEY id)",
                TARGET_TABLE_NAME
            );
            client
                .sql_exec(&create_table_sql, vec![])
                .await
                .context(format!("Failed to create table '{}'", TARGET_TABLE_NAME))?;
            tracing::info!("Table '{}' created.", TARGET_TABLE_NAME);
        }
    }

    tracing::info!(
        "Populating table '{}' with {} rows in batches of {}...",
        TARGET_TABLE_NAME,
        NUM_ROWS_TO_PERSIST,
        INSERT_BATCH_SIZE
    );

    for i_batch_start in (0..NUM_ROWS_TO_PERSIST).step_by(INSERT_BATCH_SIZE) {
        let tx_id = client.new_tx(TxMode::ReadWrite).await?;
        tracing::debug!(
            "Started transaction {} for inserting rows {} to {}",
            tx_id,
            i_batch_start,
            (i_batch_start + INSERT_BATCH_SIZE).min(NUM_ROWS_TO_PERSIST) - 1
        );

        for i in i_batch_start..(i_batch_start + INSERT_BATCH_SIZE).min(NUM_ROWS_TO_PERSIST) {
            let insert_sql = format!(
                "INSERT INTO {} (id, message) VALUES (@id_{}, @msg_{})",
                TARGET_TABLE_NAME, i % INSERT_BATCH_SIZE, i % INSERT_BATCH_SIZE
            );
            let params = vec![
                NamedParam {
                    name: format!("id_{}", i % INSERT_BATCH_SIZE),
                    value: Some(SqlValue {
                        value: Some(Value::N(i as i64)),
                    }),
                },
                NamedParam {
                    name: format!("msg_{}", i % INSERT_BATCH_SIZE),

                    value: Some(SqlValue {
                        value: Some(Value::S(format!("Persisted message {}", i))),
                    }),
                },
            ];
            client
                .tx_sql_exec(&insert_sql, &tx_id, params)
                .await
                .context(format!("Failed to insert row {} in transaction {}", i, tx_id))?;
        }
        client
            .commit(&tx_id)
            .await
            .context(format!("Failed to commit transaction {}", tx_id))?;

        if (i_batch_start / INSERT_BATCH_SIZE) % 10 == 0 || i_batch_start + INSERT_BATCH_SIZE >= NUM_ROWS_TO_PERSIST {
            tracing::info!(
                "Inserted up to row {}.",
                (i_batch_start + INSERT_BATCH_SIZE).min(NUM_ROWS_TO_PERSIST) - 1
            );
        }
    }
    tracing::info!("Finished populating table '{}'.", TARGET_TABLE_NAME);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_tracing();
    tracing::info!("Starting Immudb Streaming Tool...");

    let mut client = Client::new(SERVER_URL, USERNAME, PASSWORD).await?; // .await was removed as Client::new is not async in immudb-rs 0.5.0

    client.login().await.context("Failed to login")?;
    tracing::info!("Logged in successfully to default database.");

    if !client.has_database(TARGET_DB_NAME).await.unwrap_or(false) {
         tracing::info!("Database '{}' does not exist. Creating it...", TARGET_DB_NAME);
         client.create_database(TARGET_DB_NAME)
            .await
            .context(format!("Failed to create database: {}", TARGET_DB_NAME))?;
        tracing::info!("Database '{}' created.", TARGET_DB_NAME);
    } else {
        tracing::info!("Database '{}' already exists.", TARGET_DB_NAME);
    }

    client
        .open_session(TARGET_DB_NAME) // open_session needs user/pass in immudb-rs 0.5.0
        .await
        .context(format!("Failed to open session for database: {}", TARGET_DB_NAME))?;
    tracing::info!("Session opened for database '{}'.", TARGET_DB_NAME);

    ensure_table_and_data(&mut client).await?;

    tracing::info!("Performing streaming SQL query to retrieve all rows from '{}'...", TARGET_TABLE_NAME);
    let select_sql = format!("SELECT id, message FROM {}", TARGET_TABLE_NAME);
    
    // For immudb-rs, streaming_sql_query returns a stream directly
    let stream_result = client
        .streaming_sql_query(&select_sql, vec![])
        .await
        .context("Failed to execute streaming SQL query")?;
    let mut stream = stream_result.into_inner();

    let mut retrieved_row_count = 0;
    let mut log_count_interval = 0;

    // The stream from immudb-rs yields Result<Row, Status>
    while let Some(row_result) = stream.next().await {
        match row_result {
            Ok(_row) => { // _row is immudb_rs::Row
                retrieved_row_count += 1;
                log_count_interval += 1;

                if log_count_interval >= NUM_ROWS_TO_PERSIST / 20 {
                    tracing::debug!("Retrieved {} rows so far...", retrieved_row_count);
                    log_count_interval = 0;
                }
            }
            Err(e) => {
                return Err(anyhow!("Error receiving row from stream: {:?}", e));
            }
        }
    }

    tracing::info!(
        "Streaming query finished. Total rows retrieved: {}. Expected: {}",
        retrieved_row_count,
        NUM_ROWS_TO_PERSIST
    );

    if retrieved_row_count == NUM_ROWS_TO_PERSIST {
        tracing::info!("Successfully retrieved all expected rows via streaming.");
    } else {
        tracing::warn!(
            "Retrieved row count ({}) does not match expected count ({}).",
            retrieved_row_count,
            NUM_ROWS_TO_PERSIST
        );
    }

    tracing::info!("Closing session...");
    client.close_session().await.context("Failed to close session")?;
    tracing::info!("Session closed.");

    client.logout().await.context("Failed to logout")?;
    tracing::info!("Logged out. Application finished.");

    Ok(())
}