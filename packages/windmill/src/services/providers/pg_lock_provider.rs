// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::date::ISO8601;
use crate::services::pg_lock::PgLock;
use anyhow::Result;
use chrono::Duration;
use std::future::Future;
use std::pin::Pin;
use tracing::{info, instrument};
use uuid::Uuid;

#[instrument(skip(handler), err)]
pub async fn provide_hasura_transaction<F>(handler: F, lock_name: String) -> Result<()>
where
    F: FnOnce() -> Pin<Box<dyn Future<Output = Result<()>> + Send>>,
{
    let lock: PgLock = PgLock::acquire(
        lock_name,
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(120),
    )
    .await?;

    let result = handler().await;
    info!("result: {:?}", result);

    lock.release().await?;

    Ok(result?)
}
