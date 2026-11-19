// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::export::export_trustees::read_trustees_config;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::services::task_secret;
use crate::services::tasks_execution::update_fail;
use crate::types::error::Result as TaskResult;
use anyhow::Result;
use celery::error::TaskError;
use sequent_core::types::hasura::core::TasksExecution;
use tracing::instrument;

#[instrument(skip(password), err)]
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

/// `password_secret` refers to the export password, stored by the sender with
/// `task_secret::stash`; the message never carries the password itself.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_trustees_task(
    tenant_id: String,
    document_id: String,
    password_secret: String,
    task_execution: TasksExecution,
) -> TaskResult<()> {
    let password = match task_secret::redeem(&tenant_id, &password_secret).await {
        Ok(password) => password,
        Err(error) => {
            let _ = update_fail(&task_execution, &format!("{error:#}")).await;
            return Err(error.into());
        }
    };
    export_trustees_service(tenant_id, document_id, password, task_execution).await?;
    Ok(())
}
