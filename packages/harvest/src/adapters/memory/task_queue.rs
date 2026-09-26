// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::task_queue::{QueuedTask, TaskBroker, TaskQueue};
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

/// Records the tasks handlers send. A refusing broker rejects every task,
/// as when RabbitMQ is unreachable after connecting.
#[derive(Clone, Default)]
pub struct MemoryTaskQueue(Arc<Broker>);

#[derive(Default)]
struct Broker {
    sent: Mutex<Vec<SentTask>>,
    refuses: bool,
}

impl MemoryTaskQueue {
    pub fn refusing() -> Self {
        Self(Arc::new(Broker {
            refuses: true,
            ..Default::default()
        }))
    }

    pub fn sent(&self) -> Vec<SentTask> {
        self.0.sent.lock().unwrap().clone()
    }
}

#[rocket::async_trait]
impl TaskQueue for MemoryTaskQueue {
    async fn connect(&self) -> Arc<dyn TaskBroker> {
        self.0.clone()
    }
}

#[rocket::async_trait]
impl TaskBroker for Broker {
    async fn send(&self, task: QueuedTask) -> Result<AsyncResult, CeleryError> {
        if self.refuses {
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
