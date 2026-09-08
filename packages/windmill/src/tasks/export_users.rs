// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::document::insert_document_with_annotations;
use crate::services::database::get_hasura_pool;
use crate::services::export::export_users::{export_users_file, ExportBody};
use crate::services::tasks_execution::{
    is_matching_completed_export_task, update_export_complete, update_export_fail,
    validate_secret_export_task, validate_secret_export_task_for_recovery,
};
use crate::types::error::{Error, Result};
use anyhow::Context;
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction as _};
use sequent_core::services::s3;
use sequent_core::types::hasura::core::{DocumentAnnotations, TasksExecution};
use sequent_core::util;
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExportUsersOutput {
    pub document_id: String,
    pub error_msg: Option<String>,
    pub task_execution: Option<TasksExecution>,
}

fn event_export_scope(body: &ExportBody) -> Result<Option<(String, String, bool)>> {
    match body {
        ExportBody::Users {
            tenant_id,
            election_event_id: Some(election_event_id),
            include_secret_attributes,
            ..
        } => Ok(Some((
            tenant_id.clone(),
            election_event_id.clone(),
            *include_secret_attributes,
        ))),
        ExportBody::Users {
            election_event_id: None,
            include_secret_attributes: true,
            ..
        } => Err(Error::String(
            "Secret attributes can only be exported for election-event voters".to_string(),
        )),
        _ => Ok(None),
    }
}

async fn export_users_impl(
    body: ExportBody,
    document_id: String,
    task_execution: Option<TasksExecution>,
) -> Result<()> {
    let mut hasura_db_client: DbClient = match get_hasura_pool().await.get().await {
        Ok(client) => client,
        Err(err) => {
            return Err(Error::String(format!(
                "Error getting Hasura DB pool: {}",
                err
            )));
        }
    };

    let hasura_transaction = match hasura_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            return Err(Error::String(format!(
                "Error starting Hasura transaction: {err}"
            )));
        }
    };

    let mut coordination_transaction = Some(hasura_transaction);

    // Reload the task by its id while holding a crash-released advisory lock.
    // The serialized task and body are untrusted; all scope and authorization
    // comes from the authenticated request's durable task row.
    let persisted_task = match event_export_scope(&body)? {
        Some((tenant_id, election_event_id, requires_voter_secrets)) => {
            let broker_task = task_execution.as_ref().ok_or_else(|| {
                Error::String("Election-event voter export is missing its task execution".into())
            })?;

            let coordination = coordination_transaction
                .as_ref()
                .expect("coordination transaction must exist for event exports");
            crate::postgres::tasks_execution::lock_export_task_with_transaction(
                coordination,
                &broker_task.id,
            )
            .await
            .context("Failed to lock voter export task")?;
            let persisted_task = crate::postgres::tasks_execution::get_task_by_id_with_transaction(
                coordination,
                &tenant_id,
                &broker_task.id,
            )
            .await
            .context("Failed to load persisted voter export task")?;

            if is_matching_completed_export_task(
                &persisted_task,
                &tenant_id,
                &election_event_id,
                &crate::types::tasks::ETasksExecution::EXPORT_VOTERS,
                &document_id,
            ) {
                coordination_transaction
                    .take()
                    .expect("completed export must hold its coordination transaction")
                    .commit()
                    .await
                    .context("Failed to release completed voter export task lock")?;
                return Ok(());
            }

            let authorization_result = validate_secret_export_task_for_recovery(
                &persisted_task,
                &tenant_id,
                &election_event_id,
                &crate::types::tasks::ETasksExecution::EXPORT_VOTERS,
                &document_id,
                requires_voter_secrets,
            );
            if let Err(error) = authorization_result {
                drop(coordination_transaction.take());
                update_export_fail(&persisted_task, &error.to_string()).await?;
                return Err(Error::String(error.to_string()));
            }

            // If a prior authorized attempt committed the document and crashed
            // before its status update, repair the task instead of exporting a
            // second copy. Expiry does not invalidate an already-created result.
            let existing_document = match crate::postgres::document::get_document(
                coordination,
                &tenant_id,
                Some(election_event_id.clone()),
                &document_id,
            )
            .await
            {
                Ok(document) => document.is_some(),
                Err(error) => {
                    drop(coordination_transaction.take());
                    let error =
                        error.context("Failed to check for an existing voter export document");
                    update_export_fail(&persisted_task, &error.to_string()).await?;
                    return Err(Error::String(error.to_string()));
                }
            };
            if existing_document {
                update_export_complete(&persisted_task, document_id.clone())
                    .await
                    .context("Failed to recover completed voter export task")?;
                coordination_transaction
                    .take()
                    .expect("recovered export must hold its coordination transaction")
                    .commit()
                    .await
                    .context("Failed to release recovered voter export task lock")?;
                return Ok(());
            }

            // Starting or repeating the actual decryption still requires a
            // live grant; only the completed-document recovery above ignores
            // expiry.
            if let Err(error) = validate_secret_export_task(
                &persisted_task,
                &tenant_id,
                &election_event_id,
                &crate::types::tasks::ETasksExecution::EXPORT_VOTERS,
                &document_id,
                requires_voter_secrets,
            ) {
                drop(coordination_transaction.take());
                update_export_fail(&persisted_task, &error.to_string()).await?;
                return Err(Error::String(error.to_string()));
            }
            Some(persisted_task)
        }
        None => {
            if task_execution.is_some() {
                return Err(Error::String(
                    "Non-event user export cannot use an election-event task execution".into(),
                ));
            }
            None
        }
    };

    // Event exports keep their advisory-lock transaction open while a second
    // transaction produces the document. A worker crash releases both; a
    // redelivery then either retries or repairs the committed result.
    let mut export_db_client = if let Some(task_execution) = &persisted_task {
        match get_hasura_pool().await.get().await {
            Ok(client) => Some(client),
            Err(error) => {
                drop(coordination_transaction.take());
                let error =
                    anyhow::Error::new(error).context("Failed to get voter export database client");
                update_export_fail(task_execution, &error.to_string()).await?;
                return Err(Error::String(error.to_string()));
            }
        }
    } else {
        None
    };
    let hasura_transaction = if let Some(export_db_client) = export_db_client.as_mut() {
        match export_db_client.transaction().await {
            Ok(transaction) => transaction,
            Err(error) => {
                drop(coordination_transaction.take());
                let error = anyhow::Error::new(error)
                    .context("Failed to start voter export data transaction");
                update_export_fail(
                    persisted_task
                        .as_ref()
                        .expect("event export must have its persisted task"),
                    &error.to_string(),
                )
                .await?;
                return Err(Error::String(error.to_string()));
            }
        }
    } else {
        coordination_transaction
            .take()
            .expect("non-event export must own its data transaction")
    };

    let export_result: Result<()> = async {
        let temp_path = export_users_file(&hasura_transaction, body.clone())
            .await
            .map_err(|err| Error::String(format!("Error listing users: {err:?}")))?;
        let size = temp_path.metadata()?.len();

        let (tenant_id, election_event_id) = match &body {
            ExportBody::Users {
                tenant_id,
                election_event_id,
                ..
            } => (
                tenant_id.to_string(),
                election_event_id.clone().unwrap_or_default(),
            ),
            ExportBody::TenantUsers { tenant_id } => (tenant_id.to_string(), "".to_string()),
        };
        let timestamp = util::date::timestamp()
            .map_err(|err| Error::String(format!("Error obtaining timestamp: {err}")))?;
        let name = format!("users-export-{timestamp}.csv");
        let key = s3::get_document_key(&tenant_id, Some(&election_event_id), &document_id, &name);
        let media_type = "text/csv".to_string();

        s3::upload_file_to_s3(
            key,
            false,
            s3::get_private_bucket()?,
            media_type.clone(),
            temp_path.to_string_lossy().to_string(),
            None,
            Some(name.clone()),
        )
        .await
        .map_err(|err| Error::String(format!("Error uploading file to s3: {err}")))?;

        let document_annotations = matches!(
            &body,
            ExportBody::Users {
                include_secret_attributes: true,
                ..
            }
        )
        .then(DocumentAnnotations::voter_secret_export);
        insert_document_with_annotations(
            &hasura_transaction,
            &tenant_id,
            match &body {
                ExportBody::Users {
                    election_event_id, ..
                } => election_event_id.clone(),
                ExportBody::TenantUsers { .. } => None,
            },
            &name,
            &media_type,
            size.try_into()?,
            false,
            Some(document_id.clone()),
            document_annotations.as_ref(),
        )
        .await
        .map_err(|err| Error::String(format!("Error inserting document: {err:?}")))?;

        // The resource must be durable before SUCCESS is observable. If the
        // worker dies after this commit, the advisory lock is released and a
        // retry repairs the task from the existing document.
        hasura_transaction
            .commit()
            .await
            .context("Failed to commit voter export document")?;
        Ok(())
    }
    .await;

    if let Err(error) = export_result {
        if let Some(task_execution) = &persisted_task {
            let coordination = coordination_transaction
                .as_ref()
                .expect("event export failure must hold its coordination transaction");
            match crate::postgres::document::get_document(
                coordination,
                &task_execution.tenant_id,
                task_execution.election_event_id.clone(),
                &document_id,
            )
            .await
            {
                // PostgreSQL may commit successfully even if the commit
                // acknowledgement is lost. The task-bound document proves the
                // export completed, so repair SUCCESS instead of recording a
                // false terminal failure.
                Ok(Some(_)) => {
                    update_export_complete(task_execution, document_id.clone())
                        .await
                        .context("Failed to recover voter export after ambiguous commit")?;
                    coordination_transaction
                        .take()
                        .expect("recovered export must hold its coordination transaction")
                        .commit()
                        .await
                        .context("Failed to release recovered voter export task lock")?;
                    return Ok(());
                }
                Ok(None) => {
                    drop(coordination_transaction.take());
                    update_export_fail(task_execution, &error.to_string()).await?;
                }
                Err(recovery_error) => {
                    drop(coordination_transaction.take());
                    return Err(Error::String(format!(
                        "{error}; unable to determine whether the voter export document committed: {recovery_error}"
                    )));
                }
            }
        }
        return Err(error);
    }

    if let Some(task_execution) = &persisted_task {
        update_export_complete(task_execution, document_id.clone())
            .await
            .context("Failed to update voter export task to SUCCESS")?;
    }

    if let Some(coordination_transaction) = coordination_transaction.take() {
        coordination_transaction
            .commit()
            .await
            .context("Failed to release voter export task lock")?;
    }

    Ok(())
}

#[instrument(err, skip(body, task_execution))]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0, acks_late = true)]
pub async fn export_users(
    body: ExportBody,
    document_id: String,
    task_execution: Option<TasksExecution>,
) -> Result<()> {
    export_users_impl(body, document_id, task_execution).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use celery::task::Task;

    fn voters_export(include_secret_attributes: bool) -> ExportBody {
        ExportBody::Users {
            tenant_id: "tenant".to_string(),
            election_event_id: Some("event".to_string()),
            election_id: None,
            include_secret_attributes,
        }
    }

    #[test]
    fn identifies_whether_an_event_export_requests_secrets() {
        assert_eq!(
            event_export_scope(&voters_export(true)).unwrap(),
            Some(("tenant".to_string(), "event".to_string(), true))
        );
        assert_eq!(
            event_export_scope(&voters_export(false)).unwrap(),
            Some(("tenant".to_string(), "event".to_string(), false))
        );
    }

    #[test]
    fn rejects_secret_claims_without_an_election_event() {
        let body = ExportBody::Users {
            tenant_id: "tenant".to_string(),
            election_event_id: None,
            election_id: None,
            include_secret_attributes: true,
        };

        assert!(event_export_scope(&body).is_err());
    }

    #[test]
    fn voter_exports_ack_only_after_execution() {
        assert_eq!(export_users::DEFAULTS.acks_late, Some(true));
    }
}
