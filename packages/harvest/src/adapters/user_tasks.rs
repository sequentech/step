// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::user_tasks::*;
use crate::services::authorization::authorize;
use celery::Celery;
use rocket::http::Status;
use sequent_core::services::jwt::{self, JwtClaims};
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;
use windmill::services::celery_app::try_get_celery_app;
use windmill::services::electoral_log::{
    post_voter_secret_attribute_audit, ElectoralLogAdminContext,
    VoterSecretAttributeAction, VoterSecretAttributeAudit,
};
use windmill::services::export::export_users::ExportBody;
use windmill::services::tasks_execution::{
    post, post_with_annotations, secret_export_task_annotations, update_fail,
};
use windmill::tasks::{export_users, import_users};
use windmill::types::tasks::ETasksExecution;

pub struct WindmillUserTaskLedger;

impl UserTaskLedger for WindmillUserTaskLedger {
    async fn create_import(
        &self,
        tenant_id: &str,
        event_id: &str,
        executed_by: &str,
    ) -> anyhow::Result<TasksExecution> {
        post(
            tenant_id,
            Some(event_id),
            ETasksExecution::IMPORT_USERS,
            executed_by,
        )
        .await
    }
    async fn create_export(
        &self,
        tenant_id: &str,
        event_id: &str,
        executed_by: &str,
        annotations: Value,
    ) -> anyhow::Result<TasksExecution> {
        post_with_annotations(
            tenant_id,
            Some(event_id),
            ETasksExecution::EXPORT_VOTERS,
            executed_by,
            annotations,
        )
        .await
    }
    async fn fail(
        &self,
        task: &TasksExecution,
        message: &str,
    ) -> anyhow::Result<()> {
        update_fail(task, message).await
    }
}

pub struct ClaimsUserTaskAuthorization;

impl UserTaskAuthorization for ClaimsUserTaskAuthorization {
    fn authorize(
        &self,
        claims: &JwtClaims,
        tenant_id: String,
        permissions: Vec<Permissions>,
    ) -> UserTaskResult<()> {
        authorize(claims, true, Some(tenant_id), permissions)
    }
}

pub struct CeleryUserTaskDispatch;

impl UserTaskDispatch for CeleryUserTaskDispatch {
    type Connection = Arc<Celery>;
    async fn connect(&self) -> Result<Self::Connection, UserTaskSendFailure> {
        try_get_celery_app()
            .await
            .map_err(|error| UserTaskSendFailure {
                display: format!("{error:#}"),
                debug: format!("{error:?}"),
            })
    }
    async fn import(
        &self,
        connection: &Self::Connection,
        input: import_users::ImportUsersBody,
        task: TasksExecution,
    ) -> Result<(), UserTaskSendFailure> {
        connection
            .send_task(import_users::import_users::new(input, task))
            .await
            .map(|_| ())
            .map_err(|error| UserTaskSendFailure {
                display: error.to_string(),
                debug: format!("{error:?}"),
            })
    }
    async fn export(
        &self,
        connection: &Self::Connection,
        input: ExportBody,
        document_id: String,
        task: Option<TasksExecution>,
    ) -> Result<(), UserTaskSendFailure> {
        connection
            .send_task(export_users::export_users::new(
                input,
                document_id,
                task,
            ))
            .await
            .map(|_| ())
            .map_err(|error| UserTaskSendFailure {
                display: error.to_string(),
                debug: format!("{error:?}"),
            })
    }
}

pub struct AuditedUserExportDocuments;

impl UserExportDocuments for AuditedUserExportDocuments {
    fn new_document_id(&self) -> String {
        Uuid::new_v4().to_string()
    }
    fn grant_annotations(
        &self,
        document_id: &str,
        may_read_secrets: bool,
    ) -> Value {
        secret_export_task_annotations(document_id, may_read_secrets)
    }
    async fn audit_secrets(
        &self,
        claims: &JwtClaims,
        tenant_id: &str,
        event_id: &str,
        document_id: &str,
    ) -> UserTaskResult<()> {
        audit_secret_attributes(
            claims,
            tenant_id,
            event_id,
            VoterSecretAttributeAction::Export,
            VoterSecretAttributeAudit {
                voter_id: None,
                voter_username: None,
                attribute_names: &[],
                document_id: Some(document_id),
            },
        )
        .await
    }
}

/// Records a secret-attribute action before it takes effect, so an action
/// that cannot be audited does not happen.
pub(crate) async fn audit_secret_attributes(
    claims: &jwt::JwtClaims,
    tenant_id: &str,
    election_event_id: &str,
    action: VoterSecretAttributeAction,
    audit: VoterSecretAttributeAudit<'_>,
) -> UserTaskResult<()> {
    post_voter_secret_attribute_audit(
        tenant_id,
        election_event_id,
        &ElectoralLogAdminContext::from_claims(claims),
        action,
        audit,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Failed to record the secret-attribute electoral-log entry: {error:#}"),
        )
    })
}

/// Persists a post-processing register-read quorum before dispatching export.
/// Ordinary preparation exports use the existing task-bound permission grant.
pub struct BoardAuthorizedUserTaskDispatch {
    pub claims: JwtClaims,
    pub request_id: Option<Uuid>,
}
impl UserTaskDispatch for BoardAuthorizedUserTaskDispatch {
    type Connection = Arc<Celery>;
    async fn connect(&self) -> Result<Self::Connection, UserTaskSendFailure> {
        CeleryUserTaskDispatch.connect().await
    }
    async fn import(
        &self,
        connection: &Self::Connection,
        input: import_users::ImportUsersBody,
        task: TasksExecution,
    ) -> Result<(), UserTaskSendFailure> {
        if let (Some(request), Some(event)) =
            (self.request_id, input.election_event_id.as_deref())
        {
            crate::services::board_authorization::bind_operation(
                &self.claims,
                event,
                request,
                windmill::services::board_operations::BindingKind::Import,
                &task.id,
            )
            .await
            .map_err(|(_, message)| UserTaskSendFailure {
                display: message.clone(),
                debug: message,
            })?;
        }
        CeleryUserTaskDispatch.import(connection, input, task).await
    }
    async fn export(
        &self,
        connection: &Self::Connection,
        input: ExportBody,
        document_id: String,
        task: Option<TasksExecution>,
    ) -> Result<(), UserTaskSendFailure> {
        if let (
            Some(request),
            ExportBody::Users {
                election_event_id: Some(event),
                ..
            },
        ) = (self.request_id, &input)
        {
            crate::services::board_authorization::bind_operation(
                &self.claims,
                event,
                request,
                windmill::services::board_operations::BindingKind::VoterExport,
                &document_id,
            )
            .await
            .map_err(|(_, message)| UserTaskSendFailure {
                display: message.clone(),
                debug: message,
            })?;
        }
        CeleryUserTaskDispatch
            .export(connection, input, document_id, task)
            .await
    }
}
