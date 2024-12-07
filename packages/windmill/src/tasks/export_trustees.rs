// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::export::export_trustees::read_trustees_config;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
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
    password: String,
    task_execution: TasksExecution,
) -> Result<()> {
    provide_hasura_transaction(|hasura_transaction| {
        let tenant_id = tenant_id.clone();
        let document_id = document_id.clone();
        let password = password.clone();
        let task_execution = task_execution.clone();
        Box::pin(async move {
            read_trustees_config(
                hasura_transaction,
                &tenant_id,
                &document_id,
                &password,
                &task_execution,
            )
            .await
        })
    })
    .await
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_trustees_task(
    tenant_id: String,
    document_id: String,
    password: String,
    task_execution: TasksExecution,
) -> TaskResult<()> {
    export_trustees_service(tenant_id, document_id, password, task_execution).await?;
    Ok(())
}
