// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use rocket::http::Status;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde_json::Value;
use std::future::Future;
use windmill::services::export::export_users::ExportBody;
use windmill::tasks::import_users::ImportUsersBody;

pub type UserTaskResult<T> = Result<T, (Status, String)>;

pub trait UserTaskLedger: Sync {
    fn create_import(
        &self,
        tenant_id: &str,
        event_id: &str,
        executed_by: &str,
    ) -> impl Future<Output = anyhow::Result<TasksExecution>> + Send;
    fn create_export(
        &self,
        tenant_id: &str,
        event_id: &str,
        executed_by: &str,
        annotations: Value,
    ) -> impl Future<Output = anyhow::Result<TasksExecution>> + Send;
    fn fail(
        &self,
        task: &TasksExecution,
        message: &str,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}

pub trait UserTaskAuthorization: Sync {
    fn authorize(
        &self,
        claims: &JwtClaims,
        tenant_id: String,
        permissions: Vec<Permissions>,
    ) -> UserTaskResult<()>;
}

/// Both representations are used by the existing export response and ledger.
pub struct UserTaskSendFailure {
    pub display: String,
    pub debug: String,
}

pub trait UserTaskDispatch: Sync {
    type Connection: Send + Sync;
    fn connect(&self) -> impl Future<Output = Self::Connection> + Send;
    fn import(
        &self,
        connection: &Self::Connection,
        input: ImportUsersBody,
        task: TasksExecution,
    ) -> impl Future<Output = Result<(), UserTaskSendFailure>> + Send;
    fn export(
        &self,
        connection: &Self::Connection,
        input: ExportBody,
        document_id: String,
        task: Option<TasksExecution>,
    ) -> impl Future<Output = Result<(), UserTaskSendFailure>> + Send;
}

pub trait UserExportDocuments: Sync {
    fn new_document_id(&self) -> String;
    fn grant_annotations(
        &self,
        document_id: &str,
        may_read_secrets: bool,
    ) -> Value;
    fn audit_secrets(
        &self,
        claims: &JwtClaims,
        tenant_id: &str,
        event_id: &str,
        document_id: &str,
    ) -> impl Future<Output = UserTaskResult<()>> + Send;
}
