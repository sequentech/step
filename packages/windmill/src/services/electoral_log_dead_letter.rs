// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Dead-letter queue for electoral-log events that cannot be stored.
//!
//! Dead-lettered messages keep the format of the event queue, so moving them back
//! to `electoral_log_event_queue` replays them; delivery IDs make a replay of an
//! already stored event store nothing new.

use crate::services::celery_app::{get_celery_connection, Queue};
use crate::tasks::electoral_log::LogEventInput;
use anyhow::{Context, Result};
use lapin::options::{BasicPublishOptions, ConfirmSelectOptions, QueueDeclareOptions};
use lapin::types::{AMQPValue, FieldTable, LongString};
use lapin::{BasicProperties, Channel};

/// Header with the reason a message was dead-lettered.
pub const ERROR_HEADER: &str = "x-electoral-log-error";
/// Header with the stage that dead-lettered a message.
pub const STAGE_HEADER: &str = "x-electoral-log-stage";
/// Name of the Celery task that event-queue messages carry.
const ENQUEUE_TASK: &str = "enqueue_electoral_log_event";
/// Longest reason kept in a header.
const MAX_REASON_CHARS: usize = 2_000;
/// AMQP delivery mode of messages that survive a broker restart.
const PERSISTENT: u8 = 2;

/// Where an event was dead-lettered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadLetterStage {
    /// The dispatcher could not parse the message.
    Dispatcher,
    /// The batch task could not build or store the record.
    Batch,
}

impl DeadLetterStage {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeadLetterStage::Dispatcher => "dispatcher",
            DeadLetterStage::Batch => "batch",
        }
    }
}

/// Publishes to the dead-letter queue, waiting for RabbitMQ to confirm each message
/// before the caller acknowledges or drops the original.
pub struct DeadLetterPublisher {
    channel: Channel,
    queue: String,
}

impl DeadLetterPublisher {
    pub async fn open() -> Result<Self> {
        let slug = std::env::var("ENV_SLUG").context("missing env var ENV_SLUG")?;
        let channel = get_celery_connection()
            .await?
            .create_channel()
            .await
            .context("Error creating a RabbitMQ channel for the dead-letter queue")?;
        channel
            .confirm_select(ConfirmSelectOptions::default())
            .await
            .context("Error enabling publisher confirms")?;
        let queue = Queue::ElectoralLogDeadLetter.queue_name(&slug);
        channel
            .queue_declare(
                &queue,
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await
            .with_context(|| format!("Error declaring {queue}"))?;
        Ok(Self { channel, queue })
    }

    /// Dead-letter an event-queue message exactly as it was received.
    pub async fn publish_raw(
        &self,
        body: &[u8],
        properties: &BasicProperties,
        stage: DeadLetterStage,
        reason: &str,
    ) -> Result<()> {
        let mut headers = properties.headers().clone().unwrap_or_default();
        add_reason(&mut headers, stage, reason);
        let properties = properties
            .clone()
            .with_headers(headers)
            .with_delivery_mode(PERSISTENT);
        self.publish(body, properties).await
    }

    /// Dead-letter an event as an event-queue message.
    pub async fn publish_event(
        &self,
        event: &LogEventInput,
        stage: DeadLetterStage,
        reason: &str,
    ) -> Result<()> {
        let body = event_message_body(event)?;
        let mut headers = FieldTable::default();
        headers.insert(
            "task".into(),
            AMQPValue::LongString(LongString::from(ENQUEUE_TASK)),
        );
        if let Some(delivery_id) = &event.delivery_id {
            headers.insert(
                "id".into(),
                AMQPValue::LongString(LongString::from(delivery_id.as_str())),
            );
        }
        add_reason(&mut headers, stage, reason);
        let properties = BasicProperties::default()
            .with_content_type("application/json".into())
            .with_content_encoding("utf-8".into())
            .with_delivery_mode(PERSISTENT)
            .with_headers(headers);
        self.publish(&body, properties).await
    }

    async fn publish(&self, body: &[u8], properties: BasicProperties) -> Result<()> {
        let confirmation = self
            .channel
            .basic_publish(
                "",
                &self.queue,
                BasicPublishOptions::default(),
                body,
                properties,
            )
            .await
            .with_context(|| format!("Error publishing to {}", self.queue))?
            .await
            .with_context(|| format!("Error waiting for {} to confirm", self.queue))?;
        anyhow::ensure!(
            confirmation.is_ack(),
            "RabbitMQ did not confirm the message dead-lettered to {}",
            self.queue
        );
        Ok(())
    }
}

/// The body of an event-queue message carrying `event`, as producers send it.
pub fn event_message_body(event: &LogEventInput) -> Result<Vec<u8>> {
    serde_json::to_vec(&serde_json::json!([
        [],
        { "input": event },
        { "callbacks": null, "errbacks": null, "chain": null, "chord": null }
    ]))
    .context("Error encoding a dead-lettered event")
}

fn add_reason(headers: &mut FieldTable, stage: DeadLetterStage, reason: &str) {
    let reason: String = reason.chars().take(MAX_REASON_CHARS).collect();
    headers.insert(
        ERROR_HEADER.into(),
        AMQPValue::LongString(LongString::from(reason.as_str())),
    );
    headers.insert(
        STAGE_HEADER.into(),
        AMQPValue::LongString(LongString::from(stage.as_str())),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reasons_are_truncated_and_tagged_with_their_stage() {
        let mut headers = FieldTable::default();
        add_reason(&mut headers, DeadLetterStage::Batch, &"é".repeat(5_000));
        let reason = headers
            .inner()
            .get(ERROR_HEADER)
            .and_then(|value| value.as_long_string())
            .unwrap();
        assert_eq!(
            String::from_utf8(reason.as_bytes().to_vec())
                .unwrap()
                .chars()
                .count(),
            MAX_REASON_CHARS
        );
        let stage = headers
            .inner()
            .get(STAGE_HEADER)
            .and_then(|value| value.as_long_string())
            .unwrap();
        assert_eq!(stage.as_bytes(), b"batch");
    }
}
