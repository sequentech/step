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
    pub may_read_voter_secrets: bool,
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
    let is_authorized_password_export = export_config.include_voters
        && export_config.encrypt_with_password
        && export_config.is_encrypted
        && export_config.may_read_voter_secrets
        && has_password;

    if !is_authorized_password_export {
        return Err(Error::String(
            "Election-event voter secret export requires password encryption and voter-secret-attribute-read authorization"
                .to_string(),
        ));
    }

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_election_event(
    tenant_id: String,
    election_event_id: String,
    export_config: ExportOptions,
    document_id: String,
    task_execution: TasksExecution,
) -> Result<()> {
    if let Err(error) = validate_voter_secret_export(&export_config) {
        if let Err(update_error) = update_fail(&task_execution, &error.to_string()).await {
            event!(
                Level::ERROR,
                "Failed to update task execution status to FAILED: {:?}",
                update_error
            );
        }
        return Err(error);
    }

    let mut hasura_db_client: DbClient = match get_hasura_pool().await.get().await {
        Ok(client) => client,
        Err(err) => {
            let err_str = format!("Failed to get Hasura DB pool: {err:?}");
            if let Err(err) = update_fail(&task_execution, &err_str).await {
                event!(
                    Level::ERROR,
                    "Failed to update task execution status to FAILED: {:?}",
                    err
                );
            }
            return Err(Error::String(err_str));
        }
    };

    let hasura_transaction = match hasura_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            let err_str = format!("Failed to start Hasura transaction: {err:?}");
            if let Err(err) = update_fail(&task_execution, &err_str).await {
                event!(
                    Level::ERROR,
                    "Failed to update task execution status to FAILED: {:?}",
                    err
                );
            }
            return Err(Error::String(err_str));
        }
    };

    // Process the export
    match process_export_zip(&tenant_id, &election_event_id, &document_id, export_config).await {
        Ok(_) => (),
        Err(err) => {
            let err_str = format!("Failed to export election event data: {err:?}");
            if let Err(update_err) = update_fail(&task_execution, &err_str).await {
                event!(
                    Level::ERROR,
                    "Failed to update task execution status to FAILED: {:?}",
                    update_err
                );
            }
            return Err(Error::String(err_str));
        }
    }

    match hasura_transaction.commit().await {
        Ok(_) => (),
        Err(err) => {
            let err_str = format!("Commit failed: {err:?}");
            if let Err(err) = update_fail(&task_execution, &err_str).await {
                event!(
                    Level::ERROR,
                    "Failed to update task execution status to FAILED: {:?}",
                    err
                );
            }
            return Err(Error::String(err_str));
        }
    };

    update_complete(&task_execution, Some(document_id.to_string()))
        .await
        .context("Failed to update task execution status to COMPLETED")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn authorized_secret_export() -> ExportOptions {
        ExportOptions {
            password: Some("generated-password".to_string()),
            is_encrypted: true,
            encrypt_with_password: true,
            include_voters: true,
            contains_voter_secrets: true,
            may_read_voter_secrets: true,
            ..ExportOptions::default()
        }
    }

    #[test]
    fn accepts_authorized_password_encrypted_voter_secrets() {
        assert!(validate_voter_secret_export(&authorized_secret_export()).is_ok());
    }

    #[test]
    fn rejects_voter_secrets_without_password_encryption_selection() {
        let mut export_config = authorized_secret_export();
        export_config.encrypt_with_password = false;

        assert!(validate_voter_secret_export(&export_config).is_err());
    }

    #[test]
    fn rejects_voter_secrets_without_task_authorization() {
        let mut export_config = authorized_secret_export();
        export_config.may_read_voter_secrets = false;

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
}
