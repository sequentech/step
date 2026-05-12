// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! Miru consolidation tasks (transmission packages and signatures).
use crate::services::consolidation::upload_signature_service::upload_transmission_package_signature_service;
use anyhow::Result as AnyhowResult;
use anyhow::{anyhow, Context};
use tracing::{info, instrument};

/// Celery workers for Miru transmission package creation and delivery.
mod miru_consolidation_tasks {
    #![allow(missing_docs)]
    #![allow(clippy::missing_docs_in_private_items)]

    use crate::services::consolidation::create_transmission_package_service::create_transmission_package_service;
    use crate::services::consolidation::send_transmission_package_service::send_transmission_package_service;
    use crate::services::tasks_execution::{update_complete, update_fail};
    use crate::types::error::Error;
    use crate::types::error::Result;
    use anyhow::{anyhow, Context};
    use celery::error::TaskError;
    use sequent_core::types::hasura::core::TasksExecution;
    use tracing::{info, instrument};

    /// Celery task: create a transmission package.
    #[instrument(err)]
    #[wrap_map_err::wrap_map_err(TaskError)]
    #[celery::task(max_retries = 0)]
    pub async fn create_transmission_package_task(
        tenant_id: String,
        election_id: String,
        area_id: String,
        tally_session_id: String,
        force: bool,
        task_execution: TasksExecution,
    ) -> Result<()> {
        let task_execution_clone: TasksExecution = task_execution.clone();
        let handle = tokio::task::spawn_blocking({
            move || {
                tokio::runtime::Handle::current().block_on(async move {
                    match Box::pin(create_transmission_package_service(
                        &tenant_id,
                        &election_id,
                        &area_id,
                        &tally_session_id,
                        force,
                    ))
                    .await
                    {
                        Ok(()) => Ok(()),
                        Err(err) => {
                            info!(
                                "Captured backtrace inside spawn_blocking:\n{}",
                                err.backtrace()
                            );
                            update_fail(&task_execution_clone, &err.to_string()).await?;
                            Err(err.context("Failed to create transmission package"))
                        }
                    }
                })
            }
        });

        match handle.await {
            Ok(inner_result) => inner_result.map_err(|err| Error::from(err.context("Task failed"))),
            Err(join_error) => Err(Error::from(anyhow!("Task panicked: {join_error}"))),
        }?;

        update_complete(&task_execution, None)
            .await
            .context("Failed to update task execution status to COMPLETED")?;

        Ok(())
    }

    /// Celery task: send a transmission package.
    #[instrument(err)]
    #[wrap_map_err::wrap_map_err(TaskError)]
    #[celery::task(max_retries = 0)]
    pub async fn send_transmission_package_task(
        tenant_id: String,
        election_id: String,
        area_id: String,
        tally_session_id: String,
    ) -> Result<()> {
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
                    .map_err(|err| anyhow!("{err}"))
                })
            }
        });

        match handle.await {
            Ok(inner_result) => inner_result.map_err(|err| Error::from(err.context("Task failed"))),
            Err(join_error) => Err(Error::from(anyhow!("Task panicked: {join_error}"))),
        }?;

        Ok(())
    }
}

pub use miru_consolidation_tasks::{
    create_transmission_package_task, send_transmission_package_task,
};

/// Uploads a trustee-signed transmission package using credentials supplied with the document.
///
/// # Errors
///
/// Returns an error if the blocking worker panics or signature upload fails.
#[instrument(err)]
pub async fn upload_signature_task(
    tenant_id: String,
    election_id: String,
    area_id: String,
    tally_session_id: String,
    trustee_name: String,
    document_id: String,
    password: String,
) -> AnyhowResult<()> {
    let handle = tokio::task::spawn_blocking({
        move || {
            tokio::runtime::Handle::current().block_on(async move {
                let res = Box::pin(upload_transmission_package_signature_service(
                    &tenant_id,
                    &election_id,
                    &area_id,
                    &tally_session_id,
                    &trustee_name,
                    &document_id,
                    &password,
                ))
                .await;
                match res {
                    Ok(()) => Ok(()),
                    Err(err) => {
                        info!(
                            "Captured backtrace inside spawn_blocking:\n{}",
                            err.backtrace()
                        );
                        Err(err)
                    }
                }
            })
        }
    });

    match handle.await {
        Ok(inner_result) => inner_result.map_err(|err| err.context("Task failed")),
        Err(join_error) => Err(anyhow::Error::from(join_error).context("Task panicked")),
    }?;

    Ok(())
}
