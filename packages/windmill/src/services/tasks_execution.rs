// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::tasks_execution::{
    insert_tasks_execution, update_export_task_execution_status_if_in_progress,
    update_task_execution_status,
};
use crate::services::serialize_tasks_logs::*;
use crate::types::tasks::ETasksExecution;
use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskAnnotations {
    document_id: Option<String>,
}

const SECRET_EXPORT_AUTHORIZATION_KEY: &str = "secret_export_authorization";
/// How long a persisted decrypted-export grant stays valid. Read by harvest
/// when it creates the grant and by windmill when it starts the export.
pub const SECRET_EXPORT_GRANT_TTL_ENV_VAR: &str = "WINDMILL_SECRET_EXPORT_GRANT_TTL_SECONDS";
const DEFAULT_SECRET_EXPORT_GRANT_TTL_SECONDS: i64 = 24 * 60 * 60;

fn secret_export_grant_ttl() -> Duration {
    let seconds = std::env::var(SECRET_EXPORT_GRANT_TTL_ENV_VAR)
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|seconds| *seconds > 0)
        .unwrap_or(DEFAULT_SECRET_EXPORT_GRANT_TTL_SECONDS);
    Duration::seconds(seconds)
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SecretExportAuthorization {
    document_id: String,
    voter_secret_attributes: bool,
    expires_at: chrono::DateTime<Utc>,
}

pub fn secret_export_task_annotations(
    document_id: &str,
    voter_secret_attributes: bool,
) -> serde_json::Value {
    serde_json::json!({
        SECRET_EXPORT_AUTHORIZATION_KEY: SecretExportAuthorization {
            document_id: document_id.to_string(),
            voter_secret_attributes,
            expires_at: Utc::now() + secret_export_grant_ttl(),
        }
    })
}

/// Validates the durable authorization grant created by the authenticated HTTP
/// request. Broker payloads are not an authorization boundary: publishers that
/// can reach RabbitMQ must not be able to grant themselves secret-voter access.
pub fn validate_secret_export_task(
    task: &TasksExecution,
    tenant_id: &str,
    election_event_id: &str,
    task_type: &ETasksExecution,
    document_id: &str,
    requires_voter_secrets: bool,
) -> Result<()> {
    validate_export_task_binding(task, tenant_id, election_event_id, task_type)?;

    validate_secret_export_authorization(task, document_id, requires_voter_secrets, true)
}

/// Validates the durable grant for an already-created document. Expiry stops
/// new secret processing, but must not strand a task if an authorized attempt
/// committed its document immediately before the worker crashed.
pub fn validate_secret_export_task_for_recovery(
    task: &TasksExecution,
    tenant_id: &str,
    election_event_id: &str,
    task_type: &ETasksExecution,
    document_id: &str,
    requires_voter_secrets: bool,
) -> Result<()> {
    validate_export_task_binding(task, tenant_id, election_event_id, task_type)?;
    validate_secret_export_authorization(task, document_id, requires_voter_secrets, false)
}

fn validate_secret_export_authorization(
    task: &TasksExecution,
    document_id: &str,
    requires_voter_secrets: bool,
    enforce_expiry: bool,
) -> Result<()> {
    let authorization = task
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get(SECRET_EXPORT_AUTHORIZATION_KEY))
        .cloned()
        .context("Export task is missing its persisted authorization grant")?;
    let authorization: SecretExportAuthorization = serde_json::from_value(authorization)
        .context("Export task has an invalid persisted authorization grant")?;

    if authorization.document_id != document_id {
        anyhow::bail!("Export task document does not match its persisted authorization grant");
    }
    if enforce_expiry && Utc::now() >= authorization.expires_at {
        anyhow::bail!("Export task authorization grant has expired");
    }
    if requires_voter_secrets && !authorization.voter_secret_attributes {
        anyhow::bail!("Export task is not authorized to read voter secret attributes");
    }

    Ok(())
}

pub fn validate_export_task_binding(
    task: &TasksExecution,
    tenant_id: &str,
    election_event_id: &str,
    task_type: &ETasksExecution,
) -> Result<()> {
    if !export_task_identity_matches(task, tenant_id, election_event_id, task_type)
        || task.execution_status != TasksExecutionStatus::IN_PROGRESS.to_string()
    {
        anyhow::bail!("Export task does not match the persisted task execution");
    }

    Ok(())
}

fn export_task_identity_matches(
    task: &TasksExecution,
    tenant_id: &str,
    election_event_id: &str,
    task_type: &ETasksExecution,
) -> bool {
    task.tenant_id == tenant_id
        && task.election_event_id.as_deref() == Some(election_event_id)
        && task.task_type == task_type.to_string()
}

pub fn is_matching_completed_export_task(
    task: &TasksExecution,
    tenant_id: &str,
    election_event_id: &str,
    task_type: &ETasksExecution,
    document_id: &str,
) -> bool {
    export_task_identity_matches(task, tenant_id, election_event_id, task_type)
        && task.execution_status == TasksExecutionStatus::SUCCESS.to_string()
        && task
            .annotations
            .as_ref()
            .and_then(|annotations| annotations.get("document_id"))
            .and_then(serde_json::Value::as_str)
            == Some(document_id)
}

#[instrument(skip_all, err)]
pub async fn post(
    tenant_id: &str,
    election_event_id: Option<&str>,
    task_type: ETasksExecution,
    executed_by_user: &str,
) -> Result<TasksExecution, anyhow::Error> {
    post_internal(
        tenant_id,
        election_event_id,
        task_type,
        executed_by_user,
        None,
    )
    .await
}

#[instrument(skip_all, err)]
pub async fn post_with_annotations(
    tenant_id: &str,
    election_event_id: Option<&str>,
    task_type: ETasksExecution,
    executed_by_user: &str,
    annotations: serde_json::Value,
) -> Result<TasksExecution, anyhow::Error> {
    post_internal(
        tenant_id,
        election_event_id,
        task_type,
        executed_by_user,
        Some(annotations),
    )
    .await
}

async fn post_internal(
    tenant_id: &str,
    election_event_id: Option<&str>,
    task_type: ETasksExecution,
    executed_by_user: &str,
    annotations: Option<serde_json::Value>,
) -> Result<TasksExecution, anyhow::Error> {
    let logs = serde_json::to_value(general_start_log())?;

    let task = insert_tasks_execution(
        tenant_id,
        election_event_id,
        &task_type.to_name(),
        &task_type.to_string(),
        TasksExecutionStatus::IN_PROGRESS,
        annotations,
        None,
        Some(logs),
        executed_by_user,
    )
    .await
    .context("Failed to insert task execution record")?;

    Ok(task)
}

#[cfg(test)]
mod secret_export_tests {
    use super::*;
    use chrono::Local;

    fn task(annotations: Option<serde_json::Value>) -> TasksExecution {
        TasksExecution {
            id: "task-id".to_string(),
            tenant_id: "tenant-id".to_string(),
            election_event_id: Some("event-id".to_string()),
            name: "Export Voters".to_string(),
            task_type: ETasksExecution::EXPORT_VOTERS.to_string(),
            execution_status: TasksExecutionStatus::IN_PROGRESS.to_string(),
            created_at: Local::now(),
            start_at: None,
            end_at: None,
            annotations,
            labels: None,
            logs: None,
            executed_by_user: "user".to_string(),
        }
    }

    #[test]
    fn accepts_a_matching_persisted_secret_grant() {
        let task = task(Some(secret_export_task_annotations("document-id", true)));

        assert!(validate_secret_export_task(
            &task,
            "tenant-id",
            "event-id",
            &ETasksExecution::EXPORT_VOTERS,
            "document-id",
            true,
        )
        .is_ok());
    }

    #[test]
    fn rejects_a_broker_claim_without_a_persisted_secret_grant() {
        let task = task(Some(secret_export_task_annotations("document-id", false)));

        assert!(validate_secret_export_task(
            &task,
            "tenant-id",
            "event-id",
            &ETasksExecution::EXPORT_VOTERS,
            "document-id",
            true,
        )
        .is_err());
    }

    #[test]
    fn rejects_reusing_a_grant_for_another_document() {
        let task = task(Some(secret_export_task_annotations("other-document", true)));

        assert!(validate_secret_export_task(
            &task,
            "tenant-id",
            "event-id",
            &ETasksExecution::EXPORT_VOTERS,
            "document-id",
            true,
        )
        .is_err());
    }

    #[test]
    fn rejects_an_expired_grant() {
        let annotations = serde_json::json!({
            SECRET_EXPORT_AUTHORIZATION_KEY: SecretExportAuthorization {
                document_id: "document-id".to_string(),
                voter_secret_attributes: true,
                expires_at: Utc::now() - Duration::seconds(1),
            }
        });
        let task = task(Some(annotations));

        assert!(validate_secret_export_task(
            &task,
            "tenant-id",
            "event-id",
            &ETasksExecution::EXPORT_VOTERS,
            "document-id",
            true,
        )
        .is_err());
    }

    #[test]
    fn accepts_an_expired_grant_only_to_recover_its_existing_document() {
        let annotations = serde_json::json!({
            SECRET_EXPORT_AUTHORIZATION_KEY: SecretExportAuthorization {
                document_id: "document-id".to_string(),
                voter_secret_attributes: true,
                expires_at: Utc::now() - Duration::seconds(1),
            }
        });
        let task = task(Some(annotations));

        assert!(validate_secret_export_task_for_recovery(
            &task,
            "tenant-id",
            "event-id",
            &ETasksExecution::EXPORT_VOTERS,
            "document-id",
            true,
        )
        .is_ok());
    }

    #[test]
    fn recognizes_only_the_exact_completed_export_result() {
        let mut task = task(Some(serde_json::json!({ "document_id": "document-id" })));
        task.execution_status = TasksExecutionStatus::SUCCESS.to_string();

        assert!(is_matching_completed_export_task(
            &task,
            "tenant-id",
            "event-id",
            &ETasksExecution::EXPORT_VOTERS,
            "document-id",
        ));
        assert!(!is_matching_completed_export_task(
            &task,
            "tenant-id",
            "event-id",
            &ETasksExecution::EXPORT_VOTERS,
            "other-document",
        ));
        assert!(!is_matching_completed_export_task(
            &task,
            "other-tenant",
            "event-id",
            &ETasksExecution::EXPORT_VOTERS,
            "document-id",
        ));
    }
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
    update_with_annotations(tenant_id, task_id, status, logs, annotations).await
}

/// Updates a task with caller-owned structured annotations. Most tasks only
/// need `document_id` and should use `update`; workflows with typed result
/// data (for example reconciliation row failures) use this instead of
/// encoding machine-readable state in human log strings.
#[instrument(skip_all, err)]
pub async fn update_with_annotations(
    tenant_id: &str,
    task_id: &str,
    status: TasksExecutionStatus,
    logs: serde_json::Value,
    annotations: serde_json::Value,
) -> Result<(), anyhow::Error> {
    update_task_execution_status(tenant_id, task_id, status, Some(logs), annotations)
        .await
        .context("Failed to update task execution record")?;
    Ok(())
}

// TODO filter also by tenant-id and document-id
#[instrument(skip_all, err)]
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

/// Completes an export only if it is still active. This keeps late duplicate
/// deliveries from mutating a terminal task.
#[instrument(skip_all, err)]
pub async fn update_export_complete(
    task: &TasksExecution,
    document_id: String,
) -> Result<(), anyhow::Error> {
    let new_logs = serde_json::to_value(append_general_log(
        &task.logs,
        "Task completed successfully",
    ))?;
    let annotations = serde_json::to_value(TaskAnnotations {
        document_id: Some(document_id),
    })?;
    let updated = update_export_task_execution_status_if_in_progress(
        &task.tenant_id,
        &task.id,
        TasksExecutionStatus::SUCCESS,
        Some(new_logs),
        annotations,
    )
    .await
    .context("Failed to update export task execution record")?;

    if !updated {
        anyhow::bail!("Export task is no longer in progress");
    }
    Ok(())
}

#[instrument(skip_all, err)]
pub async fn update_complete_with_annotations(
    task: &TasksExecution,
    annotations: serde_json::Value,
) -> Result<(), anyhow::Error> {
    let new_logs = serde_json::to_value(append_general_log(
        &task.logs,
        "Task completed successfully",
    ))?;
    update_with_annotations(
        &task.tenant_id,
        &task.id,
        TasksExecutionStatus::SUCCESS,
        new_logs,
        annotations,
    )
    .await
    .context("Failed to update task execution record")?;
    Ok(())
}

// TODO filter also by tenant-id and document-id
#[instrument(skip_all, err)]
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

/// Fails an export only if it is still active. In particular, a replay that
/// fails validation cannot regress SUCCESS or erase the document id.
#[instrument(skip_all, err)]
pub async fn update_export_fail(
    task: &TasksExecution,
    err_message: &str,
) -> Result<(), anyhow::Error> {
    let new_logs = serde_json::to_value(append_general_log(
        &task.logs,
        &("Error: ".to_owned() + err_message),
    ))?;
    let annotations = serde_json::to_value(TaskAnnotations { document_id: None })?;

    update_export_task_execution_status_if_in_progress(
        &task.tenant_id,
        &task.id,
        TasksExecutionStatus::FAILED,
        Some(new_logs),
        annotations,
    )
    .await
    .context("Failed to update export task execution record with failure status")?;

    Ok(())
}

/// Marks a task as failed without overwriting annotations that were committed
/// as durable retry state by the task's resource transaction.
#[instrument(skip_all, err)]
pub async fn update_fail_preserving_annotations(
    task: &TasksExecution,
    err_message: &str,
) -> Result<(), anyhow::Error> {
    let new_logs = serde_json::to_value(append_general_log(
        &task.logs,
        &("Error: ".to_owned() + err_message),
    ))?;

    update_task_execution_status(
        &task.tenant_id,
        &task.id,
        TasksExecutionStatus::FAILED,
        Some(new_logs),
        serde_json::json!({}),
    )
    .await
    .context("Failed to update task execution record with failure status")
}
