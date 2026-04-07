// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::tasks_execution::{insert_tasks_execution, update_task_execution_status};
use crate::services::serialize_tasks_logs::*;
use crate::types::tasks::ETasksExecution;
use anyhow::{Context, Result};
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use serde::{Deserialize, Serialize};
use tracing::instrument;

/// Observer notified when a tracked task transitions to a terminal state.
///
/// Implementations record task outcomes to an observability backend without
/// coupling the persistence layer to any specific metrics infrastructure.
/// This keeps `tasks_execution` testable with a no-op stand-in and makes
/// the observability strategy swappable at the composition root.
///
/// # Contract
///
/// - `on_completed` is called after the DB record is successfully updated to
///   `SUCCESS`. It must not be called when the DB update itself errors.
/// - `on_failed` is called after the DB record is successfully updated to
///   `FAILED`. Same constraint applies.
/// - Implementations must never propagate panics or errors: an observability
///   failure must never abort the application service call path.
pub trait TaskObserver: Send + Sync {
    /// Called when a task transitions to the `SUCCESS` state.
    fn on_completed(&self, task: &TasksExecution);
    /// Called when a task transitions to the `FAILED` state.
    fn on_failed(&self, task: &TasksExecution);
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskAnnotations {
    document_id: Option<String>,
}

#[instrument(skip_all, err)]
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
#[instrument(skip_all, err)]
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
#[instrument(skip_all, err)]
pub async fn update_complete<O: TaskObserver>(
    task: &TasksExecution,
    document_id: Option<String>,
    observer: &O,
) -> Result<(), anyhow::Error> {
    let task_id = &task.id;
    let new_status = TasksExecutionStatus::SUCCESS;
    let logs = task.logs.clone();
    let new_msg = "Task completed successfully";
    let new_logs = serde_json::to_value(append_general_log(&logs, new_msg))?;

    update(&task.tenant_id, task_id, new_status, new_logs, document_id)
        .await
        .context("Failed to update task execution record")?;

    observer.on_completed(task);
    Ok(())
}

// TODO filter also by tenant-id and document-id
#[instrument(skip_all, err)]
pub async fn update_fail<O: TaskObserver>(
    task: &TasksExecution,
    err_message: &str,
    observer: &O,
) -> Result<(), anyhow::Error> {
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

    observer.on_failed(task);
    Ok(())
}
