use crate::postgres::ballot_style::get_ballot_styles_by_ballot_publication_by_id;
// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::ballot_publication::get_ballot_publication_by_id;
use crate::services::database::get_hasura_pool;
use crate::services::export::export_ballot_publication::process_export_json_to_csv;
use crate::services::export::export_election_event::process_export_zip;
use crate::services::tasks_execution::*;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::serialization::deserialize_with_path::*;
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

    let ballot_styles = match get_ballot_styles_by_ballot_publication_by_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication_id,
    )
    .await
    {
        Ok(ballot_styles) => ballot_styles,
        Err(err) => {
            update_fail(
                &task_execution,
                &format!("Error obtaining ballot styles: {err:?}"),
            )
            .await?;
            return Err(Error::String(format!(
                "Error obtaining ballot styles: {err:?}"
            )));
        }
    };

    let ballot_emls = match ballot_styles
        .into_iter()
        .filter_map(|val| val.ballot_eml.as_ref().map(|eml| Ok(deserialize_str(eml)?)))
        .collect::<Result<Vec<Value>>>()
    {
        Ok(ballot_emls) => ballot_emls,
        Err(err) => {
            update_fail(
                &task_execution,
                &format!("Error deserializing ballot eml: {err:?}"),
            )
            .await?;
            return Err(Error::String(format!(
                "Error deserializing ballot eml: {err:?}"
            )));
        }
    };

    let ballot_design = json!({
        "ballot_publication_id": &ballot_publication_id,
        "ballot_styles": ballot_emls,
    })
    .to_string();

    // Process the export
    match process_export_json_to_csv(&tenant_id, &election_event_id, &document_id, &ballot_design)
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
            update_fail(&task_execution, "Failed to insert task execution record").await?;
            return Err(Error::String(format!("Commit failed: {}", err)));
        }
    };

    update_complete(&task_execution)
        .await
        .context("Failed to update task execution status to COMPLETED")?;

    Ok(())
}
