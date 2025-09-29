// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::database::get_hasura_pool;
use crate::services::protocol_manager::get_immudb_client;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use immudb_rs::{Client as ImmudbClient, TxMode};
use rusqlite::Connection as SqliteConnection;
use rusqlite::Transaction as SqliteTransaction;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use tokio::task;
use tracing::instrument;

#[instrument(skip(handler), err)]
pub async fn provide_transaction<F>(handler: F, mut db_client: DbClient) -> Result<()>
where
    for<'a> F: FnOnce(&'a Transaction<'a>) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>,
{
    let hasura_transaction = db_client.transaction().await?;

    let res = handler(&hasura_transaction).await;

    match res {
        Ok(_) => {
            hasura_transaction
                .commit()
                .await
                .map_err(|e| anyhow!("Commit failed manage_election_dates: {}", e))?;
        }
        Err(err) => {
            hasura_transaction
                .rollback()
                .await
                .with_context(|| format!("Rollback error after transaction error {:?}", err))?;
            return Err(anyhow!("{}", err).into());
        }
    }

    Ok(())
}

#[instrument(skip(handler), err)]
pub async fn provide_sqlite_transaction<F>(handler: F, database_path: &Path) -> Result<()>
where
    for<'a> F: FnOnce(&'a SqliteTransaction<'a>) -> Result<()> + Send + 'static,
{
    let db_path = database_path.to_path_buf();

    task::spawn_blocking(move || {
        let mut conn = SqliteConnection::open(db_path).context("Error opening sqlite database")?;
        let tx = conn
            .transaction()
            .context("Error starting sqlite database transaction")?;

        match handler(&tx) {
            Ok(_) => {
                tx.commit()
                    .context("Commit failed for sqlite transaction")?;
                Ok(())
            }
            Err(err) => {
                tx.rollback().with_context(|| {
                    format!("Rollback error after transaction error: {:?}", err)
                })?;
                Err(err)
            }
        }
    })
    .await?
}

#[instrument(skip(handler), err)]
pub async fn provide_hasura_transaction<F>(handler: F) -> Result<()>
where
    for<'a> F: FnOnce(&'a Transaction<'a>) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>,
{
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| anyhow!("Error getting hasura client {}", e))?;

    provide_transaction(handler, hasura_db_client).await
}

#[instrument(skip(handler), err)]
pub async fn provide_transaction_immudb<F>(
    handler: F,
    mut client: ImmudbClient,
    immudb_db: &str,
) -> Result<()>
where
    for<'a> F: FnOnce(
        &'a mut ImmudbClient,
        &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>,
{
    // Open the session for the client
    client
        .open_session(immudb_db)
        .await
        .map_err(|e| anyhow!("Failed to open session: {}", e))?;

    // Start the transaction
    let tx_id = client
        .new_tx(TxMode::ReadWrite)
        .await
        .map_err(|e| anyhow!("Failed to start transaction: {}", e))?;

    // Run the handler (database operations inside the transaction)
    let res = handler(&mut client, &tx_id).await;
    // Commit or Rollback based on result
    match res {
        Ok(_) => {
            client
                .commit(&tx_id)
                .await
                .map_err(|e| anyhow!("Commit failed: {}", e))?;
        }
        Err(err) => {
            client
                .rollback(&tx_id)
                .await
                .with_context(|| format!("Rollback error after transaction error: {:?}", err))?;
            return Err(anyhow!("{}", err).into());
        }
    }

    // Close the session
    client
        .close_session()
        .await
        .map_err(|e| anyhow!("Failed to close session: {}", e))?;

    Ok(())
}

#[instrument(skip(handler), err)]
pub async fn provide_immudb_transaction<F>(handler: F, immudb_db: &str) -> Result<()>
where
    for<'a> F: FnOnce(
        &'a mut ImmudbClient,
        &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>,
{
    let mut client: ImmudbClient = get_immudb_client()
        .await
        .map_err(|e| anyhow!("Error getting Immudb client: {}", e))?;

    // Call the function that manages the session, transaction, and lifecycle
    provide_transaction_immudb(handler, client, immudb_db).await
}
