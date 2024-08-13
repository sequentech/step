// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::consolidation::create_transmission_package_service::create_transmission_package_service;
use crate::services::consolidation::send_transmission_package_service::send_transmission_package_service;
use crate::types::error::Error;
use crate::types::error::Result;
use anyhow::anyhow;
use celery::error::TaskError;
use tracing::{info, instrument};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn create_transmission_package_task(
    tenant_id: String,
    election_id: String,
    area_id: String,
    tally_session_id: String,
) -> Result<()> {
    // Spawn the task using an async block
    let handle = tokio::task::spawn_blocking({
        move || {
            tokio::runtime::Handle::current().block_on(async move {
                create_transmission_package_service(
                    &tenant_id,
                    &election_id,
                    &area_id,
                    &tally_session_id,
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

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn send_transmission_package_task(
    tenant_id: String,
    election_id: String,
    area_id: String,
    tally_session_id: String,
) -> Result<()> {
    // Spawn the task using an async block
    let handle = tokio::task::spawn_blocking({
        move || {
            tokio::runtime::Handle::current().block_on(async move {
                send_transmission_package_service(
                    &tenant_id,
                    &election_id,
                    &area_id,
                    &tally_session_id,
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
