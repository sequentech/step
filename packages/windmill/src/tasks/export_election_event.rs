// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::tasks_execution::*;
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
    // Insert the task execution record
    let task = post(&tenant_id, &election_event_id, "ExportElectionEvent") //TODO: fix type
        .await
        .context("Failed to insert task execution record")?;

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

    // Process the export
    process_export(&tenant_id, &election_event_id, &document_id)
        .await
        .context("Failed to export election event data")?;

    hasura_transaction
        .commit()
        .await
        .context("Failed to insert task execution record")?;

    update_complete(&task)
        .await
        .context("Failed to update task execution status to COMPLETED")?;
    Ok(())
}
