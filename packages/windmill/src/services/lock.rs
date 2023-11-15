// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use chrono::NaiveDateTime;
use anyhow::Result;

pub struct PgLock {
    pub key: String,
    pub value: String,
    pub expiry_date: Option<NaiveDateTime>,
}

pub async fn acquire_lock(key: String, value: String, expiry_date: Option<NaiveDateTime>) -> Result<PgLock> {
    Ok(PgLock {
        key,
        value,
        expiry_date
    })
}

impl PgLock {
    pub async fn acquire(key: String, value: String, expiry_date: Option<NaiveDateTime>) -> Result<PgLock> {
        Ok(PgLock {
            key,
            value,
            expiry_date
        })
    }

    pub async fn release(self, lock: PgLock) -> Result<()> {
        Ok(())
    }
}