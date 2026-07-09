// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::cast_vote::has_valid_cast_vote;
use crate::postgres::election_event::{get_election_event_by_id, ElectionEventDatafix};
use crate::services::database::get_hasura_pool;
use crate::services::datafix;
use crate::services::datafix::types::{SoapRequest, SoapRequestResponse};
use crate::services::datafix::utils::post_operation_result_to_electoral_log;
use crate::services::serialize_tasks_logs::append_general_log;
use crate::services::tasks_execution::*;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction};
use electoral_log::messages::newtypes::ExtApiRequestDirection;
use sequent_core::services::keycloak::{get_event_realm, KeycloakAdminClient};
use sequent_core::types::hasura::core::{ElectionEvent, TasksExecution};
use sequent_core::types::keycloak::User;
use sequent_core::util::retry::retry_with_exponential_backoff;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, instrument};

/// Input for the `edit_user` task. Mirrors the fields the `/edit-user` route
/// forwards to Keycloak. Only launched when disabling a voter in a Datafix
/// election event, so `election_event_id` is always present.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EditUserTaskBody {
    pub tenant_id: String,
    pub user_id: String,
    pub election_event_id: String,
    pub enabled: Option<bool>,
    pub attributes: HashMap<String, Vec<String>>,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub temporary: Option<bool>,
}

/// Response of the `/edit-user` route. For Datafix election events the edit is
/// deferred to the `edit_user` task and `task_execution` is returned (`user` is
/// `None`); otherwise the voter is edited synchronously and returned in `user`
/// with `task_execution` set to `None`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EditUserOutput {
    pub user: Option<User>,
    pub task_execution: Option<TasksExecution>,
}

/// Edits a Datafix voter asynchronously: when the edit disables a voter that
/// already cast a valid vote, VoterView is notified first (`SetNotVoted`, a
/// Datafix requirement) and only then is the voter edited in Keycloak. The
/// VoterView outcome (operation + direction) is mirrored into the task logs so
/// it matches the electoral-log entry.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
// `max_retries = 0`: a retry would re-issue the `SetNotVoted` request to
// VoterView (a non-idempotent external side effect), so failures are terminal.
#[celery::task(max_retries = 0)]
pub async fn edit_user(body: EditUserTaskBody, task_execution: TasksExecution) -> Result<()> {
    let mut hasura_db_client: DbClient = match get_hasura_pool().await.get().await {
        Ok(client) => client,
        Err(err) => {
            update_fail(&task_execution, "Failed to get Hasura DB pool").await?;
            return Err(Error::String(format!(
                "Error getting Hasura DB pool: {err}"
            )));
        }
    };

    let hasura_transaction = match hasura_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            update_fail(&task_execution, "Failed to start Hasura transaction").await?;
            return Err(Error::String(format!(
                "Error starting Hasura transaction: {err}"
            )));
        }
    };

    let realm = get_event_realm(&body.tenant_id, &body.election_event_id);

    let client = match KeycloakAdminClient::new().await {
        Ok(client) => client,
        Err(err) => {
            let msg = format!("Error obtaining the Keycloak client: {err:?}");
            update_fail(&task_execution, &msg).await?;
            return Err(Error::String(msg));
        }
    };

    let current_user = match client.get_user(&realm, &body.user_id).await {
        Ok(user) => user,
        Err(err) => {
            let msg = format!("Error fetching the voter from Keycloak: {err:?}");
            update_fail(&task_execution, &msg).await?;
            return Err(Error::String(msg));
        }
    };

    // The task is only dispatched when disabling a voter in a Datafix election
    // event (the `/edit-user` route enforces both), so notify VoterView before
    // editing Keycloak. A failed notification aborts the edit, so the voter
    // stays enabled.
    let election_event = match get_election_event_by_id(
        &hasura_transaction,
        &body.tenant_id,
        &body.election_event_id,
    )
    .await
    {
        Ok(election_event) => election_event,
        Err(err) => {
            let msg = format!("Error getting election event by id: {err:?}");
            update_fail(&task_execution, &msg).await?;
            return Err(Error::String(msg));
        }
    };

    let mut task_execution = task_execution;
    match set_not_voted_in_voterview(
        &hasura_transaction,
        election_event,
        &body.tenant_id,
        &body.election_event_id,
        &body.user_id,
        current_user.username.clone(),
    )
    .await
    {
        // VoterView notified: mirror the electoral-log entry into the task logs
        // and continue with the Keycloak edit.
        Ok(Some(operation)) => {
            task_execution = append_operation_log(
                &task_execution,
                &operation,
                ExtApiRequestDirection::Outbound,
            )?;
        }
        // No prior valid vote: nothing to notify, nothing logged.
        Ok(None) => {}
        // VoterView notification failed: abort the edit.
        Err(operation) => {
            let msg = format!("{operation} ({})", ExtApiRequestDirection::Outbound);
            update_fail(&task_execution, &msg).await?;
            return Err(Error::String(msg));
        }
    }

    if let Err(err) = client
        .edit_user(
            &realm,
            &body.user_id,
            body.enabled,
            Some(body.attributes.clone()),
            body.email.clone(),
            body.first_name.clone(),
            body.last_name.clone(),
            body.username.clone(),
            body.password.clone(),
            body.temporary,
        )
        .await
    {
        let msg = format!("Error editing the voter in Keycloak: {err:?}");
        update_fail(&task_execution, &msg).await?;
        return Err(Error::String(msg));
    }

    update_complete(&task_execution, None)
        .await
        .context("Failed to update task execution status to COMPLETED")?;

    Ok(())
}

/// Notifies VoterView that a disabled voter can vote through another channel
/// (`SetNotVoted`, a Datafix requirement) and records the outcome in the
/// electoral log. Returns `Ok(None)` when the voter has no prior valid vote
/// (nothing to clear and nothing logged), `Ok(Some(operation))` on success, and
/// `Err(operation)` on failure, so the caller can abort the edit. The
/// `operation` string is the same one written to the electoral log.
#[instrument(skip(hasura_transaction, election_event))]
async fn set_not_voted_in_voterview(
    hasura_transaction: &Transaction<'_>,
    election_event: ElectionEvent,
    tenant_id: &str,
    election_event_id: &str,
    user_id: &str,
    username: Option<String>,
) -> std::result::Result<Option<String>, String> {
    let prior_valid_vote =
        has_valid_cast_vote(hasura_transaction, tenant_id, election_event_id, user_id)
            .await
            .map_err(|e| format!("Error checking prior valid cast votes: {e:?}"))?;

    if !prior_valid_vote {
        info!("Voter {user_id} has no prior valid cast votes, skipping SetNotVoted request to VoterView");
        return Ok(None);
    }

    // Retry only transport/HTTP-level failures (Err and SOAP faults): a
    // `Success=false` answer such as "The voter has not voted." is a
    // definitive VoterView response and must not be repeated.
    let res = retry_with_exponential_backoff(
        || async {
            match datafix::voterview_requests::send(
                SoapRequest::SetNotVoted,
                ElectionEventDatafix(election_event.clone()),
                &username,
            )
            .await?
            {
                SoapRequestResponse::Faultstring(msg) => Err(anyhow!("{msg}")),
                response => Ok(response),
            }
        },
        3,
        Duration::from_millis(500),
    )
    .await;

    let req_type = SoapRequest::SetNotVoted;
    let operation = match &res {
        Ok(SoapRequestResponse::Ok) => format!("{req_type} Succeeded"),
        Ok(response) => req_type.failed_operation(response.error_message()),
        Err(e) => req_type.failed_operation(Some(e.to_string())),
    };

    post_operation_result_to_electoral_log(
        hasura_transaction,
        tenant_id,
        election_event_id,
        user_id,
        username.as_deref().unwrap_or_default(),
        ExtApiRequestDirection::Outbound,
        operation.clone(),
    )
    .await;

    match res {
        Ok(SoapRequestResponse::Ok) => Ok(Some(operation)),
        _ => Err(operation),
    }
}

/// Appends the electoral-log operation (with its request direction) to the task
/// execution logs, so the widget shows the same information recorded in the
/// electoral log.
fn append_operation_log(
    task: &TasksExecution,
    operation: &str,
    direction: ExtApiRequestDirection,
) -> Result<TasksExecution> {
    let mut task = task.clone();
    let logs = append_general_log(&task.logs, &format!("{operation} ({direction})"));
    task.logs = Some(serde_json::to_value(logs)?);
    Ok(task)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;

    fn dummy_task(logs: Option<serde_json::Value>) -> TasksExecution {
        TasksExecution {
            id: "task-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            election_event_id: Some("event-1".to_string()),
            name: "Edit Voter".to_string(),
            task_type: "EDIT_USER".to_string(),
            execution_status: "IN_PROGRESS".to_string(),
            created_at: Local::now(),
            start_at: None,
            end_at: None,
            annotations: None,
            labels: None,
            logs,
            executed_by_user: "admin".to_string(),
        }
    }

    fn log_texts(task: &TasksExecution) -> Vec<String> {
        task.logs
            .as_ref()
            .and_then(|v| v.as_array())
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(|e| e.get("log_text").and_then(|t| t.as_str()))
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn append_operation_log_records_operation_and_direction() {
        let task = dummy_task(None);
        let updated = append_operation_log(
            &task,
            "SetNotVoted Succeeded",
            ExtApiRequestDirection::Outbound,
        )
        .unwrap();

        let texts = log_texts(&updated);
        assert_eq!(texts.len(), 1);
        assert_eq!(texts[0], "SetNotVoted Succeeded (Outbound)");
    }

    #[test]
    fn append_operation_log_preserves_existing_entries() {
        let existing = serde_json::json!([
            {"created_date": "2025-01-01T00:00:00.000Z", "log_text": "Task started"}
        ]);
        let task = dummy_task(Some(existing));
        let updated = append_operation_log(
            &task,
            "SetNotVoted Failed: boom",
            ExtApiRequestDirection::Outbound,
        )
        .unwrap();

        let texts = log_texts(&updated);
        assert_eq!(texts.len(), 2);
        assert!(texts.contains(&"Task started".to_string()));
        assert!(texts.contains(&"SetNotVoted Failed: boom (Outbound)".to_string()));
    }
}
