// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura::lock::*;
use crate::services::date::ISO8601;
use anyhow::{anyhow, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use sequent_core::services::connection;

pub struct PgLock {
    pub key: String,
    pub value: String,
    pub expiry_date: Option<NaiveDateTime>,
}

impl PgLock {
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
            None => Err(anyhow!("Unable to unlock '{}'", key)),
            Some(lock) => Ok(PgLock {
                key: lock.key,
                value: lock.value,
                expiry_date: lock
                    .expiry_date
                    .map(|d| ISO8601::to_date(d.as_str()).unwrap()),
            }),
        }
    }

    pub async fn release(self, auth_headers: connection::AuthHeaders) -> Result<()> {
        Ok(())
    }
}
