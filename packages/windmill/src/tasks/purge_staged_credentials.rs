// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Removes expired messaging credentials staged by the direct package intake
//! from the vault, for every tenant, whether or not that tenant uploads again.
//! Bounded per pass; rows another transaction holds wait for the next pass.
//! Apply refuses an expired entry regardless, so this only bounds retention.

use crate::services::database::get_hasura_pool;
use crate::services::import::configuration_package_credentials::sweep_expired_everywhere;
use crate::types::error::Result;
use anyhow::Context;
use celery::error::TaskError;
use tracing::{info, instrument};

/// Seconds between passes.
pub const PURGE_STAGED_CREDENTIALS_INTERVAL_SECS: u64 = 15 * 60;
/// Expired staging rows cleared per pass, so a pass never holds many locks.
pub const PURGE_BATCH: i64 = 200;

/// Windmill beat schedules it every `PURGE_STAGED_CREDENTIALS_INTERVAL_SECS`.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 300, max_retries = 0, expires = 60)]
pub async fn purge_staged_credentials() -> Result<()> {
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .context("Error connecting to the database")?;
    let tx = client
        .transaction()
        .await
        .context("Error opening a transaction")?;
    let cleared = sweep_expired_everywhere(&tx, chrono::Utc::now(), PURGE_BATCH)
        .await
        .context("Error sweeping expired staged credentials")?;
    tx.commit().await.context("Error committing the sweep")?;
    if cleared > 0 {
        // Counts only: never a tenant, document or key name.
        info!("Removed {cleared} expired staged messaging credentials");
    }
    Ok(())
}
