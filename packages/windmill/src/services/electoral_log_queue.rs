// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{ensure, Result};
use lapin::{
    message::BasicGetMessage,
    options::{BasicAckOptions, BasicGetOptions, QueueDeclareOptions},
    types::FieldTable,
    Channel,
};
use std::future::Future;

/// Own the channel so cancellation closes it and RabbitMQ requeues every
/// unacknowledged delivery. The sink must tolerate a commit whose ACK is lost.
pub async fn drain_electoral_log_queue<F, Fut>(
    channel: Channel,
    queue_name: &str,
    batch_size: usize,
    mut persist: F,
) -> Result<()>
where
    F: FnMut(Vec<BasicGetMessage>) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    let result = async {
        ensure!(batch_size > 0, "electoral log batch size must be positive");
        channel
            .queue_declare(
                queue_name,
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await?;
        loop {
            let mut deliveries = Vec::with_capacity(batch_size);
            for _ in 0..batch_size {
                match channel
                    .basic_get(queue_name, BasicGetOptions { no_ack: false })
                    .await?
                {
                    Some(delivery) => deliveries.push(delivery),
                    None => break,
                }
            }
            if deliveries.is_empty() {
                return Ok(());
            }
            let tags: Vec<_> = deliveries
                .iter()
                .map(|delivery| delivery.delivery_tag)
                .collect();
            // Only the confirmed sink outcome releases the original messages.
            // An error, timeout or lost ACK leaves receipts to make replay safe.
            persist(deliveries).await?;
            for tag in tags {
                channel.basic_ack(tag, BasicAckOptions::default()).await?;
            }
        }
    }
    .await;
    if let Err(error) = channel.close(200, "electoral log batch complete").await {
        tracing::warn!(?error, "Error closing electoral log delivery channel");
    }
    result
}
