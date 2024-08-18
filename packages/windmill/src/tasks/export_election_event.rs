// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::tasks_execution::insert_tasks_execution;
use crate::services::{database::get_hasura_pool, export_election_event::process_export};
use crate::types::error::Result;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use tracing::{event, instrument, Level};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_election_event(
    tenant_id: String,
    election_event_id: String,
    document_id: String,
) -> Result<()> {
    // Get the database connection from the pool
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("Error getting hasura db pool: {err}"))?;

    // Start a new database transaction
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("Error starting hasura transaction: {err}"))?;

    // Process the export, returning an error if it fails
    process_export(&tenant_id, &election_event_id, &document_id)
        .await
        .context("Failed to export election event data")?;

    // Insert the task execution record
    insert_tasks_execution(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        "Export Election Event",
        "ExportElectionEvent",
        TasksExecutionStatus::IN_PROGRESS,
        None,
        None,
        None,
        &tenant_id, //TODO: Replace with the actual user ID or obtain it dynamically
    )
    .await
    .context("Failed to insert task execution record")?;

    hasura_transaction
        .commit()
        .await
        .context("Failed to insert task execution record")?;

    Ok(())
}
