// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::user_tasks::*;
use crate::services::access::{create_permission, read_permission, UserScope};
use rocket::http::Status;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use tracing::info;
use windmill::services::electoral_log::ElectoralLogAdminContext;
use windmill::services::export::export_users::{ExportBody, ExportUsersBody};
use windmill::tasks::export_users::ExportUsersOutput;
use windmill::tasks::import_users::{ImportUsersBody, ImportUsersOutput};

pub async fn import_users_with(
    ledger: &impl UserTaskLedger,
    authorization: &impl UserTaskAuthorization,
    dispatch: &impl UserTaskDispatch,
    claims: JwtClaims,
    input: ImportUsersBody,
) -> UserTaskResult<ImportUsersOutput> {
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let election_event_id = input.election_event_id.clone().unwrap_or_default();
    let is_admin = election_event_id.is_empty();
    info!("Calculated is_admin: {}", is_admin);

    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());
    let required_perm =
        create_permission(UserScope::of(input.election_event_id.as_deref()));

    // Insert the task execution record
    let task_execution = ledger
        .create_import(&tenant_id, &election_event_id, &executer_name)
        .await
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!("Failed to insert task execution record: {error:?}"),
            )
        })?;

    authorization.authorize(
        &claims,
        input.tenant_id.clone(),
        vec![required_perm],
    )?;
    let celery_app = dispatch.connect().await;

    let mut task_input = input.clone();
    task_input.is_admin = is_admin;
    task_input.may_write_secret_attributes = input.election_event_id.is_some()
        && authorization
            .authorize(
                &claims,
                input.tenant_id.clone(),
                vec![Permissions::VOTER_SECRET_ATTRIBUTE_WRITE],
            )
            .is_ok();
    task_input.secret_write_initiator = task_input
        .may_write_secret_attributes
        .then(|| ElectoralLogAdminContext::from_claims(&claims));

    match dispatch
        .import(&celery_app, task_input, task_execution.clone())
        .await
    {
        Ok(()) => {}
        Err(_) => {
            return Ok(ImportUsersOutput {
                task_execution: task_execution.clone(),
            });
        }
    };

    info!("Sent IMPORT_USERS task {}", task_execution.id);

    let output = ImportUsersOutput {
        task_execution: task_execution.clone(),
    };

    Ok(output)
}

pub async fn export_users_with(
    ledger: &impl UserTaskLedger,
    authorization: &impl UserTaskAuthorization,
    dispatch: &impl UserTaskDispatch,
    documents: &impl UserExportDocuments,
    claims: JwtClaims,
    body: ExportUsersBody,
) -> UserTaskResult<ExportUsersOutput> {
    let tenant_id = body.tenant_id.clone();
    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    let required_perm =
        read_permission(UserScope::of(body.election_event_id.as_deref()));

    authorization.authorize(
        &claims,
        body.tenant_id.clone(),
        vec![required_perm],
    )?;

    let may_read_secret_attributes = if body.include_secret_attributes {
        if body.election_event_id.is_none() {
            return Err((
                Status::BadRequest,
                "Secret attributes can only be included in an election-event voter export"
                    .to_string(),
            ));
        }
        authorization.authorize(
            &claims,
            body.tenant_id.clone(),
            vec![Permissions::VOTER_SECRET_ATTRIBUTE_READ],
        )?;
        true
    } else {
        false
    };

    let document_id = documents.new_document_id();
    if let (true, Some(election_event_id)) = (
        may_read_secret_attributes,
        body.election_event_id.as_deref(),
    ) {
        documents
            .audit_secrets(
                &claims,
                &body.tenant_id,
                election_event_id,
                &document_id,
            )
            .await?;
    }

    // Authorize before creating the task row, then persist a task-bound grant.
    // The worker reloads this row and never trusts a broker-supplied boolean.
    let task_execution =
        if let Some(ref election_event_id) = body.election_event_id {
            Some(
                ledger
                    .create_export(
                        &tenant_id,
                        election_event_id,
                        &executer_name,
                        documents.grant_annotations(
                            &document_id,
                            may_read_secret_attributes,
                        ),
                    )
                    .await
                    .map_err(|error| {
                        (
                            Status::InternalServerError,
                            format!(
                            "Failed to insert task execution record: {error:?}"
                        ),
                        )
                    })?,
            )
        } else {
            None
        };

    let celery_app = dispatch.connect().await;

    match dispatch
        .export(
            &celery_app,
            ExportBody::Users {
                tenant_id: body.tenant_id,
                election_event_id: body.election_event_id.clone(),
                election_id: body.election_id,
                include_secret_attributes: body.include_secret_attributes,
            },
            document_id.clone(),
            task_execution.clone(),
        )
        .await
    {
        Ok(()) => {}
        Err(err) => {
            if let Some(task_execution) = &task_execution {
                ledger.fail(
                    task_execution,
                    &format!("Failed to enqueue voter export: {}", err.debug),
                )
                .await
                .map_err(|update_error| {
                    (
                        Status::InternalServerError,
                        format!(
                            "Failed to revoke voter export authorization: {update_error:?}"
                        ),
                    )
                })?;
            }
            return Ok(ExportUsersOutput {
                document_id,
                error_msg: Some(format!(
                    "Error sending Export Users task: ${}",
                    err.display
                )),
                task_execution: task_execution.clone(),
            });
        }
    };

    let output = ExportUsersOutput {
        document_id,
        error_msg: None,
        task_execution: task_execution.clone(),
    };

    info!("Sent EXPORT_USERS task");

    Ok(output)
}

#[cfg(test)]
#[path = "../../tests/support/user_task_orchestration.rs"]
mod tests;
