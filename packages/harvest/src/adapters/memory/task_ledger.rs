// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::task_ledger::TaskLedger;
use anyhow::anyhow;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use serde_json::json;
use std::sync::Mutex;
use windmill::types::tasks::ETasksExecution;

/// Which writes a ledger refuses, as when its database goes down.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Refusal {
    #[default]
    Nothing,
    Everything,
    /// Rows can be created but not updated afterwards.
    Updates,
}

/// Task execution rows in memory.
#[derive(Default)]
pub struct MemoryTaskLedger {
    tasks: Mutex<Vec<TasksExecution>>,
    refuses: Refusal,
}

impl MemoryTaskLedger {
    pub fn refusing() -> Self {
        Self {
            refuses: Refusal::Everything,
            ..Default::default()
        }
    }

    pub fn refusing_updates() -> Self {
        Self {
            refuses: Refusal::Updates,
            ..Default::default()
        }
    }

    pub fn tasks(&self) -> Vec<TasksExecution> {
        self.tasks.lock().unwrap().clone()
    }

    fn check(&self, refused_by: Refusal) -> anyhow::Result<()> {
        match self.refuses == Refusal::Everything || self.refuses == refused_by
        {
            true => Err(anyhow!("tasks_execution is unavailable")),
            false => Ok(()),
        }
    }

    fn update(
        &self,
        task: &TasksExecution,
        status: TasksExecutionStatus,
        log: &str,
        document_id: Option<String>,
    ) -> anyhow::Result<()> {
        self.check(Refusal::Updates)?;
        let mut tasks = self.tasks.lock().unwrap();
        let stored = tasks
            .iter_mut()
            .find(|stored| stored.id == task.id)
            .ok_or_else(|| anyhow!("unknown task {}", task.id))?;
        stored.execution_status = status.to_string();
        stored.logs = Some(json!([log]));
        stored.annotations = Some(json!({ "document_id": document_id }));
        Ok(())
    }
}

#[rocket::async_trait]
impl TaskLedger for MemoryTaskLedger {
    async fn post(
        &self,
        tenant_id: &str,
        election_event_id: Option<&str>,
        task_type: ETasksExecution,
        executed_by_user: &str,
    ) -> anyhow::Result<TasksExecution> {
        self.check(Refusal::Everything)?;
        let mut tasks = self.tasks.lock().unwrap();
        let task: TasksExecution = serde_json::from_value(json!({
            "id": format!("task-{}", tasks.len() + 1), "tenant_id": tenant_id,
            "election_event_id": election_event_id,
            "name": task_type.to_name(), "task_type": task_type.to_string(),
            "execution_status": TasksExecutionStatus::IN_PROGRESS.to_string(),
            "created_at": "2026-01-01T00:00:00Z",
            "executed_by_user": executed_by_user, "annotations": null
        }))?;
        tasks.push(task.clone());
        Ok(task)
    }

    async fn update_complete(
        &self,
        task: &TasksExecution,
        document_id: Option<String>,
    ) -> anyhow::Result<()> {
        self.update(
            task,
            TasksExecutionStatus::SUCCESS,
            "Task completed successfully",
            document_id,
        )
    }

    async fn update_fail(
        &self,
        task: &TasksExecution,
        err_message: &str,
    ) -> anyhow::Result<()> {
        self.update(
            task,
            TasksExecutionStatus::FAILED,
            &format!("Error: {err_message}"),
            None,
        )
    }
}
