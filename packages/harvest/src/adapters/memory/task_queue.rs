// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::task_queue::{QueuedTask, TaskBroker, TaskQueue};
use anyhow::anyhow;
use celery::error::CeleryError;
use celery::task::AsyncResult;
use serde_json::Value;
use std::sync::{Arc, Mutex};

/// A task as the worker would receive it.
#[derive(Clone, Debug, PartialEq)]
pub struct SentTask {
    pub name: String,
    pub kwargs: Value,
}

/// Records the tasks handlers send.
#[derive(Clone, Default)]
pub struct MemoryTaskQueue(Arc<Broker>);

/// How the broker answers.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Availability {
    #[default]
    Accepting,
    /// Connects, then rejects every task, as a database that fails sends.
    Refusing,
    /// Cannot be connected to, as an unreachable task-queue database.
    Unreachable,
}

#[derive(Default)]
struct Broker {
    sent: Mutex<Vec<SentTask>>,
    availability: Availability,
}

impl MemoryTaskQueue {
    fn answering(availability: Availability) -> Self {
        Self(Arc::new(Broker {
            availability,
            ..Default::default()
        }))
    }

    pub fn refusing() -> Self {
        Self::answering(Availability::Refusing)
    }

    pub fn unreachable() -> Self {
        Self::answering(Availability::Unreachable)
    }

    pub fn sent(&self) -> Vec<SentTask> {
        self.0.sent.lock().unwrap().clone()
    }
}

#[rocket::async_trait]
impl TaskQueue for MemoryTaskQueue {
    async fn connect(&self) -> anyhow::Result<Arc<dyn TaskBroker>> {
        if self.0.availability == Availability::Unreachable {
            return Err(anyhow!(
                "Timed out connecting to the task-queue database"
            ));
        }
        let broker: Arc<dyn TaskBroker> = self.0.clone();
        Ok(broker)
    }
}

#[rocket::async_trait]
impl TaskBroker for Broker {
    async fn send(&self, task: QueuedTask) -> Result<AsyncResult, CeleryError> {
        if self.availability == Availability::Refusing {
            return Err(CeleryError::ForcedShutdown);
        }
        let message = task.into_message()?;
        // Celery's JSON body is [args, kwargs, embed].
        let (_, kwargs, _): (Value, Value, Value) =
            serde_json::from_slice(&message.raw_body)
                .expect("a Celery JSON message body");
        self.sent.lock().unwrap().push(SentTask {
            name: message.headers.task.clone(),
            kwargs,
        });
        Ok(AsyncResult::new(&message.headers.id))
    }
}
