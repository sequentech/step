// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::database::get_hasura_pool;
use crate::services::export::export_tenant_config::process_export_zip;
use crate::services::tasks_execution::*;
use crate::types::error::{Error, Result};
use anyhow::Context;
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_tenant_config(
    tenant_id: String,
    document_id: String,
    task_execution: TasksExecution,
) -> Result<()> {
    let mut hasura_db_client: DbClient = match get_hasura_pool().await.get().await {
        Ok(client) => client,
        Err(err) => {
            let err_str = format!("Failed to get Hasura DB pool: {err:?}");
            if let Err(err) = update_fail(&task_execution, &err_str).await {
                event!(
                    Level::ERROR,
                    "Failed to update task execution status to FAILED: {:?}",
                    err
                );
            }
            return Err(Error::String(err_str));
        }
    };

    let hasura_transaction = match hasura_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            let err_str = format!("Failed to start Hasura transaction: {err:?}");
            if let Err(err) = update_fail(&task_execution, &err_str).await {
                event!(
                    Level::ERROR,
                    "Failed to update task execution status to FAILED: {:?}",
                    err
                );
            }
            return Err(Error::String(err_str));
        }
    };

    // Process the export
    match process_export_zip(&tenant_id, &document_id, &hasura_transaction).await {
        Ok(_) => (),
        Err(err) => {
            let err_str = format!("Failed to export tenant config zip: {err:?}");
            if let Err(update_err) = update_fail(&task_execution, &err_str).await {
                event!(
                    Level::ERROR,
                    "Failed to update task execution status to FAILED: {:?}",
                    update_err
                );
            }
            return Err(Error::String(err_str));
        }
    }

    match hasura_transaction.commit().await {
        Ok(_) => (),
        Err(err) => {
            let err_str = format!("Commit failed: {err:?}");
            if let Err(err) = update_fail(&task_execution, &err_str).await {
                event!(
                    Level::ERROR,
                    "Failed to update task execution status to FAILED: {:?}",
                    err
                );
            }
            return Err(Error::String(err_str));
        }
    };

    update_complete(&task_execution, Some(document_id.to_string()))
        .await
        .context("Failed to update task execution status to COMPLETED")?;

    Ok(())
}
