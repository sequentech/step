// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::database::get_hasura_pool;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
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
