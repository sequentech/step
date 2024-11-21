// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::database::get_hasura_pool;
use crate::postgres::lock;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Local};
use deadpool_postgres::Client as DbClient;
use sequent_core::services::connection;
use sequent_core::services::date::ISO8601;
use tokio_postgres::row::Row;
use tracing::instrument;

#[derive(Debug)]
pub struct PgLock {
    pub key: String,
    pub value: String,
    pub expiry_date: Option<DateTime<Local>>,
}

impl PgLock {
    #[instrument(skip(self), err)]
    pub async fn update_expiry(&self) -> Result<()> {
        let mut hasura_db_client: DbClient = get_hasura_pool()
            .await
            .get()
            .await
            .with_context(|| "Error acquiring hasura connection pool")?;
        let hasura_transaction = hasura_db_client
            .transaction()
            .await
            .with_context(|| "Error acquiring hasura transaction")?;
        let new_expiry_date: DateTime<Local> = ISO8601::now() + Duration::seconds(120);
        lock::upsert_lock(
            &hasura_transaction,
            self.key.as_str(),
            self.value.as_str(),
            new_expiry_date,
        )
        .await?;
        hasura_transaction
            .commit()
            .await
            .with_context(|| "error comitting transaction")?;
        Ok(())
    }

    #[instrument(err)]
    pub async fn acquire(
        key: String,
        value: String,
        expiry_date: DateTime<Local>,
    ) -> Result<PgLock> {
        let mut hasura_db_client: DbClient = get_hasura_pool()
            .await
            .get()
            .await
            .with_context(|| "Error acquiring hasura connection pool")?;
        let hasura_transaction = hasura_db_client
            .transaction()
            .await
            .with_context(|| "Error acquiring hasura transaction")?;
        let lock = lock::upsert_lock(
            &hasura_transaction,
            key.as_str(),
            value.as_str(),
            expiry_date,
        )
        .await?;
        hasura_transaction
            .commit()
            .await
            .with_context(|| "error comitting transaction")?;

        Ok(lock)
    }

    #[instrument(err)]
    pub async fn release(self) -> Result<()> {
        let mut hasura_db_client: DbClient = get_hasura_pool()
            .await
            .get()
            .await
            .with_context(|| "Error acquiring hasura connection pool")?;
        let hasura_transaction = hasura_db_client
            .transaction()
            .await
            .with_context(|| "Error acquiring hasura transaction")?;
        lock::delete_lock(&hasura_transaction, self.key.as_str(), self.value.as_str()).await?;
        hasura_transaction
            .commit()
            .await
            .with_context(|| "error comitting transaction")
    }
}
