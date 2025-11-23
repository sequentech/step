// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::ballot_publication::get_ballot_publication_by_id;
use crate::services::database::get_hasura_pool;
use crate::services::export::export_ballot_publication::process_export_ballot_publication;
use crate::services::tasks_execution::*;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{event, instrument, Level};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_ballot_publication(
    tenant_id: String,
    election_event_id: String,
    document_id: String,
    ballot_publication_id: String,
    task_execution: TasksExecution,
) -> Result<()> {
    let mut hasura_db_client: DbClient = match get_hasura_pool().await.get().await {
        Ok(client) => client,
        Err(err) => {
            let err_str = format!("Error getting Hasura DB pool: {err:?}");
            update_fail(&task_execution, &err_str).await;
            return Err(Error::String(err_str));
        }
    };

    let hasura_transaction = match hasura_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            let err_str = format!("Failed to start Hasura transaction: {err:?}");
            update_fail(&task_execution, &err_str).await;
            return Err(Error::String(err_str));
        }
    };

    let ballot_publication = match get_ballot_publication_by_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication_id,
    )
    .await
    {
        Ok(Some(ballot_publication)) => ballot_publication,
        Ok(None) => {
            update_fail(
                &task_execution,
                &format!("Ballot Publication not found by id={ballot_publication_id:?}"),
            )
            .await?;
            return Err(Error::String(format!(
                "Ballot Publication not found by id={ballot_publication_id:?}"
            )));
        }
        Err(err) => {
            update_fail(
                &task_execution,
                &format!("Error obtaining ballot by id: {err:?}"),
            )
            .await?;
            return Err(Error::String(format!(
                "Error obtaining ballot by id: {err:?}"
            )));
        }
    };

    // Process the export
    match process_export_ballot_publication(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &document_id,
        &vec![ballot_publication],
        true,
    )
    .await
    {
        Ok(_) => (),
        Err(err) => {
            update_fail(&task_execution, &err.to_string()).await?;
            return Err(Error::String(format!(
                "Failed to export ballot publication data: {err:?}"
            )));
        }
    }

    match hasura_transaction.commit().await {
        Ok(_) => (),
        Err(err) => {
            let err_str = format!("Commit failed: {err:?}");
            update_fail(&task_execution, &err_str).await;
            return Err(Error::String(err_str));
        }
    };

    update_complete(&task_execution, Some(document_id.to_string()))
        .await
        .context("Failed to update task execution status to COMPLETED")?;

    Ok(())
}
