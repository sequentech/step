// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::vote_receipt;
use crate::types::error::Error;
use crate::types::error::Result;
use anyhow::anyhow;
use celery::error::TaskError;
use sequent_core::types::date_time::{DateFormat, TimeZone};
use tracing::instrument;
use sequent_core::types::hasura::core::TasksExecution;

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn create_vote_receipt(
    element_id: String,
    ballot_id: String,
    ballot_tracker_url: String,
    tenant_id: String,
    election_event_id: String,
    election_id: String,
    area_id: String,
    voter_id: String,
    time_zone: Option<TimeZone>,
    date_format: Option<DateFormat>,
) -> Result<()> {
    // Spawn the task using an async block
    let handle = tokio::task::spawn_blocking({
        move || {
            tokio::runtime::Handle::current().block_on(async move {
                vote_receipt::create_vote_receipt_task(
                    element_id,
                    ballot_id,
                    ballot_tracker_url,
                    tenant_id,
                    election_event_id,
                    election_id,
                    area_id,
                    voter_id,
                    time_zone,
                    date_format,
                )
                .await
                .map_err(|err| anyhow!("{}", err))
            })
        }
    });

    // Await the result and handle JoinError explicitly
    match handle.await {
        Ok(inner_result) => inner_result.map_err(|err| Error::from(err.context("Task failed"))),
        Err(join_error) => Err(Error::from(anyhow!("Task panicked: {}", join_error))),
    }?;

    Ok(())
}
