// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::database::get_hasura_pool;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::services::date::ISO8601;
use sequent_core::types::{
    ceremonies::Log,
    hasura::{core::TasksExecution, extra::TasksExecutionStatus},
};
use serde_json::value::Value;
use std::collections::HashMap;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
use uuid::Uuid;

pub struct TasksExecutionWrapper(pub TasksExecution);

// implements a conversion from a database row to that TasksExecutionWrapper structure
impl TryFrom<Row> for TasksExecutionWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        Ok(TasksExecutionWrapper(TasksExecution {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item
                .try_get::<_, Option<Uuid>>("election_event_id")?
                .map(|uuid| uuid.to_string()),
            name: item.try_get::<_, String>("name")?.to_string(),
            task_type: item.try_get::<_, String>("type")?.to_string(),
            execution_status: item.try_get::<_, String>("execution_status")?.to_string(),
            created_at: item.get("created_at"),
            start_at: item.get("start_at"),
            end_at: item.get("end_at"),
            annotations: item.try_get("annotations")?,
            labels: item.try_get("labels")?,
            logs: item.try_get("logs")?,
            executed_by_user: item.try_get::<_, String>("executed_by_user")?.to_string(),
        }))
    }
}

#[instrument(skip(annotations, labels, logs), err)]
pub async fn insert_tasks_execution(
    tenant_id: &str,
    election_event_id: Option<&str>,
    name: &str,
    task_type: &str,
    execution_status: TasksExecutionStatus,
    annotations: Option<Value>,
    labels: Option<Value>,
    logs: Option<Value>,
    executed_by_user: &str,
) -> Result<TasksExecution> {
    let db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("Error getting hasura db pool: {err}"))?;

    let tenant_uuid =
        Uuid::parse_str(tenant_id).map_err(|err| anyhow!("Error parsing tenant UUID: {}", err))?;

    let election_event_uuid = if let Some(event_id) = election_event_id {
        if !event_id.is_empty() {
            Some(
                Uuid::parse_str(event_id)
                    .map_err(|err| anyhow!("Error parsing election event UUID: {}", err))?,
            )
        } else {
            None
        }
    } else {
        None
    };

    let statement = db_client
        .prepare(
            r#"
                INSERT INTO
                    sequent_backend.tasks_execution
                (tenant_id, election_event_id, name, type, execution_status, annotations, labels, logs, executed_by_user)
                VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9
                )
                RETURNING
                    *;
            "#,
        )
        .await?;

    let row = db_client
        .query_one(
            &statement,
            &[
                &tenant_uuid,
                &election_event_uuid,
                &name,
                &task_type,
                &execution_status.to_string(),
                &annotations,
                &labels,
                &logs,
                &executed_by_user,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error inserting task execution: {}", err))?;

    // Convert the resulting row into `TasksExecution` struct
    let task_execution: TasksExecution = row
        .try_into()
        .map(|wrapper: TasksExecutionWrapper| wrapper.0)
        .context("Error converting database row to TasksExecution")?;

    Ok(task_execution)
}

pub async fn update_task_execution_status(
    tenant_id: &str,
    task_execution_id: &str,
    new_status: TasksExecutionStatus,
    new_logs: Option<Value>,
    annotations: Value,
) -> Result<()> {
    let db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .context("Failed to get database client from pool")?;

    let task_execution_uuid =
        Uuid::parse_str(task_execution_id).context("Failed to parse task_execution_id as UUID")?;

    let tenant_uuid = Uuid::parse_str(tenant_id).context("Failed to parse tenant_id as UUID")?;

    let statement = db_client
        .prepare(
            r#"
            UPDATE sequent_backend.tasks_execution
            SET 
                execution_status = $1,
                logs = $2,
                end_at = CASE
                    WHEN $1 != 'IN_PROGRESS' THEN now()
                    ELSE end_at
                END,
                    annotations = COALESCE(annotations, '{}'::jsonb) || $3::jsonb
            WHERE
                id = $4 AND
                tenant_id = $5;
            "#,
        )
        .await
        .context("Failed to prepare SQL statement")?;

    // Execute the update statement
    db_client
        .execute(
            &statement,
            &[
                &new_status.to_string(),
                &new_logs,
                &annotations,
                &task_execution_uuid,
                &tenant_uuid,
            ],
        )
        .await
        .context("Failed to execute update task execution status query")?;

    Ok(())
}

#[instrument(skip(), err)]
pub async fn get_task_by_id(task_id: &str) -> Result<TasksExecution> {
    let db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("Error getting hasura db pool: {err}"))?;

    let task_uuid =
        Uuid::parse_str(task_id).map_err(|err| anyhow!("Error parsing task UUID: {}", err))?;

    let statement = db_client
        .prepare(
            r#"
                SELECT
                    *
                FROM 
                    sequent_backend.tasks_execution
                WHERE
                    id = $1
            "#,
        )
        .await?;

    let row = db_client
        .query_one(&statement, &[&task_uuid])
        .await
        .map_err(|err| anyhow!("Error fetching task: {}", err))?;

    // Convert the resulting row into `TasksExecution` struct
    let task_execution: TasksExecution = row
        .try_into()
        .map(|wrapper: TasksExecutionWrapper| wrapper.0)
        .context("Error converting database row to TasksExecution")?;

    Ok(task_execution)
}

#[instrument(skip(), err)]
pub async fn get_tasks_by_election_event_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<TasksExecution>> {
    let tenant_uuid =
        Uuid::parse_str(tenant_id).map_err(|err| anyhow!("Error parsing tenant UUID: {}", err))?;
    let election_event_uuid = Uuid::parse_str(election_event_id)
        .map_err(|err| anyhow!("Error parsing task UUID: {}", err))?;

    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM 
                sequent_backend.tasks_execution
            WHERE
                tenant_id = $1
                AND election_event_id = $2
        "#,
        )
        .await?;

    let rows = hasura_transaction
        .query(&statement, &[&tenant_uuid, &election_event_uuid])
        .await
        .map_err(|err| anyhow!("Error fetching tasks: {}", err))?;

    // Convert the resulting row into `TasksExecution` struct
    let tasks_execution: Vec<TasksExecution> = rows
        .into_iter()
        .map(|row| {
            row.try_into()
                .map(|wrapper: TasksExecutionWrapper| wrapper.0)
        })
        .collect::<Result<Vec<_>, _>>()
        .context("Error converting database rows to TasksExecution")?;

    Ok(tasks_execution)
}
