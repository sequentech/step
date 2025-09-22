// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::tasks_execution::get_tasks_by_election_event_id;
use crate::services::database::get_hasura_pool;
use crate::services::documents::upload_and_return_document;
use anyhow::{anyhow, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::util::temp_path::write_into_named_temp_file;
use sequent_core::{services::keycloak::get_event_realm, types::hasura::core::Document};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tracing::{event, info, instrument, Level};

#[instrument(err, skip(transaction))]
pub async fn read_export_data(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<TasksExecution>> {
    let tasks = get_tasks_by_election_event_id(&transaction, tenant_id, election_event_id).await?;

    Ok(tasks)
}

#[instrument(err, skip(transaction))]
pub async fn write_export_document(
    transaction: &Transaction<'_>,
    data: Vec<TasksExecution>,
    election_event_id: &str,
    document_id: &str,
) -> Result<Document> {
    let data_str = serde_json::to_string(&data)?;
    let data_bytes = data_str.into_bytes();

    let name = format!("tasks_execution-{}", election_event_id);

    let (temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&data_bytes, &name, ".json")?;

    // Using the first task to get the tenant_id and election_event_id
    if let Some(first_task) = data.first() {
        upload_and_return_document(
            transaction,
            &temp_path_string,
            file_size,
            "application/json",
            &first_task.tenant_id.to_string(),
            first_task.election_event_id.clone(),
            &name,
            Some(document_id.to_string()),
            false,
        )
        .await
    } else {
        Err(anyhow::anyhow!("No tasks available to write"))
    }
}

#[instrument(err)]
pub async fn process_export(
    tenant_id: &str,
    election_event_id: &str,
    document_id: &str,
) -> Result<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("Error getting hasura db pool: {err}"))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("Error starting hasura transaction: {err}"))?;

    let export_data = read_export_data(&hasura_transaction, tenant_id, election_event_id).await?;
    write_export_document(
        &hasura_transaction,
        export_data,
        &election_event_id,
        document_id,
    )
    .await?;

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed: {}", e));

    Ok(())
}
