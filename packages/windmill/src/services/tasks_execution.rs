// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::tasks_execution::{insert_tasks_execution, update_task_execution_status};
use crate::services::protocol_manager::{create_named_param, get_board_client, get_immudb_client};
use anyhow::{anyhow, Context, Result};
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use serde::{Deserialize, Serialize};
use board_messages::tasks_execution::message::Message;

pub async fn post(
    tenant_id: &str,
    election_event_id: &str,
    task_type: &str,
) -> Result<TasksExecution, anyhow::Error> {
    let message = Message::post_task_message(tenant_id, election_event_id, task_type)?;
    println!("------------------------------ {:?}", message);

    let task = insert_tasks_execution(
        tenant_id,
        election_event_id,
        "Export Election Event", // TODO: delete
        task_type,
        TasksExecutionStatus::IN_PROGRESS,
        None,      // Optional annotations
        None,      // Optional labels
        None,      // Optional logs
        tenant_id, // TODO: Replace with the actual user ID or obtain it dynamically
    )
    .await
    .context("Failed to insert task execution record")?;

    Ok(task)
}

pub async fn update(task_id: &str, status: TasksExecutionStatus) -> Result<(), anyhow::Error> {
    update_task_execution_status(task_id, status)
        .await
        .context("Failed to update task execution record")?;
    Ok(())
}
