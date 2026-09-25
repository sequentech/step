// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::task_queue::{QueuedTask, TaskBroker, TaskQueue};
use celery::error::CeleryError;
use celery::task::AsyncResult;
use celery::Celery;
use std::sync::Arc;
use windmill::services::celery_app::get_celery_app;

/// Windmill's process-wide Celery app, connected on first use.
pub struct CeleryTaskQueue;

#[rocket::async_trait]
impl TaskQueue for CeleryTaskQueue {
    async fn connect(&self) -> Arc<dyn TaskBroker> {
        get_celery_app().await
    }
}

#[rocket::async_trait]
impl TaskBroker for Celery {
    async fn send(&self, task: QueuedTask) -> Result<AsyncResult, CeleryError> {
        task.send_with(self).await
    }
}
