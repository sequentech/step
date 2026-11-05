// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What runs a `Deferred` action once its last signature arrives.
//!
//! The approve step calls the action's executor in its own transaction,
//! inside a savepoint, with the request and every approval. An executor
//! that fails leaves nothing behind (the savepoint is rolled back) and the
//! request fails with its signatures kept.
//!
//! **Contract.** [`SigningExecutor::execute`] has no side effect outside the
//! database transaction it is given: a rollback must undo everything it
//! did. An [`ExecutionOutcome::Executed`] outcome may only leave
//! [`PostCommit`] work that is safe to repeat or lose (a notification, a
//! log kick). Anything else, such as calling another service, sending a
//! signed package or writing to the board, takes the dispatched path:
//! `execute` writes the `tasks_execution` row and answers
//! [`ExecutionOutcome::Dispatched`]; the task is sent after the commit,
//! claims the execution with
//! [`crate::services::signing::requests::claim_dispatched`] before its side
//! effect, and reports with
//! [`crate::services::signing::requests::finish_dispatched`]. A copy of the
//! task that can't claim does nothing. The sweeper sends the task again
//! while the execution is unclaimed, its claim is older than a lease, or
//! its task failed, at most a few times.

use crate::postgres::signing::{SigningApprovalRow, SigningRequestRow};
use anyhow::Result;
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::signing::SigningAction;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use uuid::Uuid;

/// Something to do after the transaction commits.
#[async_trait]
pub trait PostCommitTask: Send + Sync {
    async fn send(&self) -> Result<()>;
}

/// Work an execution leaves for after the commit.
pub enum PostCommit {
    SendTask(Box<dyn PostCommitTask>),
}

impl fmt::Debug for PostCommit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PostCommit::SendTask(_) => write!(f, "PostCommit::SendTask"),
        }
    }
}

impl PostCommit {
    /// Runs it; a failure is left to the sweeper.
    pub async fn run(self) -> Result<()> {
        match self {
            PostCommit::SendTask(task) => task.send().await,
        }
    }
}

#[derive(Debug)]
pub enum ExecutionOutcome {
    /// The action ran in the transaction; `result` is kept on the request.
    Executed {
        result: Option<Value>,
        post_commit: Vec<PostCommit>,
    },
    /// The task with this `tasks_execution` id runs the action after the
    /// commit; it claims the execution first and reports back.
    Dispatched {
        task_execution_id: Uuid,
        task: PostCommit,
    },
}

#[async_trait]
pub trait SigningExecutor: Send + Sync {
    fn action(&self) -> SigningAction;

    /// Runs the action of a request that has every signature it needs; see
    /// the module documentation for what it may do.
    async fn execute(
        &self,
        hasura_transaction: &Transaction<'_>,
        request: &SigningRequestRow,
        approvals: &[SigningApprovalRow],
    ) -> Result<ExecutionOutcome>;

    /// The task to send again for a dispatched request that has not
    /// reported back. `None` for executors that run in the transaction.
    async fn redispatch(&self, _request: &SigningRequestRow) -> Result<Option<PostCommit>> {
        Ok(None)
    }
}

/// The executor of each action.
#[derive(Clone, Default)]
pub struct SigningExecutorRegistry {
    executors: HashMap<SigningAction, Arc<dyn SigningExecutor>>,
}

impl fmt::Debug for SigningExecutorRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut actions: Vec<String> = self.executors.keys().map(|a| a.to_string()).collect();
        actions.sort();
        f.debug_struct("SigningExecutorRegistry")
            .field("actions", &actions)
            .finish()
    }
}

impl SigningExecutorRegistry {
    /// Adds `executor` for its action, replacing an earlier one.
    pub fn with(mut self, executor: Arc<dyn SigningExecutor>) -> Self {
        self.executors.insert(executor.action(), executor);
        self
    }

    pub fn get(&self, action: SigningAction) -> Option<Arc<dyn SigningExecutor>> {
        self.executors.get(&action).cloned()
    }
}

/// The executors of the product's actions.
pub fn default_registry() -> SigningExecutorRegistry {
    super::actions::executors(Arc::new(super::actions::CeleryDispatcher))
        .into_iter()
        .fold(SigningExecutorRegistry::default(), |registry, executor| {
            registry.with(executor)
        })
}
