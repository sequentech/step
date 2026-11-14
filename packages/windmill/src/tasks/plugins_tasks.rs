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
use sequent_core::types::plugins::{PLUGIN_DOCUMENT_ID_KEY, PLUGIN_TASK_EXECUTION_KEY};
use serde_json::{Map, Value};
use tracing::{info, instrument};

fn task_data_object(data: Value) -> AnyhowResult<Map<String, Value>> {
    match data {
        Value::Object(map) => Ok(map),
        Value::Null => Ok(Map::new()),
        _ => Err(anyhow!("Plugin task data must be a JSON object")),
    }
}

async fn mark_failed(task_execution: &TasksExecution, err: anyhow::Error) -> Error {
    if let Err(update_err) = update_fail(task_execution, &err.to_string()).await {
        info!("Failed to update task as failed: {}", update_err);
    }
    Error::from(err)
}

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

    let mut task_data = match task_data_object(data) {
        Ok(task_data) => task_data,
        Err(err) => return Err(mark_failed(&task_execution, err).await),
    };

    let task_execution_str: String = match serde_json::to_string(&task_execution)
        .context("Failed to serialize task_execution to string")
    {
        Ok(task_execution_str) => task_execution_str,
        Err(err) => return Err(mark_failed(&task_execution, err).await),
    };

    task_data.insert(
        PLUGIN_TASK_EXECUTION_KEY.to_string(),
        Value::String(task_execution_str),
    );

    if let Some(doc_id) = document_id {
        task_data.insert(PLUGIN_DOCUMENT_ID_KEY.to_string(), Value::String(doc_id));
    }
    let task_data = Value::Object(task_data);

    let plugin_manager: &'static plugin_manager::PluginManager =
        match plugin_manager::get_plugin_manager()
            .await
            .context("Failed to get plugin manager")
        {
            Ok(plugin_manager) => plugin_manager,
            Err(err) => return Err(mark_failed(&task_execution, err).await),
        };

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn task_data_rejects_non_object_values() {
        for data in [json!([1, 2, 3]), json!("data"), json!(42), json!(true)] {
            assert!(task_data_object(data).is_err());
        }
    }

    #[test]
    fn task_data_accepts_objects() {
        let task_data = task_data_object(json!({"election_event_id": "e1"})).unwrap();
        assert_eq!(task_data.get("election_event_id"), Some(&json!("e1")));
        assert_eq!(task_data.len(), 1);
    }

    #[test]
    fn task_data_treats_null_as_empty_object() {
        assert!(task_data_object(Value::Null).unwrap().is_empty());
    }
}
