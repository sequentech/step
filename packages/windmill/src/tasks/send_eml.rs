// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::types::error::Error;
use crate::{services::consolidation::send_eml_service::send_eml_service, types::error::Result};
use anyhow::anyhow;
use celery::error::TaskError;
use tracing::{info, instrument};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn send_eml_task(
    tenant_id: String,
    election_id: String,
    area_id: String,
    tally_session_id: String,
) -> Result<()> {
    // Spawn the task using an async block
    let handle = tokio::task::spawn_blocking({
        move || {
            tokio::runtime::Handle::current().block_on(async move {
                send_eml_service(&tenant_id, &election_id, &area_id, &tally_session_id)
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
