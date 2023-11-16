// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura::lock::*;
use crate::services::date::ISO8601;
use anyhow::{anyhow, Result};
use chrono::NaiveDateTime;
use sequent_core::services::connection;
use tracing::instrument;

#[derive(Debug)]
pub struct PgLock {
    pub key: String,
    pub value: String,
    pub expiry_date: Option<NaiveDateTime>,
}

impl PgLock {
    #[instrument(skip(auth_headers))]
    pub async fn acquire(
        auth_headers: connection::AuthHeaders,
        key: String,
        value: String,
        expiry_date: Option<NaiveDateTime>,
    ) -> Result<PgLock> {
        let expiry_str = expiry_date.clone().map(|naive| ISO8601::from_date(&naive));
        let lock_data = upsert_lock(auth_headers, key.clone(), value.clone(), expiry_str)
            .await?
            .data
            .expect("expected data")
            .insert_sequent_backend_lock_one;

        match lock_data {
            None => Err(anyhow!("Unable to acquire lock '{}'", key)),
            Some(lock) => Ok(PgLock {
                key: lock.key,
                value: lock.value,
                expiry_date: lock
                    .expiry_date
                    .map(|d| ISO8601::to_date(d.as_str()).unwrap()),
            }),
        }
    }

    #[instrument(skip(auth_headers))]
    pub async fn release(self, auth_headers: connection::AuthHeaders) -> Result<()> {
        let affected_rows = delete_lock(auth_headers, self.key.clone(), self.value.clone())
            .await?
            .data
            .expect("expected data")
            .delete_sequent_backend_lock
            .expect("expected delete_sequent_backend_lock")
            .affected_rows;

        if 0 == affected_rows {
            Err(anyhow!("Unable to unlock '{}'", self.key.clone()))
        } else {
            Ok(())
        }
    }
}
