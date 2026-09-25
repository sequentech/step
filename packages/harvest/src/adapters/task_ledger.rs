// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::task_ledger::TaskLedger;
use sequent_core::types::hasura::core::TasksExecution;
use windmill::services::tasks_execution;
use windmill::types::tasks::ETasksExecution;

/// Windmill's task execution rows, written through the process-wide pool.
pub struct WindmillTaskLedger;

#[rocket::async_trait]
impl TaskLedger for WindmillTaskLedger {
    async fn post(
        &self,
        tenant_id: &str,
        election_event_id: Option<&str>,
        task_type: ETasksExecution,
        executed_by_user: &str,
    ) -> anyhow::Result<TasksExecution> {
        tasks_execution::post(
            tenant_id,
            election_event_id,
            task_type,
            executed_by_user,
        )
        .await
    }



    async fn update_complete(
        &self,
        task: &TasksExecution,
        document_id: Option<String>,
    ) -> anyhow::Result<()> {
        tasks_execution::update_complete(task, document_id).await
    }

    async fn update_fail(
        &self,
        task: &TasksExecution,
        err_message: &str,
    ) -> anyhow::Result<()> {
        tasks_execution::update_fail(task, err_message).await
    }
}
