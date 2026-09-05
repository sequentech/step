// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::document::get_document;
use crate::postgres::tasks_execution::{
    get_task_by_id_with_transaction, merge_task_execution_annotations,
};
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::document_password::read_password;
use crate::services::documents::upload_and_return_document_with_annotations;
use crate::services::electoral_log::{
    prepare_voter_password_change, ElectoralLogAdminContext, PreparedVoterPasswordChangeLog,
    VoterPasswordChangeSource,
};
use crate::services::pdf_encryption::encrypt_pdf;
use crate::services::pg_lock::PgLock;
use crate::services::reports::voter_information_letter::VoterInformationLetterTemplate;
use crate::services::tasks_execution::{
    update_complete_with_annotations, update_fail_preserving_annotations,
};
use crate::services::tasks_semaphore::acquire_semaphore;
use anyhow::{anyhow, Context, Result};
use celery::error::TaskError;
use chrono::Duration;
use deadpool_postgres::Client as DbClient;
use sequent_core::services::date::ISO8601;
use sequent_core::services::keycloak::{
    get_event_realm, get_realm_password_policy, KeycloakAdminClient,
};
use sequent_core::types::hasura::core::{DocumentAnnotations, TasksExecution};
use sequent_core::util::temp_path::write_into_named_temp_file;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::time::Duration as StdDuration;
use tracing::{error, instrument};
use uuid::Uuid;

const FAILURE_MESSAGE: &str = "Voter Information Letter generation failed";
const VOTER_LOCKED_MESSAGE: &str = "Another operation is updating this voter";
// A single renderer attempt can launch Chrome up to five times with backoff.
// Keep a five-minute lease, renewed every 30 seconds, so a live attempt remains
// serialized while an abandoned lock clears well before the next manual retry.
const VIL_CREDENTIAL_LOCK_EXPIRY_SECONDS: i64 = 5 * 60;
const VIL_CREDENTIAL_LOCK_HEARTBEAT_SECONDS: u64 = 30;

#[derive(Debug)]
enum VoterInformationLetterTaskError {
    Retryable(anyhow::Error),
    NonRetryable(String),
}

impl Display for VoterInformationLetterTaskError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Retryable(error) => write!(formatter, "{error:#}"),
            Self::NonRetryable(error) => formatter.write_str(error),
        }
    }
}

impl From<VoterInformationLetterTaskError> for TaskError {
    fn from(error: VoterInformationLetterTaskError) -> Self {
        match error {
            VoterInformationLetterTaskError::Retryable(error) => {
                TaskError::ExpectedError(format!("{error:#}"))
            }
            VoterInformationLetterTaskError::NonRetryable(error) => {
                TaskError::UnexpectedError(error)
            }
        }
    }
}

fn voter_credential_lock_key(tenant_id: &str, election_event_id: &str, voter_id: &str) -> String {
    format!("voter-credential-{tenant_id}-{election_event_id}-{voter_id}")
}

async fn run_with_lock_heartbeat<F, T>(lock: &PgLock, operation: F) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    tokio::pin!(operation);
    let mut heartbeat = tokio::time::interval(StdDuration::from_secs(
        VIL_CREDENTIAL_LOCK_HEARTBEAT_SECONDS,
    ));
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    heartbeat.tick().await;

    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                lock.update_expiry_for(VIL_CREDENTIAL_LOCK_EXPIRY_SECONDS)
                    .await
                    .context("Failed to renew the voter credential lock")?;
            }
            result = &mut operation => return result,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct VoterInformationLetterTaskAnnotations {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    document_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    voter_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    password_change_audit: Option<VoterPasswordChangeAuditDelivery>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct VoterPasswordChangeAuditDelivery {
    delivered: bool,
    log: PreparedVoterPasswordChangeLog,
}

impl VoterInformationLetterTaskAnnotations {
    fn pending(document_id: &str, voter_id: &str, log: PreparedVoterPasswordChangeLog) -> Self {
        Self {
            document_id: Some(document_id.to_string()),
            voter_id: Some(voter_id.to_string()),
            password_change_audit: Some(VoterPasswordChangeAuditDelivery {
                delivered: false,
                log,
            }),
        }
    }

    fn validate_for(&self, document_id: &str, voter_id: &str) -> Result<()> {
        if self.password_change_audit.is_some()
            && (self.document_id.as_deref() != Some(document_id)
                || self.voter_id.as_deref() != Some(voter_id))
        {
            return Err(anyhow!(
                "Task audit state does not match the Voter Information Letter document"
            ));
        }
        Ok(())
    }
}

async fn deliver_audit_and_complete(
    task_execution: &TasksExecution,
    mut annotations: VoterInformationLetterTaskAnnotations,
) -> Result<()> {
    let mut annotations_changed = false;
    if let Some(audit) = annotations.password_change_audit.as_mut() {
        if !audit.delivered {
            audit
                .log
                .post()
                .await
                .context("Voter credential changed, but its electoral-log entry failed")?;
            audit.delivered = true;
            annotations_changed = true;
        }
    }

    if task_execution.execution_status == "SUCCESS" && !annotations_changed {
        return Ok(());
    }

    update_complete_with_annotations(task_execution, serde_json::to_value(annotations)?).await?;
    Ok(())
}

#[instrument(skip_all, err)]
async fn generate(
    tenant_id: &str,
    election_event_id: &str,
    voter_id: &str,
    document_id: &str,
    password_secret_id: &str,
    password_change_initiator: &ElectoralLogAdminContext,
    task_execution: &TasksExecution,
    may_read_secret_attributes: bool,
) -> Result<()> {
    let mut hasura_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .context("Failed to get Hasura DB client")?;
    let hasura_transaction = hasura_client
        .transaction()
        .await
        .context("Failed to start Hasura transaction")?;
    let current_task_execution =
        get_task_by_id_with_transaction(&hasura_transaction, tenant_id, &task_execution.id)
            .await
            .context("Failed to load the current Voter Information Letter task state")?;
    let current_annotations: VoterInformationLetterTaskAnnotations = current_task_execution
        .annotations
        .clone()
        .map(serde_json::from_value)
        .transpose()
        .context("Failed to parse Voter Information Letter task annotations")?
        .unwrap_or_default();
    current_annotations.validate_for(document_id, voter_id)?;

    if get_document(
        &hasura_transaction,
        tenant_id,
        Some(election_event_id.to_string()),
        document_id,
    )
    .await?
    .is_some()
    {
        hasura_transaction.commit().await?;
        return deliver_audit_and_complete(&current_task_execution, current_annotations).await;
    }

    let document_password = read_password(
        &hasura_transaction,
        tenant_id,
        Some(election_event_id),
        document_id,
        password_secret_id,
    )
    .await?
    .ok_or_else(|| anyhow!("Document password secret is not available"))?;
    let voter_password = get_realm_password_policy(tenant_id, election_event_id)
        .await
        .context("Failed to load the election event password policy")?
        .generate_password()
        .context("The election event password policy is not configured or valid")?;

    let mut keycloak_client = get_keycloak_pool()
        .await
        .get()
        .await
        .context("Failed to get Keycloak DB client")?;
    let keycloak_transaction = keycloak_client
        .transaction()
        .await
        .context("Failed to start Keycloak transaction")?;

    let report = VoterInformationLetterTemplate::new(
        tenant_id.to_string(),
        election_event_id.to_string(),
        voter_id.to_string(),
        voter_password.clone(),
        may_read_secret_attributes,
    );
    let pdf = report
        .render_pdf(&hasura_transaction, &keycloak_transaction)
        .await?;
    let encrypted_pdf = encrypt_pdf(&pdf, &document_password.password)?;

    let (_temporary_file, path, file_size) =
        write_into_named_temp_file(&encrypted_pdf, "voter-information-letter-", ".pdf")
            .context("Failed to create encrypted PDF temporary file")?;
    let document_name = format!("voter-information-letter-{voter_id}.pdf");
    let mut document_annotations =
        DocumentAnnotations::password_protected(password_secret_id.to_string());
    if may_read_secret_attributes {
        if let Some(access) = document_annotations.access.as_mut() {
            access.voter_secret_attributes = true;
        }
    }
    upload_and_return_document_with_annotations(
        &hasura_transaction,
        &path,
        file_size,
        "application/pdf",
        tenant_id,
        Some(election_event_id.to_string()),
        &document_name,
        Some(document_id.to_string()),
        false,
        &document_annotations,
    )
    .await
    .context("Failed to store encrypted Voter Information Letter")?;

    let voter = KeycloakAdminClient::new()
        .await
        .context("Failed to initialize Keycloak admin client")?
        .edit_user(
            &get_event_realm(tenant_id, election_event_id),
            voter_id,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(voter_password),
            Some(false),
        )
        .await
        .context("Failed to assign the generated voter credential")?;

    let prepared_audit = prepare_voter_password_change(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        voter_id,
        voter.username,
        password_change_initiator,
        VoterPasswordChangeSource::VoterInformationLetter,
    )
    .await
    .context("Failed to prepare the voter password-change electoral-log entry")?;
    let pending_annotations =
        VoterInformationLetterTaskAnnotations::pending(document_id, voter_id, prepared_audit);
    let pending_annotations_value = serde_json::to_value(&pending_annotations)?;
    merge_task_execution_annotations(
        &hasura_transaction,
        tenant_id,
        &task_execution.id,
        &pending_annotations_value,
    )
    .await
    .context("Failed to persist pending Voter Information Letter audit state")?;

    hasura_transaction
        .commit()
        .await
        .context("Failed to commit Voter Information Letter document")?;
    deliver_audit_and_complete(task_execution, pending_annotations).await
}

#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 2, retry_for_unexpected = false)]
pub async fn generate_voter_information_letter(
    tenant_id: String,
    election_event_id: String,
    voter_id: String,
    document_id: String,
    password_secret_id: String,
    password_change_initiator: ElectoralLogAdminContext,
    task_execution: TasksExecution,
    may_read_secret_attributes: bool,
) -> std::result::Result<(), VoterInformationLetterTaskError> {
    let _permit = acquire_semaphore()
        .await
        .map_err(VoterInformationLetterTaskError::Retryable)?;
    let lock = match PgLock::acquire(
        voter_credential_lock_key(&tenant_id, &election_event_id, &voter_id),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(VIL_CREDENTIAL_LOCK_EXPIRY_SECONDS),
    )
    .await
    {
        Ok(lock) => lock,
        Err(lock_error) => {
            error!(
                task_id = %task_execution.id,
                "{VOTER_LOCKED_MESSAGE}: {lock_error:#}"
            );
            update_fail_preserving_annotations(&task_execution, VOTER_LOCKED_MESSAGE)
                .await
                .ok();
            return Err(VoterInformationLetterTaskError::NonRetryable(format!(
                "{VOTER_LOCKED_MESSAGE}: {lock_error:#}"
            )));
        }
    };

    let generation_result = run_with_lock_heartbeat(
        &lock,
        generate(
            &tenant_id,
            &election_event_id,
            &voter_id,
            &document_id,
            &password_secret_id,
            &password_change_initiator,
            &task_execution,
            may_read_secret_attributes,
        ),
    )
    .await;
    let release_result = lock.release().await;

    if let Err(generation_error) = generation_result {
        if let Err(release_error) = &release_result {
            error!(
                task_id = %task_execution.id,
                "Failed to release voter credential lock after generation failure: {release_error:#}"
            );
        }
        error!(
            task_id = %task_execution.id,
            "Voter Information Letter generation failed"
        );
        update_fail_preserving_annotations(&task_execution, FAILURE_MESSAGE)
            .await
            .ok();
        return Err(VoterInformationLetterTaskError::Retryable(anyhow!(
            "Voter Information Letter task failed: {generation_error:#}"
        )));
    }

    if let Err(release_error) = release_result {
        error!(
            task_id = %task_execution.id,
            "Failed to release voter credential lock: {release_error:#}"
        );
        update_fail_preserving_annotations(&task_execution, FAILURE_MESSAGE)
            .await
            .ok();
        return Err(VoterInformationLetterTaskError::Retryable(anyhow!(
            "Failed to release voter credential lock: {release_error:#}"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{voter_credential_lock_key, VoterInformationLetterTaskError};
    use celery::error::TaskError;

    #[test]
    fn voter_credential_lock_is_scoped_to_tenant_event_and_voter() {
        assert_eq!(
            "voter-credential-tenant-a-event-b-voter-c",
            voter_credential_lock_key("tenant-a", "event-b", "voter-c")
        );
        assert_ne!(
            voter_credential_lock_key("tenant-a", "event-b", "voter-c"),
            voter_credential_lock_key("tenant-a", "event-b", "voter-d")
        );
    }

    #[test]
    fn lock_conflicts_are_non_retryable_but_generation_errors_retry() {
        assert!(matches!(
            TaskError::from(VoterInformationLetterTaskError::NonRetryable(
                "Another operation is updating this voter".to_string()
            )),
            TaskError::UnexpectedError(_)
        ));
        assert!(matches!(
            TaskError::from(VoterInformationLetterTaskError::Retryable(anyhow::anyhow!(
                "render failed"
            ))),
            TaskError::ExpectedError(_)
        ));
    }
}
