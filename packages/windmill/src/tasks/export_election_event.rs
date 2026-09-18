// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::database::get_hasura_pool;
use crate::services::export::export_election_event::process_export_zip;
use crate::services::tasks_execution::*;
use crate::types::error::{Error, Result};
use anyhow::Context;
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct ExportOptions {
    pub password: Option<String>,
    pub is_encrypted: bool,
    pub encrypt_with_password: bool,
    pub include_voters: bool,
    pub contains_voter_secrets: bool,
    pub activity_logs: bool,
    pub bulletin_board: bool,
    pub publications: bool,
    pub s3_files: bool,
    pub scheduled_events: bool,
    pub reports: bool,
    pub applications: bool,
    pub tally: bool,
    pub include_certificates: bool,
}

fn validate_voter_secret_export(export_config: &ExportOptions) -> Result<()> {
    if !export_config.contains_voter_secrets {
        return Ok(());
    }

    let has_password = match export_config.password.as_deref() {
        Some(password) => !password.is_empty(),
        None => false,
    };
    let is_authorized_password_export = (export_config.include_voters || export_config.s3_files)
        && export_config.encrypt_with_password
        && export_config.is_encrypted
        && has_password;

    if !is_authorized_password_export {
        return Err(Error::String(
            "Election-event voter secret export requires password encryption and voter-secret-attribute-read authorization"
                .to_string(),
        ));
    }

    Ok(())
}

async fn export_election_event_impl(
    tenant_id: String,
    election_event_id: String,
    export_config: ExportOptions,
    document_id: String,
    task_execution: TasksExecution,
) -> Result<()> {
    let mut hasura_db_client: DbClient = match get_hasura_pool().await.get().await {
        Ok(client) => client,
        Err(err) => {
            let err_str = format!("Failed to get Hasura DB pool: {err:?}");
            return Err(Error::String(err_str));
        }
    };

    let hasura_transaction = match hasura_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            let err_str = format!("Failed to start Hasura transaction: {err:?}");
            return Err(Error::String(err_str));
        }
    };

    crate::postgres::tasks_execution::lock_export_task_with_transaction(
        &hasura_transaction,
        &task_execution.id,
    )
    .await
    .context("Failed to lock election-event export task")?;

    // Reload the task from PostgreSQL. Only its opaque id comes from RabbitMQ;
    // task scope and secret authorization come from the durable HTTP-side row.
    let persisted_task = crate::postgres::tasks_execution::get_task_by_id_with_transaction(
        &hasura_transaction,
        &tenant_id,
        &task_execution.id,
    )
    .await
    .context("Failed to load persisted election-event export task")?;

    if is_matching_completed_export_task(
        &persisted_task,
        &tenant_id,
        &election_event_id,
        &crate::types::tasks::ETasksExecution::EXPORT_ELECTION_EVENT,
        &document_id,
    ) {
        hasura_transaction
            .commit()
            .await
            .context("Failed to release completed election-event export task lock")?;
        return Ok(());
    }

    if let Err(error) = validate_voter_secret_export(&export_config) {
        drop(hasura_transaction);
        update_export_fail(&persisted_task, &error.to_string()).await?;
        return Err(error);
    }

    let recovery_authorization_result = validate_secret_export_task_for_recovery(
        &persisted_task,
        &tenant_id,
        &election_event_id,
        &crate::types::tasks::ETasksExecution::EXPORT_ELECTION_EVENT,
        &document_id,
        export_config.contains_voter_secrets,
    );
    if let Err(error) = recovery_authorization_result {
        drop(hasura_transaction);
        if let Err(update_error) = update_export_fail(&persisted_task, &error.to_string()).await {
            event!(
                Level::ERROR,
                "Failed to update task execution status to FAILED: {:?}",
                update_error
            );
        }
        return Err(Error::String(error.to_string()));
    }

    let existing_document = match crate::postgres::document::get_document(
        &hasura_transaction,
        &tenant_id,
        Some(election_event_id.clone()),
        &document_id,
    )
    .await
    {
        Ok(document) => document.is_some(),
        Err(error) => {
            drop(hasura_transaction);
            let error =
                error.context("Failed to check for an existing election-event export document");
            update_export_fail(&persisted_task, &error.to_string()).await?;
            return Err(Error::String(error.to_string()));
        }
    };
    if existing_document {
        update_export_complete(&persisted_task, document_id.clone())
            .await
            .context("Failed to recover completed election-event export task")?;
        hasura_transaction
            .commit()
            .await
            .context("Failed to release recovered election-event export task lock")?;
        return Ok(());
    }

    // Recovery above only verifies the durable binding. Starting the actual
    // export still requires a grant that has not expired.
    if let Err(error) = validate_secret_export_task(
        &persisted_task,
        &tenant_id,
        &election_event_id,
        &crate::types::tasks::ETasksExecution::EXPORT_ELECTION_EVENT,
        &document_id,
        export_config.contains_voter_secrets,
    ) {
        drop(hasura_transaction);
        update_export_fail(&persisted_task, &error.to_string()).await?;
        return Err(Error::String(error.to_string()));
    }

    // Process the export
    match process_export_zip(&tenant_id, &election_event_id, &document_id, export_config).await {
        Ok(_) => (),
        Err(err) => {
            let err_str = format!("Failed to export election event data: {err:?}");
            match crate::postgres::document::get_document(
                &hasura_transaction,
                &tenant_id,
                Some(election_event_id.clone()),
                &document_id,
            )
            .await
            {
                // A successful commit can be followed by a lost acknowledgement.
                // The exact task-bound document is authoritative in that case.
                Ok(Some(_)) => {
                    update_export_complete(&persisted_task, document_id.clone())
                        .await
                        .context(
                            "Failed to recover election-event export after ambiguous commit",
                        )?;
                    hasura_transaction
                        .commit()
                        .await
                        .context("Failed to release recovered election-event export task lock")?;
                    return Ok(());
                }
                Ok(None) => {
                    if let Err(update_err) = update_export_fail(&persisted_task, &err_str).await {
                        event!(
                            Level::ERROR,
                            "Failed to update task execution status to FAILED: {:?}",
                            update_err
                        );
                    }
                }
                Err(recovery_error) => {
                    return Err(Error::String(format!(
                        "{err_str}; unable to determine whether the election-event export document committed: {recovery_error}"
                    )));
                }
            }
            return Err(Error::String(err_str));
        }
    }

    update_export_complete(&persisted_task, document_id.to_string())
        .await
        .context("Failed to update task execution status to COMPLETED")?;

    hasura_transaction
        .commit()
        .await
        .context("Failed to release election-event export task lock")?;

    Ok(())
}

#[instrument(err, skip(export_config, task_execution))]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0, acks_late = true)]
pub async fn export_election_event(
    tenant_id: String,
    election_event_id: String,
    export_config: ExportOptions,
    document_id: String,
    task_execution: TasksExecution,
) -> Result<()> {
    export_election_event_impl(
        tenant_id,
        election_event_id,
        export_config,
        document_id,
        task_execution,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use celery::task::Task;

    fn authorized_secret_export() -> ExportOptions {
        ExportOptions {
            password: Some("generated-password".to_string()),
            is_encrypted: true,
            encrypt_with_password: true,
            include_voters: true,
            contains_voter_secrets: true,
            ..ExportOptions::default()
        }
    }

    #[test]
    fn accepts_authorized_password_encrypted_voter_secrets() {
        assert!(validate_voter_secret_export(&authorized_secret_export()).is_ok());
    }

    #[test]
    fn secret_report_artifacts_require_the_same_password_and_grant() {
        let mut config = authorized_secret_export();
        config.include_voters = false;
        assert!(validate_voter_secret_export(&config).is_err());
        config.s3_files = true;
        assert!(validate_voter_secret_export(&config).is_ok());
        config.password = None;
        assert!(validate_voter_secret_export(&config).is_err());
        config.password = Some("test-password".into());
        config.encrypt_with_password = false;
        assert!(validate_voter_secret_export(&config).is_err());
    }

    #[test]
    fn rejects_voter_secrets_without_password_encryption_selection() {
        let mut export_config = authorized_secret_export();
        export_config.encrypt_with_password = false;

        assert!(validate_voter_secret_export(&export_config).is_err());
    }

    #[test]
    fn accepts_ordinary_voters_without_secret_authorization() {
        let export_config = ExportOptions {
            include_voters: true,
            ..ExportOptions::default()
        };

        assert!(validate_voter_secret_export(&export_config).is_ok());
    }

    #[test]
    fn election_event_exports_ack_only_after_execution() {
        assert_eq!(export_election_event::DEFAULTS.acks_late, Some(true));
    }
}
