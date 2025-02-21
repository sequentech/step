// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::tasks_execution::{insert_tasks_execution, update_task_execution_status};
use crate::services::serialize_tasks_logs::*;
use crate::types::tasks::ETasksExecution;
use anyhow::{Context, Result};
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskAnnotations {
    document_id: Option<String>,
}

pub async fn post(
    tenant_id: &str,
    election_event_id: Option<&str>,
    task_type: ETasksExecution,
    executed_by_user: &str,
) -> Result<TasksExecution, anyhow::Error> {
    let logs = serde_json::to_value(general_start_log())?;

    let task = insert_tasks_execution(
        tenant_id,
        election_event_id,
        &task_type.to_name(),
        &task_type.to_string(),
        TasksExecutionStatus::IN_PROGRESS,
        None,
        None,
        Some(logs),
        executed_by_user,
    )
    .await
    .context("Failed to insert task execution record")?;

    Ok(task)
}

// TODO filter also by tenant-id and document-id
pub async fn update(
    tenant_id: &str,
    task_id: &str,
    status: TasksExecutionStatus,
    logs: serde_json::Value,
    document_id: Option<String>,
) -> Result<(), anyhow::Error> {
    let annotations = serde_json::to_value(TaskAnnotations { document_id })?;
    update_task_execution_status(tenant_id, task_id, status, Some(logs), annotations)
        .await
        .context("Failed to update task execution record")?;
    Ok(())
}

// TODO filter also by tenant-id and document-id
pub async fn update_complete(
    task: &TasksExecution,
    document_id: Option<String>,
) -> Result<(), anyhow::Error> {
    let task_id = &task.id;
    let new_status = TasksExecutionStatus::SUCCESS;
    let logs = task.logs.clone();
    let new_msg = "Task completed successfully";
    let new_logs = serde_json::to_value(append_general_log(&logs, new_msg))?;

    update(&task.tenant_id, &task_id, new_status, new_logs, document_id)
        .await
        .context("Failed to update task execution record")?;
    Ok(())
}

// TODO filter also by tenant-id and document-id
pub async fn update_fail(task: &TasksExecution, err_message: &str) -> Result<(), anyhow::Error> {
    let task_id = &task.id;
    let new_status = TasksExecutionStatus::FAILED;
    let logs = task.logs.clone();
    let new_logs = serde_json::to_value(append_general_log(
        &logs,
        &("Error: ".to_owned() + err_message),
    ))?;
    let annotations = serde_json::to_value(TaskAnnotations { document_id: None })?;

    update_task_execution_status(
        &task.tenant_id,
        task_id,
        new_status,
        Some(new_logs),
        annotations,
    )
    .await
    .context("Failed to update task execution record with failure status")?;

    Ok(())
}
