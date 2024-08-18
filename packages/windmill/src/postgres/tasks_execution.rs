// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::{hasura::core::TasksExecution, hasura::extra::TasksExecutionStatus};
use serde_json::value::Value;
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
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            name: item
                .try_get::<_, Option<Uuid>>("name")?
                .map(|val| val.to_string()),
            task_type: item
                .try_get::<_, Option<Uuid>>("type")?
                .map(|val| val.to_string()),
            execution_status: item
                .try_get::<_, Option<Uuid>>("execution_status")?
                .map(|val| val.to_string()),
            created_at: item.get("created_at"),
            start_at: item.get("start_at"),
            end_at: item.get("end_at"),
            annotations: item.try_get("annotations")?,
            labels: item.try_get("labels")?,
            logs: item.try_get("logs")?,
            executed_by_user_id: item
                .try_get::<_, Option<Uuid>>("executed_by_user_id")?
                .map(|val| val.to_string()),
        }))
    }
}

#[instrument(skip(transaction), err)]
pub async fn insert_tasks_execution(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    name: &str,
    task_type: &str,
    execution_status: TasksExecutionStatus,
    annotations: Option<Value>,
    labels: Option<Value>,
    logs: Option<Value>,
    executed_by_user_id: &str,
) -> Result<()> {
    let tenant_uuid =
        Uuid::parse_str(tenant_id).map_err(|err| anyhow!("Error parsing tenant UUID: {}", err))?;

    let election_event_uuid = Uuid::parse_str(election_event_id)
        .map_err(|err| anyhow!("Error parsing election event UUID: {}", err))?;

    let executed_by_user_uuid = Uuid::parse_str(executed_by_user_id)
        .map_err(|err| anyhow!("Error parsing executed by user UUID: {}", err))?;

    let statement = transaction
        .prepare(
            r#"
                INSERT INTO
                    sequent_backend.tasks_execution
                (tenant_id, election_event_id, name, type, execution_status, annotations, labels, logs, executed_by_user_id)
                VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9
                )
                RETURNING
                    *;
            "#,
        )
        .await?;

    // Execute the query and expect a single row
    let row = transaction
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
                &executed_by_user_uuid,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error inserting task execution: {}", err))?;

    // Convert the resulting row into `TasksExecution`
    // let task_execution: TasksExecution = row
    //     .try_into()
    //     .map(|wrapper: TasksExecutionWrapper| wrapper.0)
    //     .context("Error converting database row to TasksExecution")?;

    Ok(())
}
