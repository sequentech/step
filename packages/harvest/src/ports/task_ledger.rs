// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::types::hasura::core::TasksExecution;
use windmill::types::tasks::ETasksExecution;

/// The task execution rows that let the admin portal follow a background
/// task. Each call writes on its own connection, outside the route's
/// transaction.
#[rocket::async_trait]
pub trait TaskLedger: Send + Sync {
    async fn post(
        &self,
        tenant_id: &str,
        election_event_id: Option<&str>,
        task_type: ETasksExecution,
        executed_by_user: &str,
    ) -> anyhow::Result<TasksExecution>;

    async fn update_complete(
        &self,
        task: &TasksExecution,
        document_id: Option<String>,
    ) -> anyhow::Result<()>;
    async fn update_fail(
        &self,
        task: &TasksExecution,
        err_message: &str,
    ) -> anyhow::Result<()>;
}
