// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::tasks_execution::*;
use crate::services::{database::get_hasura_pool, export_election_event::process_export};
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::hasura::core::TasksExecution;
use tracing::{event, instrument, Level};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_election_event(
    tenant_id: String,
    election_event_id: String,
    document_id: String,
    task_execution: TasksExecution,
) -> Result<()> {
    let mut hasura_db_client: DbClient = match get_hasura_pool().await.get().await {
        Ok(client) => client,
        Err(err) => {
            update_fail(&task_execution, "Failed to get Hasura DB pool").await;
            return Err(Error::String(format!(
                "Error getting Hasura DB pool: {}",
                err
            )));
        }
    };

    // Start a new database transaction
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("Error starting hasura transaction: {err}"))?;

    // Process the export
    match process_export(&tenant_id, &election_event_id, &document_id).await {
        Ok(_) => (),
        Err(err) => {
            update_fail(&task_execution, "Failed to export election event data").await?;
            return Err(Error::String(format!(
                "Failed to export election event data: {}",
                err
            )));
        }
    }

    match hasura_transaction.commit().await {
        Ok(_) => (),
        Err(err) => {
            update_fail(&task_execution, "Failed to insert task execution record").await?;
            return Err(Error::String(format!("Commit failed: {}", err)));
        }
    };

    update_complete(&task_execution)
        .await
        .context("Failed to update task execution status to COMPLETED")?;

    Ok(())
}
