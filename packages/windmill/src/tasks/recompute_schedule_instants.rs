// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The daily tz database check of future scheduled dates (VOTE-LIFECYCLE):
//! see `services::schedule_recompute`. It records pending changes only.

use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::services::schedule_recompute::check_all;
use crate::types::error::Result;
use celery::error::TaskError;
use chrono::Utc;
use tracing::{info, instrument};

/// Seconds between two checks.
pub const RECOMPUTE_INTERVAL_SECONDS: u64 = 24 * 60 * 60;

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 600, max_retries = 0, expires = 3600)]
pub async fn recompute_schedule_instants() -> Result<()> {
    provide_hasura_transaction(|hasura_transaction| {
        Box::pin(async move {
            let pending = check_all(hasura_transaction, Utc::now()).await?;
            info!("{pending} scheduled dates move with the current tz database");
            Ok(())
        })
    })
    .await?;
    Ok(())
}
