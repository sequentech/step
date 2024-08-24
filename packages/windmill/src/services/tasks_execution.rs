// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::tasks_execution::{insert_tasks_execution, update_task_execution_status};
use crate::services::serialize_tasks_logs::*;
use crate::types::tasks::ETasks;
use anyhow::{anyhow, Context, Result};
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use sequent_core::types::{ceremonies::Log, hasura::core::TasksExecution};
use serde::{Deserialize, Serialize};

pub async fn post(
    tenant_id: &str,
    election_event_id: &str,
    task_type: ETasks,
    executed_by_user_id: &str,
) -> Result<TasksExecution, anyhow::Error> {
    let logs = serde_json::to_value(general_start_log())?;

    let task = insert_tasks_execution(
        tenant_id,
        election_event_id,
        "Export Election Event", // TODO: delete
        &task_type.to_string(),
        TasksExecutionStatus::IN_PROGRESS,
        None,
        None,
        Some(logs),
        executed_by_user_id,
    )
    .await
    .context("Failed to insert task execution record")?;

    Ok(task)
}

pub async fn update(
    task_id: &str,
    status: TasksExecutionStatus,
    logs: serde_json::Value,
) -> Result<(), anyhow::Error> {
    update_task_execution_status(task_id, status, Some(logs))
        .await
        .context("Failed to update task execution record")?;
    Ok(())
}

pub async fn update_complete(task: &TasksExecution) -> Result<(), anyhow::Error> {
    let task_id = &task.id;
    let new_status = TasksExecutionStatus::SUCCESS;
    let logs = task.logs.clone();
    let new_msg = "Task completed successfully";
    let new_logs = serde_json::to_value(append_general_log(&logs, new_msg))?;

    update(&task_id, new_status, new_logs)
        .await
        .context("Failed to update task execution record")?;
    Ok(())
}

pub async fn update_fail(task: &TasksExecution, err_message: &str) -> Result<(), anyhow::Error> {
    let task_id = &task.id;
    let new_status = TasksExecutionStatus::FAILED;
    let logs = task.logs.clone();
    let new_logs = serde_json::to_value(append_general_log(&logs, err_message))?;

    update_task_execution_status(task_id, new_status, Some(new_logs))
        .await
        .context("Failed to update task execution record with failure status")?;

    Ok(())
}
