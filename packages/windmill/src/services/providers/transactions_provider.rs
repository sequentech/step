// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::database::get_hasura_pool;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use std::future::Future;
use tracing::instrument;

#[instrument(skip(handler), err)]
pub async fn provide_transaction<F, Fut>(handler: F, mut db_client: DbClient) -> Result<()>
where
    F: Fn(&Transaction<'_>) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    let hasura_transaction = db_client.transaction().await?;

    let res = handler(&hasura_transaction).await;

    match res {
        Ok(_) => {
            let commit = hasura_transaction
                .commit()
                .await
                .map_err(|e| anyhow!("Commit failed manage_election_dates: {}", e));
            commit?;
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
pub async fn provide_hasura_transaction<F, Fut>(handler: F) -> Result<()>
where
    F: Fn(&Transaction<'_>) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| anyhow!("Error getting hasura client {}", e))?;
    provide_transaction(handler, hasura_db_client).await
}
