// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use celery::error::CeleryError;
use celery::task::{AsyncResult, Signature, Task};
use celery::Celery;
use rocket::futures::future::BoxFuture;
use std::sync::Arc;

/// The broker route handlers enqueue Windmill tasks on.
#[rocket::async_trait]
pub trait TaskQueue: Send + Sync {
    /// Handlers connect where they always have, some of them before writing
    /// anything else, so an unavailable broker stops those requests first.
    async fn connect(&self) -> Arc<dyn TaskBroker>;
}

#[rocket::async_trait]
pub trait TaskBroker: Send + Sync {
    async fn send(&self, task: QueuedTask) -> Result<AsyncResult, CeleryError>;
}

impl dyn TaskBroker {
    pub async fn send_task<T: Task + 'static>(
        &self,
        signature: Signature<T>,
    ) -> Result<AsyncResult, CeleryError> {
        self.send(QueuedTask(Box::new(signature))).await
    }
}

/// A task signature of any task type.
pub struct QueuedTask(Box<dyn AnySignature>);

impl QueuedTask {
    pub fn send_with(
        self,
        app: &Celery,
    ) -> BoxFuture<'_, Result<AsyncResult, CeleryError>> {
        self.0.send_with(app)
    }

    /// The message the broker would receive.
    #[cfg(test)]
    pub fn into_message(
        self,
    ) -> Result<celery::protocol::Message, celery::error::ProtocolError> {
        self.0.into_message()
    }
}

trait AnySignature: Send {
    fn send_with<'a>(
        self: Box<Self>,
        app: &'a Celery,
    ) -> BoxFuture<'a, Result<AsyncResult, CeleryError>>;
    #[cfg(test)]
    fn into_message(
        self: Box<Self>,
    ) -> Result<celery::protocol::Message, celery::error::ProtocolError>;
}

impl<T: Task + 'static> AnySignature for Signature<T> {
    fn send_with<'a>(
        self: Box<Self>,
        app: &'a Celery,
    ) -> BoxFuture<'a, Result<AsyncResult, CeleryError>> {
        Box::pin(app.send_task(*self))
    }

    #[cfg(test)]
    fn into_message(
        self: Box<Self>,
    ) -> Result<celery::protocol::Message, celery::error::ProtocolError> {
        celery::protocol::Message::try_from(*self)
    }
}
