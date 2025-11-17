// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::pg_lock::PgLock;
pub use crate::types::hasura_types::*;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Local};
use deadpool_postgres::Transaction;
use tokio_postgres::row::Row;
use tracing::instrument;

impl TryFrom<Row> for PgLock {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        Ok(PgLock {
            key: item.get("key"),
            value: item.get("value"),
            expiry_date: item.get("expiry_date"),
        })
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn upsert_lock(
    hasura_transaction: &Transaction<'_>,
    key: &str,
    value: &str,
    expiry_date: DateTime<Local>,
) -> Result<PgLock> {
    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO
                    sequent_backend.lock (
                        key,
                        value,
                        expiry_date,
                        last_updated_at
                    )
                VALUES
                    ($1, $2, $3, now())
                ON CONFLICT (key) DO UPDATE
                SET
                    value = EXCLUDED.value,
                    expiry_date = EXCLUDED.expiry_date,
                    last_updated_at = now()
                WHERE
                    sequent_backend.lock.expiry_date < now() OR
                    sequent_backend.lock.value = EXCLUDED.value
                RETURNING
                    key, value, expiry_date, last_updated_at;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[&key.to_string(), &value.to_string(), &expiry_date],
        )
        .await
        .map_err(|err| anyhow!("Error running query: {}", err))?;

    if 1 == rows.len() {
        let mut locks = rows
            .into_iter()
            .map(|row| -> Result<PgLock> { row.try_into() })
            .collect::<Result<Vec<PgLock>>>()?;
        Ok(locks.remove(0))
    } else {
        Err(anyhow!("Couldn't upsert lock"))
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn delete_lock(
    hasura_transaction: &Transaction<'_>,
    key: &str,
    value: &str,
) -> Result<()> {
    let statement = hasura_transaction
        .prepare(
            r#"
                DELETE FROM
                    sequent_backend.lock
                WHERE
                    key = $1 AND
                    value = $2
                RETURNING key;
            "#,
        )
        .await?;

    let _rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&key.to_string(), &value.to_string()])
        .await
        .map_err(|err| anyhow!("Error running query: {}", err))?;

    Ok(())
}
