use crate::postgres::document;
// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::plugins_manager::plugin_manager;
use crate::services::tasks_execution::*;
use crate::types::error::Error;
use crate::types::error::Result;
use anyhow::Result as AnyhowResult;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use sequent_core::types::hasura::core::TasksExecution;
use serde_json::Value;
use tracing::{info, instrument};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn execute_plugin_task(
    task: String,
    data: Value,
    task_execution: TasksExecution,
    document_id: Option<String>,
) -> Result<()> {
    let task_execution_clone = task_execution.clone();

    let mut task_data = data;

    let task_execution_str: String = serde_json::to_string(&task_execution)
        .expect("Failed to serialize task_execution to string");

    task_data["task_execution"] = serde_json::Value::String(task_execution_str);

    if let Some(doc_id) = document_id {
        task_data["document_id"] = serde_json::Value::String(doc_id);
    }

    let plugin_manager: &'static plugin_manager::PluginManager =
        plugin_manager::get_plugin_manager()
            .await
            .context("Failed to get plugin manager")
            .map_err(Error::from)?;

    let res = tokio::spawn(async move {
        let execution_result = plugin_manager
            .execute_task(&task, task_data.to_string())
            .await;

        match execution_result {
            AnyhowResult::Ok(_) => {
                if let Err(e) = update_complete(&task_execution_clone, None).await {
                    info!("Failed to update task as complete: {}", e);
                }
            }
            AnyhowResult::Err(e) => {
                info!(
                    "Captured error backtrace from background execute_task for task '{}':\n{:?}",
                    task,
                    e.backtrace()
                );
                if let Err(update_err) = update_fail(&task_execution_clone, &e.to_string()).await {
                    info!("Failed to update task as failed: {}", update_err);
                }
            }
        }
    });

    match res.await {
        Ok(_) => Ok(()),
        Err(join_error) => Err(Error::from(anyhow!("Task panicked: {}", join_error))),
    }
}
