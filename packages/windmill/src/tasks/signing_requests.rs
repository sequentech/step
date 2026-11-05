// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The periodic jobs of signing requests: expiring the requests whose time
//! is up, and sending again the tasks of completed requests whose execution
//! has not reported back.

use crate::services::database::get_hasura_pool;
use crate::services::initialization_record::sweep_initialization_logs;
use crate::services::signing::actions::reports::{
    start_all_held_tally_reports, sweep_held_mails, StoredReports,
};
use crate::services::signing::executors::default_registry;
use crate::services::signing::pdf::S3RevisionStore;
use crate::services::signing::requests::{expire_overdue_requests, redispatch_unexecuted_requests};
use crate::types::error::Result;
use anyhow::anyhow;
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use tracing::{info, instrument};

/// Seconds between two passes of each job.
pub const SIGNING_JOBS_INTERVAL_SECONDS: u64 = 60;

async fn hasura_client() -> anyhow::Result<DbClient> {
    get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|error| anyhow!("Error getting the hasura client: {error:?}"))
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0, expires = 60)]
pub async fn expire_signing_requests() -> Result<()> {
    let mut client = hasura_client().await?;
    let expired = expire_overdue_requests(&mut client).await?;
    info!("expired {expired} signing requests");
    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0, expires = 60)]
pub async fn sweep_signing_executions() -> Result<()> {
    let mut client = hasura_client().await?;
    let staged = sweep_initialization_logs(&mut client).await?;
    info!("staged {staged} initialization records for electoral logging");
    let sent = redispatch_unexecuted_requests(&mut client, &default_registry()).await?;
    info!("sent {sent} signing executions again");
    // Held tally reports whose request didn't start after their tally.
    let started = start_all_held_tally_reports(&mut client).await?;
    info!("started {started} held tally reports' signing requests");
    // Held e-mails of released reports that no task sent.
    let mailed = sweep_held_mails(&mut client, &S3RevisionStore, &StoredReports).await?;
    info!("mailed {mailed} held reports");
    Ok(())
}
