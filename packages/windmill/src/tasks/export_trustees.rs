// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::database::get_hasura_pool;
use crate::services::export::export_election_event::process_export_zip;
use crate::services::tasks_execution::*;
use crate::types::error::{Error, Result as TaskResult};
use anyhow::{anyhow, Context, Result};
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};

#[instrument(err)]
async fn export_trustees_service(
    tenant_id: String,
    document_id: String,
    task_execution: TasksExecution,
) -> Result<()> {

}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_trustees_task(
    tenant_id: String,
    document_id: String,
    task_execution: TasksExecution,
) -> TaskResult<()> {
    export_trustees_service(
        tenant_id,
        document_id,
        task_execution,
    ).await
}