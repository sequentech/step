// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Dead-letter queue for electoral-log events that cannot be stored.
//!
//! The queue is a PGMQ queue that no worker consumes, created with the environment's other
//! queues when its task-queue database is set up. Dead-lettered messages keep the
//! format of the event queue, so sending them back to `electoral_log_event_queue`
//! replays them; delivery IDs make a replay of an already stored event store nothing
//! new. The reason and the stage are kept in the PGMQ message headers.

use crate::services::celery_app::Queue;
use crate::services::database::get_queue_pool;
use crate::tasks::electoral_log::{enqueue_electoral_log_event, LogEventInput};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum_macros::{Display, EnumString};
use tokio_postgres::{GenericClient, Transaction};

/// Header with the reason a message was dead-lettered.
pub const ERROR_HEADER: &str = "x-electoral-log-error";
/// Header with the stage that dead-lettered a message.
pub const STAGE_HEADER: &str = "x-electoral-log-stage";
/// Longest reason kept in a header.
const MAX_REASON_CHARS: usize = 2_000;

/// The outcome recorded on a dead-lettered event that an operator discarded.
pub const DISCARDED_OUTCOME: &str = "discarded";

/// What an operator does with dead-lettered events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, EnumString)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum DeadLetterOperation {
    /// Send them back to the event queue, so that the dispatcher stores them.
    Replay,
    /// Archive them as discarded: they are never stored, and the archive keeps them.
    Discard,
}

/// Apply `operation` to the messages of `dead_letters` with these IDs, in the caller's
/// transaction. Replayed messages go to `events` unchanged. Returns how many of the IDs
/// were found.
pub async fn apply_dead_letter_operation(
    tx: &Transaction<'_>,
    dead_letters: &str,
    events: &str,
    operation: DeadLetterOperation,
    message_ids: &[i64],
) -> Result<u64> {
    pgmq_broker::validate_queue_name(dead_letters).map_err(anyhow::Error::msg)?;
    let found: Vec<i64> = match operation {
        DeadLetterOperation::Replay => {
            let rows = tx
                .query(
                    &format!(
                        "SELECT msg_id, message FROM pgmq.q_{dead_letters} \
                         WHERE msg_id = ANY($1) ORDER BY msg_id FOR UPDATE"
                    ),
                    &[&message_ids],
                )
                .await
                .context("Error reading the dead-lettered events")?;
            for row in &rows {
                let message: Value = row.get(1);
                tx.query_one("SELECT pgmq.send($1, $2)", &[&events, &message])
                    .await
                    .context("Error replaying a dead-lettered event")?;
            }
            let found: Vec<i64> = rows.iter().map(|row| row.get(0)).collect();
            tx.query(
                "SELECT pgmq.delete($1, $2::bigint[])",
                &[&dead_letters, &found],
            )
            .await
            .context("Error deleting the replayed events")?;
            found
        }
        DeadLetterOperation::Discard => {
            let found: Vec<i64> = tx
                .query(
                    &format!(
                        "UPDATE pgmq.q_{dead_letters} SET headers = COALESCE(headers, '{{}}') \
                         || jsonb_build_object($2::text, $3::text) \
                         WHERE msg_id = ANY($1) RETURNING msg_id"
                    ),
                    &[
                        &message_ids,
                        &pgmq_broker::OUTCOME_HEADER,
                        &DISCARDED_OUTCOME,
                    ],
                )
                .await
                .context("Error marking the dead-lettered events as discarded")?
                .iter()
                .map(|row| row.get(0))
                .collect();
            tx.query(
                "SELECT pgmq.archive($1, $2::bigint[])",
                &[&dead_letters, &found],
            )
            .await
            .context("Error archiving the discarded events")?;
            found
        }
    };
    Ok(found.len() as u64)
}

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
    let queue = Queue::ElectoralLogDeadLetter.queue_name();
    let mut client = get_queue_pool()
        .await
        .get()
        .await
        .context("Error getting a connection for the dead-letter queue")?;
    let tx = client
        .transaction()
        .await
        .context("Error starting the dead-letter transaction")?;
    for (event, reason) in events {
        dead_letter_message(&*tx, queue, &event_message(event)?, stage, reason).await?;
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
}

#[cfg(test)]
mod pgmq_tests {
    use super::*;
    use pgmq_broker::setup::{setup, Installation};

    /// The environment of the disposable test database, shared with pgmq-broker's tests.
    const TEST_ENVIRONMENT: &str = "pgmq-test";

    async fn count(client: &tokio_postgres::Client, table: &str) -> i64 {
        client
            .query_one(&format!("SELECT count(*) FROM pgmq.{table}"), &[])
            .await
            .unwrap()
            .get(0)
    }

    #[tokio::test]
    #[ignore = "requires PGMQ_TEST_DATABASE_URL pointing to a disposable database"]
    async fn operators_replay_or_discard_dead_lettered_events() {
        let config = deadpool_postgres::Config {
            url: Some(std::env::var("PGMQ_TEST_DATABASE_URL").expect("PGMQ_TEST_DATABASE_URL")),
            ..Default::default()
        };
        let pool = config
            .create_pool(
                Some(deadpool_postgres::Runtime::Tokio1),
                tokio_postgres::NoTls,
            )
            .unwrap();
        let id = &uuid::Uuid::new_v4().simple().to_string()[..12];
        let dead_letters = format!("operator_dead_letters_{id}");
        let events = format!("operator_events_{id}");
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        setup(
            &tx,
            &Installation {
                environment: TEST_ENVIRONMENT,
                queues: &[&dead_letters, &events],
                roles: None,
            },
        )
        .await
        .unwrap();
        for n in 1..=3 {
            dead_letter_message(
                &*tx,
                &dead_letters,
                &serde_json::json!({ "event": n }),
                DeadLetterStage::Batch,
                "reason",
            )
            .await
            .unwrap();
        }
        tx.commit().await.unwrap();

        let tx = client.transaction().await.unwrap();
        let replayed = apply_dead_letter_operation(
            &tx,
            &dead_letters,
            &events,
            DeadLetterOperation::Replay,
            &[1, 99],
        )
        .await
        .unwrap();
        let discarded = apply_dead_letter_operation(
            &tx,
            &dead_letters,
            &events,
            DeadLetterOperation::Discard,
            &[2],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        assert_eq!((replayed, discarded), (1, 1));
        assert_eq!(count(&client, &format!("q_{events}")).await, 1);
        let replayed_message: Value = client
            .query_one(&format!("SELECT message FROM pgmq.q_{events}"), &[])
            .await
            .unwrap()
            .get(0);
        assert_eq!(replayed_message, serde_json::json!({ "event": 1 }));
        assert_eq!(count(&client, &format!("q_{dead_letters}")).await, 1);
        let archived: Value = client
            .query_one(&format!("SELECT headers FROM pgmq.a_{dead_letters}"), &[])
            .await
            .unwrap()
            .get(0);
        assert_eq!(archived[pgmq_broker::OUTCOME_HEADER], DISCARDED_OUTCOME);
        assert_eq!(archived[STAGE_HEADER], "batch");
        for queue in [&dead_letters, &events] {
            client
                .query_one("SELECT pgmq.drop_queue($1)", &[queue])
                .await
                .unwrap();
        }
    }
}
