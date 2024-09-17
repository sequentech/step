// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::tasks_execution::*;
use crate::types::error::{Error, Result};
use crate::services::database::get_hasura_pool;
use crate::services::export_election_event::process_export_zip;
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

    let hasura_transaction = match hasura_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            update_fail(&task_execution, "Failed to start Hasura transaction").await?;
            return Err(Error::String(format!(
                "Error starting Hasura transaction: {err}"
            )));
        }
    };

    // Process the export
    match process_export_zip(&tenant_id, &election_event_id, &document_id).await {
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
