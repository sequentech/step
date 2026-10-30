// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Delivery of the signed CastVote of an accepted vote. The event is stored
//! in `sequent_backend.electoral_log_outbox` in the transaction that inserts
//! the vote, so an accepted vote always has it. It is then published to the
//! electoral log event queue under the outbox row's ID and the row is
//! deleted; rows left behind, for example because RabbitMQ was unavailable
//! or the process stopped, are published again by
//! `relay_electoral_log_outbox`. The ID is the message's correlation ID, so
//! the dispatcher's receipts make publishing an event twice harmless.

use crate::services::celery_app::{get_celery_connection, Queue};
use crate::services::database::get_hasura_pool;
use crate::tasks::electoral_log::{enqueue_electoral_log_event, LogEventInput};
use anyhow::{anyhow, Context, Result};
use celery::task::Task;
use deadpool_postgres::{Client as DbClient, Transaction};
use lapin::{
    options::{BasicPublishOptions, ConfirmSelectOptions, QueueDeclareOptions},
    publisher_confirm::Confirmation,
    types::{AMQPValue, FieldTable, ShortString},
    BasicProperties, Connection,
};
use std::time::Duration;
use tracing::{error, info};
use uuid::Uuid;

/// AMQP delivery mode that makes RabbitMQ store a message on disk.
const PERSISTENT_DELIVERY_MODE: u8 = 2;
/// Events younger than this are left to the request that stored them.
pub const OUTBOX_RELAY_DELAY: Duration = Duration::from_secs(60);
/// Most events one relay run publishes.
pub const OUTBOX_RELAY_BATCH: i64 = 500;

/// An event stored in the outbox, waiting to be published.
#[derive(Debug, Clone)]
pub struct PendingLogEvent {
    pub id: Uuid,
    pub input: LogEventInput,
}

/// Stores an event in the outbox, in the caller's transaction.
pub async fn store_log_event(
    transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    input: LogEventInput,
) -> Result<PendingLogEvent> {
    let row = transaction
        .query_one(
            "INSERT INTO sequent_backend.electoral_log_outbox \
             (tenant_id, election_event_id, input) VALUES ($1, $2, $3) RETURNING id",
            &[tenant_id, election_event_id, &serde_json::to_value(&input)?],
        )
        .await
        .context("Error storing the electoral log event in the outbox")?;
    Ok(PendingLogEvent {
        id: row.try_get(0)?,
        input,
    })
}

/// The `enqueue_electoral_log_event` Celery message body of an event.
pub fn celery_body(input: &LogEventInput) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(&serde_json::json!([
        [],
        { "input": input },
        { "callbacks": null, "errbacks": null, "chain": null, "chord": null }
    ]))?)
}

/// Publishes an event to the electoral log event queue and returns once
/// RabbitMQ has stored it.
pub async fn publish_log_event(
    connection: &Connection,
    queue: &str,
    event: &PendingLogEvent,
) -> Result<()> {
    let channel = connection
        .create_channel()
        .await
        .context("Error creating RabbitMQ channel")?;
    channel
        .queue_declare(
            queue,
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .with_context(|| format!("Error declaring queue {queue}"))?;
    channel
        .confirm_select(ConfirmSelectOptions::default())
        .await
        .context("Error enabling publisher confirms")?;
    let id = event.id.to_string();
    let mut headers = FieldTable::default();
    headers.insert(
        ShortString::from("id"),
        AMQPValue::LongString(id.clone().into()),
    );
    headers.insert(
        ShortString::from("task"),
        AMQPValue::LongString(enqueue_electoral_log_event::NAME.into()),
    );
    let confirmation = channel
        .basic_publish(
            "",
            queue,
            BasicPublishOptions {
                mandatory: true,
                ..Default::default()
            },
            &celery_body(&event.input)?,
            BasicProperties::default()
                .with_correlation_id(id.into())
                .with_content_type("application/json".into())
                .with_content_encoding("utf-8".into())
                .with_delivery_mode(PERSISTENT_DELIVERY_MODE)
                .with_headers(headers),
        )
        .await
        .context("Error publishing the electoral log event")?
        .await
        .context("Error confirming the electoral log event publish")?;
    match confirmation {
        Confirmation::Ack(None) => Ok(()),
        other => Err(anyhow!(
            "RabbitMQ did not store the event in {queue}: {other:?}"
        )),
    }
}

/// Removes a published event from the outbox, unless the relay holds it, in
/// which case the relay removes it.
pub async fn remove_published(client: &DbClient, id: &Uuid) -> Result<()> {
    client
        .execute(
            "DELETE FROM sequent_backend.electoral_log_outbox WHERE id = \
             (SELECT id FROM sequent_backend.electoral_log_outbox \
              WHERE id = $1 FOR UPDATE SKIP LOCKED)",
            &[id],
        )
        .await
        .context("Error removing a published electoral log event from the outbox")?;
    Ok(())
}

fn event_queue() -> Result<String> {
    let slug = std::env::var("ENV_SLUG").context("missing env var ENV_SLUG")?;
    Ok(Queue::ElectoralLogEvent.queue_name(&slug))
}

/// Publishes an event stored by an accepted vote, after its commit. On error
/// the event stays in the outbox for the relay.
pub async fn deliver_log_event(event: &PendingLogEvent) {
    let delivered = async {
        let connection = get_celery_connection().await?;
        publish_log_event(&connection, &event_queue()?, event).await?;
        let client = get_hasura_pool()
            .await
            .get()
            .await
            .context("Error getting DB pool")?;
        remove_published(&client, &event.id).await
    }
    .await;
    if let Err(err) = delivered {
        error!(
            outbox_id = %event.id,
            error = ?err,
            "Electoral log event left in the outbox for the relay"
        );
    }
}

/// Publishes the outbox events older than `older_than`, oldest first, and
/// removes them. Rows are locked while they are published, so concurrent
/// relays take different events. Returns how many were published.
pub async fn relay_log_events(
    client: &mut DbClient,
    connection: &Connection,
    queue: &str,
    older_than: Duration,
    limit: i64,
) -> Result<usize> {
    let transaction = client.transaction().await?;
    let rows = transaction
        .query(
            "SELECT id, input FROM sequent_backend.electoral_log_outbox \
             WHERE created_at < now() - make_interval(secs => $1) \
             ORDER BY created_at LIMIT $2 FOR UPDATE SKIP LOCKED",
            &[&older_than.as_secs_f64(), &limit],
        )
        .await
        .context("Error reading the electoral log outbox")?;
    let mut published = Vec::with_capacity(rows.len());
    let mut failure = None;
    for row in rows {
        let event = PendingLogEvent {
            id: row.try_get(0)?,
            input: serde_json::from_value(row.try_get(1)?)
                .context("Error reading an electoral log outbox event")?,
        };
        if let Err(err) = publish_log_event(connection, queue, &event).await {
            failure = Some(err);
            break;
        }
        published.push(event.id);
    }
    transaction
        .execute(
            "DELETE FROM sequent_backend.electoral_log_outbox WHERE id = ANY($1)",
            &[&published],
        )
        .await
        .context("Error removing published electoral log events from the outbox")?;
    transaction.commit().await?;
    if !published.is_empty() {
        info!(
            "Relayed {} electoral log events from the outbox",
            published.len()
        );
    }
    match failure {
        Some(err) => Err(err.context(format!(
            "Relayed {} electoral log events before a publish failed",
            published.len()
        ))),
        None => Ok(published.len()),
    }
}

/// Runs the relay against the configured database and broker.
pub async fn relay_configured_log_events() -> Result<usize> {
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .context("Error getting DB pool")?;
    let connection = get_celery_connection().await?;
    relay_log_events(
        &mut client,
        &connection,
        &event_queue()?,
        OUTBOX_RELAY_DELAY,
        OUTBOX_RELAY_BATCH,
    )
    .await
}
