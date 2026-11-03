// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::user_tasks::*;
use crate::services::access::{
    create_permission, import_users_privileges, read_permission, UserScope,
};
use crate::services::audit;
use rocket::http::Status;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use sequent_core::types::permissions::Permissions;
use sequent_core::types::voter_import::FileIntegrityPolicy;
use tracing::info;
use windmill::services::electoral_log::ElectoralLogAdminContext;
use windmill::services::export::export_users::{ExportBody, ExportUsersBody};
use windmill::services::participation_authorization::{
    ParticipationAuthorization, ParticipationCommand,
    PARTICIPATION_AUTHORIZATION,
};
use windmill::tasks::export_users::ExportUsersOutput;
use windmill::tasks::import_users::{ImportUsersBody, ImportUsersOutput};

pub async fn import_users_with(
    ledger: &impl UserTaskLedger,
    authorization: &impl UserTaskAuthorization,
    dispatch: &impl UserTaskDispatch,
    claims: JwtClaims,
    input: ImportUsersBody,
) -> UserTaskResult<ImportUsersOutput> {
    FileIntegrityPolicy::OPTIONAL
        .validate_declared_sha256(input.sha256.as_deref())
        .map_err(|error| (Status::BadRequest, error))?;
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

    authorization.authorize(
        &claims,
        input.tenant_id.clone(),
        vec![required_perm],
    )?;
    // Only authorized requests create a task. Orphan IN_PROGRESS rows would
    // block readiness even though no worker could ever mutate the census.
    let task_execution = ledger
        .create_import(&tenant_id, &election_event_id, &executer_name)
        .await
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!("Failed to insert task execution record: {error:?}"),
            )
        })?;

    let connection = dispatch.connect().await;

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
    task_input.audit_actor = Some(audit::actor(&claims));
    task_input.privileges = import_users_privileges(|permissions| {
        authorization
            .authorize(&claims, input.tenant_id.clone(), permissions)
            .is_ok()
    });

    let sent = match connection {
        Ok(connection) => {
            dispatch
                .import(&connection, task_input, task_execution.clone())
                .await
        }
        Err(failure) => Err(failure),
    };
    match sent {
        Ok(()) => {}
        Err(error) => {
            ledger.fail(
                &task_execution,
                &format!("Failed to enqueue user import: {}", error.debug),
            ).await.map_err(|update_error| (
                Status::InternalServerError,
                format!("Failed to mark unqueued user import failed: {update_error:?}"),
            ))?;
            let mut failed_task = task_execution.clone();
            failed_task.execution_status =
                TasksExecutionStatus::FAILED.to_string();
            return Ok(ImportUsersOutput {
                task_execution: failed_task,
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

    let permitted_elections = match body.election_event_id.as_deref() {
        Some(event_id) => {
            let ids = authorization
                .participation_elections(&claims, event_id)
                .await?;
            if body
                .election_id
                .as_ref()
                .is_some_and(|id| !ids.contains(id))
            {
                return Err((
                    Status::Forbidden,
                    "Election outside the user's permission labels".into(),
                ));
            }
            Some(match body.election_id.as_ref() {
                Some(id) => vec![id.clone()],
                None => ids,
            })
        }
        None => None,
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
    let task_execution = if let Some(ref election_event_id) =
        body.election_event_id
    {
        let mut annotations = documents
            .grant_annotations(&document_id, may_read_secret_attributes);
        annotations[PARTICIPATION_AUTHORIZATION] =
            serde_json::to_value(ParticipationAuthorization {
                tenant_id: tenant_id.clone(),
                election_event_id: election_event_id.clone(),
                actor_id: claims.hasura_claims.user_id.clone(),
                command: ParticipationCommand::VoterExport,
                command_id: document_id.clone(),
                election_ids: permitted_elections.clone().unwrap_or_default(),
            })
            .map_err(|error| {
                (Status::InternalServerError, error.to_string())
            })?;
        Some(
            ledger
                .create_export(
                    &tenant_id,
                    election_event_id,
                    &executer_name,
                    annotations,
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

    let sent = match dispatch.connect().await {
        Ok(connection) => {
            dispatch
                .export(
                    &connection,
                    ExportBody::Users {
                        tenant_id: body.tenant_id,
                        election_event_id: body.election_event_id.clone(),
                        election_id: body.election_id,
                        include_secret_attributes: body
                            .include_secret_attributes,
                    },
                    document_id.clone(),
                    task_execution.clone(),
                    audit::actor(&claims),
                )
                .await
        }
        Err(failure) => Err(failure),
    };
    match sent {
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
