// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::document::insert_document;
use crate::services::database::{get_hasura_pool, get_keycloak_pool, PgConfig};
use crate::services::documents::upload_and_return_document;
use crate::services::export::export_users::{export_users_file, ExportBody};
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction as _};
use sequent_core::services::keycloak;
use sequent_core::services::s3;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::util;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExportUsersOutput {
    pub document_id: String,
    pub error_msg: Option<String>,
    pub task_execution: Option<TasksExecution>,
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_users(
    body: ExportBody,
    document_id: String,
    task_execution: Option<TasksExecution>,
) -> Result<()> {
    let mut hasura_db_client: DbClient = match get_hasura_pool().await.get().await {
        Ok(client) => client,
        Err(err) => {
            if let Some(task_execution) = &task_execution {
                update_fail(task_execution, "Failed to get Hasura DB pool").await;
            }
            return Err(Error::String(format!(
                "Error getting Hasura DB pool: {}",
                err
            )));
        }
    };

    let hasura_transaction = match hasura_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            if let Some(task_execution) = &task_execution {
                update_fail(&task_execution, "Failed to start Hasura transaction").await?;
            }
            return Err(Error::String(format!(
                "Error starting Hasura transaction: {err}"
            )));
        }
    };

    // Export the users to a temporary file
    let temp_path = match export_users_file(&hasura_transaction, body.clone()).await {
        Ok(result) => result,
        Err(err) => {
            if let Some(task_execution) = &task_execution {
                update_fail(&task_execution, &err.to_string()).await?;
            }
            return Err(Error::String(format!("Error listing users: {err:?}")));
        }
    };
    let size = temp_path.metadata()?.len();

    // Upload to S3
    let (tenant_id, election_event_id) = match &body {
        ExportBody::Users {
            tenant_id,
            election_event_id,
            ..
        } => (
            tenant_id.to_string(),
            election_event_id.clone().unwrap_or_default(),
        ),
        ExportBody::TenantUsers { tenant_id } => (tenant_id.to_string(), "".to_string()),
    };

    let timestamp = match util::date::timestamp() {
        Ok(timestamp) => timestamp,
        Err(err) => {
            if let Some(task_execution) = &task_execution {
                update_fail(&task_execution, "Failed to obtain timestamp").await?;
            }
            return Err(Error::String(format!("Error obtaining timestamp: {err}")));
        }
    };

    let name = format!("users-export-{timestamp}.csv");
    let key = s3::get_document_key(&tenant_id, Some(&election_event_id), &document_id, &name);

    let media_type = "text/csv".to_string();

    match s3::upload_file_to_s3(
        key,
        false,
        s3::get_private_bucket()?,
        media_type.clone(),
        temp_path.to_string_lossy().to_string(),
        None,
        Some(name.clone()),
    )
    .await
    {
        Ok(_) => (),
        Err(err) => {
            if let Some(task_execution) = &task_execution {
                update_fail(&task_execution, "Failed to upload file to s3").await?;
            }
            return Err(Error::String(format!("Error uploading file to s3: {err}")));
        }
    }

    temp_path
        .close()
        .with_context(|| "Error closing temporary file path")?;

    let _document = insert_document(
        &hasura_transaction,
        &tenant_id,
        match &body {
            ExportBody::Users {
                election_event_id, ..
            } => election_event_id.clone(),
            ExportBody::TenantUsers { .. } => None,
        },
        &name,
        &media_type,
        size.try_into()?,
        false,
        Some(document_id.clone()),
    )
    .await
    .map_err(|err| format!("Error inserting document: {:?}", err))?;

    if let Some(task_execution) = &task_execution {
        update_complete(&task_execution, Some(document_id.to_string()))
            .await
            .context("Failed to update task execution status to COMPLETED")?;
    }

    hasura_transaction
        .commit()
        .await
        .with_context(|| "Failed to commit Hasura transaction")?;

    Ok(())
}
