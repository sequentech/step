// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::{
    hasura::extra::{TasksExecutionStatus},
    hasura::core::{TasksExecution},
};
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


#[instrument(skip(hasura_transaction), err)]
pub async fn insert_tasks_execution(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    name: &str,
    task_type: &str,
    execution_status: TasksExecutionStatus,
    annotations: Option<Value>,
    labels: Option<Value>,
    logs: Option<Value>,
    executed_by_user_id: &str,
) -> Result<TasksExecution> {
    let statement = hasura_transaction
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

    // Execute the query
    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &name,
                &task_type,
                &execution_status.to_string(),
                &annotations,
                &labels,
                &logs,
                &Uuid::parse_str(executed_by_user_id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error inserting task execution: {}", err))?;

    // Convert the resulting row(s) into `TasksExecution`
    let values: Vec<TasksExecution> = rows
        .into_iter()
        .map(|row| -> Result<TasksExecution> {
            row.try_into()
                .map(|res: TasksExecutionWrapper| res.0)
        })
        .collect::<Result<Vec<TasksExecution>>>()?;

    // Return the first value or an error if no rows were inserted
    let Some(value) = values.first() else {
        return Err(anyhow!("Error inserting row: no rows returned"));
    };
    Ok(value.clone())
}
