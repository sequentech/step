// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::database::get_hasura_pool;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::services::date::ISO8601;
use sequent_core::services::uuid_validation::parse_uuid_v4;
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
        parse_uuid_v4(tenant_id).map_err(|err| anyhow!("Error parsing tenant UUID: {}", err))?;

    let election_event_uuid = if let Some(event_id) = election_event_id {
        if !event_id.is_empty() {
            Some(
                parse_uuid_v4(event_id)
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
        parse_uuid_v4(task_execution_id).context("Failed to parse task_execution_id as UUID")?;

    let tenant_uuid = parse_uuid_v4(tenant_id).context("Failed to parse tenant_id as UUID")?;

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
        parse_uuid_v4(task_id).map_err(|err| anyhow!("Error parsing task UUID: {}", err))?;

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
        parse_uuid_v4(tenant_id).map_err(|err| anyhow!("Error parsing tenant UUID: {}", err))?;
    let election_event_uuid = parse_uuid_v4(election_event_id)
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

/// Outcome of [`claim_document_task_execution`].
#[derive(Debug)]
pub enum DocumentTaskClaim {
    /// The caller owns the task row and must start the task.
    Claimed(TasksExecution),
    /// A recent in-progress or successful attempt with its document exists;
    /// the caller reuses it instead of starting the task again.
    Existing(TasksExecution),
}

/// Claims the task row with the caller-chosen `task_id` for a task that
/// produces the document named in its `document_id` annotation. An existing
/// row is reused while it is IN_PROGRESS and younger than
/// `in_progress_timeout_secs`, or SUCCESS with its document still present;
/// otherwise (failed, cancelled, stale or document gone) it is taken over with
/// the given values. Concurrent claims of one id serialize on the row lock, so
/// only one of them gets `Claimed`. The caller commits the transaction.
#[instrument(skip(hasura_transaction, annotations, logs), err)]
pub async fn claim_document_task_execution(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    task_id: &Uuid,
    name: &str,
    task_type: &str,
    annotations: &Value,
    logs: &Value,
    executed_by_user: &str,
    in_progress_timeout_secs: i32,
) -> Result<DocumentTaskClaim> {
    let tenant_uuid =
        parse_uuid_v4(tenant_id).map_err(|err| anyhow!("Error parsing tenant UUID: {err}"))?;
    let election_event_uuid = parse_uuid_v4(election_event_id)
        .map_err(|err| anyhow!("Error parsing election event UUID: {err}"))?;
    let in_progress = TasksExecutionStatus::IN_PROGRESS.to_string();

    let insert = hasura_transaction
        .prepare(
            r#"
                INSERT INTO sequent_backend.tasks_execution
                    (id, tenant_id, election_event_id, name, type, execution_status,
                     annotations, logs, executed_by_user)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                ON CONFLICT (tenant_id, id) DO NOTHING
                RETURNING *;
            "#,
        )
        .await?;
    let inserted = hasura_transaction
        .query_opt(
            &insert,
            &[
                task_id,
                &tenant_uuid,
                &election_event_uuid,
                &name,
                &task_type,
                &in_progress,
                annotations,
                logs,
                &executed_by_user,
            ],
        )
        .await
        .context("Error inserting document task execution")?;
    if let Some(row) = inserted {
        let TasksExecutionWrapper(task) = row.try_into()?;
        return Ok(DocumentTaskClaim::Claimed(task));
    }

    // The row exists: lock it and decide whether it can be reused.
    let select = hasura_transaction
        .prepare(
            r#"
                SELECT
                    task.*,
                    (
                        (task.execution_status = $3
                            AND task.created_at > now() - make_interval(secs => $4::int))
                        OR (task.execution_status = $5
                            AND EXISTS (
                                SELECT 1
                                FROM sequent_backend.document AS document
                                WHERE document.tenant_id = task.tenant_id
                                    AND document.id
                                        = (task.annotations->>'document_id')::uuid
                            ))
                    ) AS reusable
                FROM sequent_backend.tasks_execution AS task
                WHERE task.tenant_id = $1 AND task.id = $2
                FOR UPDATE OF task;
            "#,
        )
        .await?;
    let row = hasura_transaction
        .query_one(
            &select,
            &[
                &tenant_uuid,
                task_id,
                &in_progress,
                &in_progress_timeout_secs,
                &TasksExecutionStatus::SUCCESS.to_string(),
            ],
        )
        .await
        .context("Error locking document task execution")?;
    let reusable: bool = row.try_get("reusable")?;
    let TasksExecutionWrapper(existing) = row.try_into()?;
    if reusable {
        return Ok(DocumentTaskClaim::Existing(existing));
    }
    let take_over = hasura_transaction
        .prepare(
            r#"
                UPDATE sequent_backend.tasks_execution
                SET
                    execution_status = $3,
                    annotations = $4,
                    logs = $5,
                    executed_by_user = $6,
                    created_at = now(),
                    start_at = now(),
                    end_at = NULL
                WHERE tenant_id = $1 AND id = $2
                RETURNING *;
            "#,
        )
        .await?;
    let row = hasura_transaction
        .query_one(
            &take_over,
            &[
                &tenant_uuid,
                task_id,
                &in_progress,
                annotations,
                logs,
                &executed_by_user,
            ],
        )
        .await
        .context("Error taking over document task execution")?;
    let TasksExecutionWrapper(task) = row.try_into()?;
    Ok(DocumentTaskClaim::Claimed(task))
}

#[cfg(test)]
mod claim_document_task_tests {
    use super::*;
    use crate::services::database::generate_hasura_pool;

    const TABLES: &str = r#"
        CREATE SCHEMA IF NOT EXISTS sequent_backend;
        CREATE TABLE sequent_backend.tasks_execution (
            id UUID NOT NULL DEFAULT gen_random_uuid(),
            tenant_id UUID NOT NULL,
            election_event_id UUID,
            name TEXT NOT NULL,
            type TEXT NOT NULL,
            execution_status TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
            start_at TIMESTAMPTZ NOT NULL DEFAULT now(),
            end_at TIMESTAMPTZ,
            annotations JSONB,
            labels JSONB,
            logs JSON,
            executed_by_user TEXT NOT NULL,
            PRIMARY KEY (tenant_id, id)
        );
        CREATE TABLE sequent_backend.document (
            id UUID NOT NULL,
            tenant_id UUID NOT NULL,
            PRIMARY KEY (id, tenant_id)
        );
    "#;

    struct Fixture {
        tenant_id: String,
        election_event_id: String,
        task_id: Uuid,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                tenant_id: Uuid::new_v4().to_string(),
                election_event_id: Uuid::new_v4().to_string(),
                task_id: Uuid::new_v4(),
            }
        }

        /// Claims the task for `document_id`.
        async fn claim(
            &self,
            transaction: &Transaction<'_>,
            document_id: &str,
        ) -> Result<DocumentTaskClaim> {
            claim_document_task_execution(
                transaction,
                &self.tenant_id,
                &self.election_event_id,
                &self.task_id,
                "Ballot receipt",
                "CREATE_BALLOT_RECEIPT",
                &serde_json::json!({ "document_id": document_id }),
                &serde_json::json!([]),
                "voter",
                600,
            )
            .await
        }

        async fn set(&self, transaction: &Transaction<'_>, assignment: &str) -> Result<()> {
            transaction
                .execute(
                    &format!(
                        "UPDATE sequent_backend.tasks_execution SET {assignment} \
                         WHERE tenant_id = $1::text::uuid AND id = $2"
                    ),
                    &[&self.tenant_id, &self.task_id],
                )
                .await?;
            Ok(())
        }
    }

    fn document_id_of(task: &TasksExecution) -> Option<&str> {
        task.annotations.as_ref()?.get("document_id")?.as_str()
    }

    /// A render in progress or a finished receipt is reused; a new request
    /// does not start another render.
    #[tokio::test]
    #[ignore = "requires an empty PostgreSQL configured through HASURA_DB__*; exercised by the dedicated CI job"]
    async fn reuses_a_task_in_progress_or_with_its_document() -> Result<()> {
        let pool = generate_hasura_pool().await?;
        let mut client = pool.get().await?;
        let transaction = client.transaction().await?;
        transaction.batch_execute(TABLES).await?;
        let fixture = Fixture::new();
        let first_document = Uuid::new_v4().to_string();

        let DocumentTaskClaim::Claimed(task) = fixture.claim(&transaction, &first_document).await?
        else {
            panic!("the first request must claim the task");
        };
        assert_eq!(task.id, fixture.task_id.to_string());
        assert_eq!(task.execution_status, "IN_PROGRESS");

        let DocumentTaskClaim::Existing(task) = fixture
            .claim(&transaction, &Uuid::new_v4().to_string())
            .await?
        else {
            panic!("a render in progress must be reused");
        };
        assert_eq!(document_id_of(&task), Some(first_document.as_str()));

        fixture
            .set(&transaction, "execution_status = 'SUCCESS'")
            .await?;
        transaction
            .execute(
                "INSERT INTO sequent_backend.document (id, tenant_id) VALUES ($1::text::uuid, $2::text::uuid)",
                &[&first_document, &fixture.tenant_id],
            )
            .await?;
        let DocumentTaskClaim::Existing(task) = fixture
            .claim(&transaction, &Uuid::new_v4().to_string())
            .await?
        else {
            panic!("a finished receipt must be reused");
        };
        assert_eq!(document_id_of(&task), Some(first_document.as_str()));

        transaction.rollback().await?;
        Ok(())
    }

    /// A failed, stale or document-less attempt is taken over by the next
    /// request, which renders the receipt again under a new document id.
    #[tokio::test]
    #[ignore = "requires an empty PostgreSQL configured through HASURA_DB__*; exercised by the dedicated CI job"]
    async fn takes_over_a_failed_stale_or_document_less_task() -> Result<()> {
        let pool = generate_hasura_pool().await?;
        let mut client = pool.get().await?;
        let transaction = client.transaction().await?;
        transaction.batch_execute(TABLES).await?;
        let fixture = Fixture::new();
        fixture
            .claim(&transaction, &Uuid::new_v4().to_string())
            .await?;

        for assignment in [
            "execution_status = 'FAILED'",
            "created_at = now() - interval '11 minutes'",
            "execution_status = 'SUCCESS'",
        ] {
            fixture.set(&transaction, assignment).await?;
            let new_document = Uuid::new_v4().to_string();
            let DocumentTaskClaim::Claimed(task) =
                fixture.claim(&transaction, &new_document).await?
            else {
                panic!("{assignment}: the task must be taken over");
            };
            assert_eq!(task.execution_status, "IN_PROGRESS", "{assignment}");
            assert_eq!(
                document_id_of(&task),
                Some(new_document.as_str()),
                "{assignment}"
            );
        }

        transaction.rollback().await?;
        Ok(())
    }
}
