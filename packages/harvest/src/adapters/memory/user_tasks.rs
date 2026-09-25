// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::user_tasks::*;
use anyhow::anyhow;
use sequent_core::services::authorization::authorize_with;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use windmill::services::export::export_users::ExportBody;
use windmill::tasks::import_users::ImportUsersBody;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UserTaskCall {
    CreateImport,
    CreateExport,
    Fail,
    SendImport,
    SendExport,
    Audit,
}

#[derive(Clone, Default)]
pub struct UserTasksState {
    pub tasks: Vec<TasksExecution>,
    pub failed_tasks: Vec<(String, String)>,
    pub import_attempts: Vec<(ImportUsersBody, TasksExecution)>,
    pub export_attempts: Vec<(ExportBody, String, Option<TasksExecution>)>,
    pub audits: Vec<(String, String, String, String)>,
    pub document_count: usize,
    pub connection_count: usize,
    failures: HashMap<UserTaskCall, String>,
}

impl UserTasksState {
    fn check(&self, call: UserTaskCall) -> anyhow::Result<()> {
        match self.failures.get(&call) {
            Some(message) => Err(anyhow!("{message}")),
            None => Ok(()),
        }
    }
    fn check_send(
        &self,
        call: UserTaskCall,
    ) -> Result<(), UserTaskSendFailure> {
        match self.failures.get(&call) {
            Some(message) => Err(UserTaskSendFailure {
                display: message.clone(),
                debug: format!("BrokerFailure({message})"),
            }),
            None => Ok(()),
        }
    }
    fn insert(
        &mut self,
        tenant_id: &str,
        event_id: &str,
        executed_by: &str,
        task_type: &str,
        name: &str,
        annotations: Option<Value>,
    ) -> TasksExecution {
        let task: TasksExecution = serde_json::from_value(json!({
            "id": format!("task-{}", self.tasks.len() + 1), "tenant_id": tenant_id,
            "election_event_id": event_id, "name": name, "task_type": task_type,
            "execution_status": "IN_PROGRESS", "created_at": "2026-01-01T00:00:00Z",
            "executed_by_user": executed_by, "annotations": annotations
        })).unwrap();
        self.tasks.push(task.clone());
        task
    }
}

#[derive(Default)]
pub struct InMemoryUserTasks(Mutex<UserTasksState>);

impl InMemoryUserTasks {
    fn state(&self) -> MutexGuard<'_, UserTasksState> {
        self.0.lock().expect("user tasks lock")
    }
    pub fn snapshot(&self) -> UserTasksState {
        self.state().clone()
    }
    pub fn fail_on(&self, call: UserTaskCall, message: &str) {
        self.state().failures.insert(call, message.into());
    }
}

impl UserTaskLedger for InMemoryUserTasks {
    async fn create_import(
        &self,
        tenant_id: &str,
        event_id: &str,
        executed_by: &str,
    ) -> anyhow::Result<TasksExecution> {
        let mut state = self.state();
        state.check(UserTaskCall::CreateImport)?;
        Ok(state.insert(
            tenant_id,
            event_id,
            executed_by,
            "IMPORT_USERS",
            "Import Voters",
            None,
        ))
    }
    async fn create_export(
        &self,
        tenant_id: &str,
        event_id: &str,
        executed_by: &str,
        annotations: Value,
    ) -> anyhow::Result<TasksExecution> {
        let mut state = self.state();
        state.check(UserTaskCall::CreateExport)?;
        Ok(state.insert(
            tenant_id,
            event_id,
            executed_by,
            "EXPORT_VOTERS",
            "Export Voters",
            Some(annotations),
        ))
    }
    async fn fail(
        &self,
        task: &TasksExecution,
        message: &str,
    ) -> anyhow::Result<()> {
        let mut state = self.state();
        state.check(UserTaskCall::Fail)?;
        let stored = state
            .tasks
            .iter_mut()
            .find(|stored| stored.id == task.id)
            .ok_or_else(|| anyhow!("Task missing"))?;
        stored.execution_status = "FAILED".into();
        state.failed_tasks.push((task.id.clone(), message.into()));
        Ok(())
    }
}

impl UserTaskAuthorization for InMemoryUserTasks {
    fn authorize(
        &self,
        claims: &JwtClaims,
        tenant_id: String,
        permissions: Vec<Permissions>,
    ) -> UserTaskResult<()> {
        authorize_with(claims, true, Some(tenant_id), permissions, || None)
    }
}

impl UserTaskDispatch for InMemoryUserTasks {
    type Connection = ();
    async fn connect(&self) {
        self.state().connection_count += 1;
    }
    async fn import(
        &self,
        _connection: &(),
        input: ImportUsersBody,
        task: TasksExecution,
    ) -> Result<(), UserTaskSendFailure> {
        let mut state = self.state();
        state.import_attempts.push((input, task));
        state.check_send(UserTaskCall::SendImport)
    }
    async fn export(
        &self,
        _connection: &(),
        input: ExportBody,
        document_id: String,
        task: Option<TasksExecution>,
    ) -> Result<(), UserTaskSendFailure> {
        let mut state = self.state();
        state.export_attempts.push((input, document_id, task));
        state.check_send(UserTaskCall::SendExport)
    }
}

impl UserExportDocuments for InMemoryUserTasks {
    fn new_document_id(&self) -> String {
        let mut state = self.state();
        state.document_count += 1;
        format!("document-{}", state.document_count)
    }
    fn grant_annotations(
        &self,
        document_id: &str,
        may_read_secrets: bool,
    ) -> Value {
        json!({"secret_export_authorization": {"document_id": document_id, "voter_secret_attributes": may_read_secrets, "expires_at": "2026-01-02T00:00:00Z"}})
    }
    async fn audit_secrets(
        &self,
        claims: &JwtClaims,
        tenant_id: &str,
        event_id: &str,
        document_id: &str,
    ) -> UserTaskResult<()> {
        let mut state = self.state();
        state.check(UserTaskCall::Audit).map_err(|error| (rocket::http::Status::InternalServerError, format!("Failed to record the secret-attribute electoral-log entry: {error:#}")))?;
        state.audits.push((
            tenant_id.into(),
            event_id.into(),
            document_id.into(),
            claims.hasura_claims.user_id.clone(),
        ));
        Ok(())
    }
}
