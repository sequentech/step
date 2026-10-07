// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Dead-letter queue for electoral-log events that cannot be stored.
//!
//! The queue is a PGMQ queue that no worker consumes. Dead-lettered messages keep the
//! format of the event queue, so sending them back to `electoral_log_event_queue`
//! replays them; delivery IDs make a replay of an already stored event store nothing
//! new. The reason and the stage are kept in the PGMQ message headers.

use crate::services::celery_app::Queue;
use crate::services::database::get_keycloak_pool;
use crate::tasks::electoral_log::{enqueue_electoral_log_event, LogEventInput};
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};
use tokio_postgres::GenericClient;

/// Header with the reason a message was dead-lettered.
pub const ERROR_HEADER: &str = "x-electoral-log-error";
/// Header with the stage that dead-lettered a message.
pub const STAGE_HEADER: &str = "x-electoral-log-stage";
/// Longest reason kept in a header.
const MAX_REASON_CHARS: usize = 2_000;

/// Dead-letter queues this process has already created.
static CREATED_QUEUES: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);

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

/// The PGMQ name of the environment's dead-letter queue.
pub fn dead_letter_queue(slug: &str) -> String {
    pgmq_broker::queue_name(&Queue::ElectoralLogDeadLetter.queue_name(slug))
}

/// Create the dead-letter queue if this process has not done so yet. Workers only
/// declare the queues they consume, and none consumes this one. Call it outside a
/// transaction, so that a rollback cannot undo a creation this process remembers.
pub async fn ensure_dead_letter_queue(client: &tokio_postgres::Client, queue: &str) -> Result<()> {
    let created = |queues: &Mutex<HashSet<String>>| {
        queues
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains(queue)
    };
    if created(&CREATED_QUEUES) {
        return Ok(());
    }
    client
        .query_one("SELECT pgmq.create($1)", &[&queue])
        .await
        .with_context(|| format!("Error creating the dead-letter queue {queue}"))?;
    CREATED_QUEUES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(queue.to_owned());
    Ok(())
}

/// Dead-letter a message of the event queue exactly as it was stored, on the caller's
/// connection or transaction.
pub async fn dead_letter_message<C: GenericClient + Sync>(
    client: &C,
    queue: &str,
    message: &Value,
    stage: DeadLetterStage,
    reason: &str,
) -> Result<()> {
    client
        .query_one(
            "SELECT pgmq.send($1::text, $2::jsonb, $3::jsonb)",
            &[&queue, message, &dead_letter_headers(stage, reason)],
        )
        .await
        .with_context(|| format!("Error dead-lettering to {queue}"))?;
    Ok(())
}

/// Dead-letter events as event-queue messages, in one transaction.
pub async fn dead_letter_events(
    events: &[(&LogEventInput, &str)],
    stage: DeadLetterStage,
) -> Result<()> {
    let slug = std::env::var("ENV_SLUG").context("missing env var ENV_SLUG")?;
    let queue = dead_letter_queue(&slug);
    let mut client = get_keycloak_pool()
        .await
        .get()
        .await
        .context("Error getting a connection for the dead-letter queue")?;
    ensure_dead_letter_queue(&client, &queue).await?;
    let tx = client
        .transaction()
        .await
        .context("Error starting the dead-letter transaction")?;
    for (event, reason) in events {
        dead_letter_message(&*tx, &queue, &event_message(event)?, stage, reason).await?;
    }
    tx.commit()
        .await
        .context("Error committing the dead-lettered events")
}

/// The event-queue message carrying `event`, as producers send it. Its Celery ID is the
/// event's delivery ID.
pub fn event_message(event: &LogEventInput) -> Result<Value> {
    let mut message =
        celery::protocol::Message::try_from(enqueue_electoral_log_event::new(event.clone()))
            .context("Error encoding a dead-lettered event")?;
    if let Some(delivery_id) = &event.delivery_id {
        message.headers.id = delivery_id.clone();
    }
    let encoded = message
        .json_serialized(None)
        .context("Error encoding a dead-lettered event")?;
    serde_json::from_slice(&encoded).context("Error encoding a dead-lettered event")
}

fn dead_letter_headers(stage: DeadLetterStage, reason: &str) -> Value {
    let mut headers = serde_json::Map::new();
    headers.insert(
        ERROR_HEADER.into(),
        Value::String(reason.chars().take(MAX_REASON_CHARS).collect()),
    );
    headers.insert(STAGE_HEADER.into(), Value::String(stage.as_str().into()));
    Value::Object(headers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reasons_are_truncated_and_tagged_with_their_stage() {
        let headers = dead_letter_headers(DeadLetterStage::Batch, &"é".repeat(5_000));
        assert_eq!(
            headers[ERROR_HEADER].as_str().unwrap().chars().count(),
            MAX_REASON_CHARS
        );
        assert_eq!(headers[STAGE_HEADER], "batch");
    }

    #[test]
    fn dead_letter_queue_names_differ_by_environment() {
        assert_ne!(dead_letter_queue("dev"), dead_letter_queue("prod"));
        assert!(dead_letter_queue("dev").starts_with("step_"));
    }
}
