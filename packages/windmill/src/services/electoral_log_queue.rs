// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::tasks::electoral_log::LogEventInput;
use anyhow::{anyhow, ensure, Context, Result};
use async_trait::async_trait;
use lapin::{
    message::BasicGetMessage,
    options::{
        BasicAckOptions, BasicGetOptions, BasicPublishOptions, ConfirmSelectOptions,
        QueueDeclareOptions,
    },
    publisher_confirm::Confirmation,
    types::FieldTable,
    Channel,
};
use std::collections::HashMap;
use tracing::error;

/// The AMQP default exchange routes a message to the queue named by its
/// routing key.
const DEFAULT_EXCHANGE: &str = "";
/// AMQP delivery mode that makes RabbitMQ store a message on disk.
const PERSISTENT_DELIVERY_MODE: u8 = 2;

/// What became of one queued event before it is written to its board.
pub enum LogEventPreparation<M> {
    /// The event resolved to these entries on this board.
    Ready { board: String, messages: Vec<M> },
    /// The event can never be written, for example because its election
    /// event no longer exists.
    Rejected(anyhow::Error),
}

/// The deliveries bound for one board and the entries they produced.
type BoardBatch<D, M> = (Vec<D>, Vec<M>);

/// The queue the electoral log events are read from.
#[async_trait]
pub trait LogEventQueue {
    type Delivery: Send + Sync;

    fn payload(delivery: &Self::Delivery) -> &[u8];

    /// Takes the next delivery without acknowledging it.
    async fn next(&mut self) -> Result<Option<Self::Delivery>>;

    async fn ack(&mut self, delivery: &Self::Delivery) -> Result<()>;

    /// Stores a copy of a delivery that can never be written.
    async fn dead_letter(&mut self, delivery: &Self::Delivery) -> Result<()>;
}

/// Where the electoral log events are written.
#[async_trait]
pub trait LogEventStore {
    type Message: Send;

    /// Returns one preparation per event, in order. An error means the
    /// events could not be judged now and must be retried later.
    async fn prepare(
        &mut self,
        events: &[LogEventInput],
    ) -> Result<Vec<LogEventPreparation<Self::Message>>>;

    /// Writes the entries of one board and returns once they are committed.
    async fn persist(&mut self, board: &str, messages: Vec<Self::Message>) -> Result<()>;
}

/// Reads the `input` argument of an `enqueue_electoral_log_event` message.
pub fn decode_log_event(payload: &[u8]) -> Result<LogEventInput> {
    let message: serde_json::Value =
        serde_json::from_slice(payload).context("Error parsing Celery message as JSON")?;
    let input = message
        .as_array()
        .and_then(|arguments| arguments.get(1))
        .and_then(|keyword_arguments| keyword_arguments.get("input"))
        .ok_or_else(|| {
            anyhow!("Invalid message format: expected an array with an 'input' field")
        })?;
    serde_json::from_value(input.clone())
        .context("Error deserializing LogEventInput from input field")
}

/// Writes the queued events in batches until the queue is empty. A delivery is
/// acknowledged only once its entries are committed or a copy of it is in the
/// dead-letter queue. On error the remaining deliveries stay unacknowledged and
/// RabbitMQ returns them to the queue when the channel closes.
pub async fn drain_log_event_queue<Q, S>(
    queue: &mut Q,
    store: &mut S,
    batch_size: usize,
) -> Result<()>
where
    Q: LogEventQueue + Send,
    S: LogEventStore + Send,
{
    loop {
        let mut deliveries = Vec::with_capacity(batch_size);
        while deliveries.len() < batch_size {
            match queue.next().await? {
                Some(delivery) => deliveries.push(delivery),
                None => break,
            }
        }
        if deliveries.is_empty() {
            return Ok(());
        }

        let mut decoded = Vec::with_capacity(deliveries.len());
        let mut events = Vec::with_capacity(deliveries.len());
        for delivery in deliveries {
            match decode_log_event(Q::payload(&delivery)) {
                Ok(event) => {
                    decoded.push(delivery);
                    events.push(event);
                }
                Err(err) => dead_letter(queue, &delivery, err).await?,
            }
        }

        let preparations = store.prepare(&events).await?;
        ensure!(
            preparations.len() == decoded.len(),
            "Expected {} electoral log preparations, got {}",
            decoded.len(),
            preparations.len()
        );

        let mut by_board: HashMap<String, BoardBatch<Q::Delivery, S::Message>> = HashMap::new();
        for (delivery, preparation) in decoded.into_iter().zip(preparations) {
            match preparation {
                LogEventPreparation::Ready { board, messages } => {
                    let (board_deliveries, board_messages) = by_board.entry(board).or_default();
                    board_deliveries.push(delivery);
                    board_messages.extend(messages);
                }
                LogEventPreparation::Rejected(err) => dead_letter(queue, &delivery, err).await?,
            }
        }

        let mut failed_boards = Vec::new();
        for (board, (board_deliveries, messages)) in by_board {
            if let Err(err) = store.persist(&board, messages).await {
                error!(%board, error = ?err, "Error writing electoral log entries");
                failed_boards.push(board);
                continue;
            }
            for delivery in &board_deliveries {
                queue.ack(delivery).await?;
            }
        }
        ensure!(
            failed_boards.is_empty(),
            "Electoral log entries for boards {failed_boards:?} were left in the queue"
        );
    }
}

async fn dead_letter<Q>(queue: &mut Q, delivery: &Q::Delivery, reason: anyhow::Error) -> Result<()>
where
    Q: LogEventQueue + Send,
{
    error!(error = ?reason, "Moving electoral log event to the dead-letter queue");
    queue.dead_letter(delivery).await?;
    queue.ack(delivery).await
}

/// Reads electoral log events from RabbitMQ and keeps a copy of the ones that
/// can never be written in a separate durable queue.
pub struct AmqpLogEventQueue {
    channel: Channel,
    queue_name: String,
    dead_letter_queue_name: String,
}

impl AmqpLogEventQueue {
    pub async fn new(
        channel: Channel,
        queue_name: String,
        dead_letter_queue_name: String,
    ) -> Result<Self> {
        for name in [&queue_name, &dead_letter_queue_name] {
            channel
                .queue_declare(
                    name,
                    QueueDeclareOptions {
                        durable: true,
                        ..Default::default()
                    },
                    FieldTable::default(),
                )
                .await
                .with_context(|| format!("Error declaring queue {name}"))?;
        }
        channel
            .confirm_select(ConfirmSelectOptions::default())
            .await
            .context("Error enabling publisher confirms")?;
        Ok(Self {
            channel,
            queue_name,
            dead_letter_queue_name,
        })
    }
}

#[async_trait]
impl LogEventQueue for AmqpLogEventQueue {
    type Delivery = BasicGetMessage;

    fn payload(delivery: &BasicGetMessage) -> &[u8] {
        &delivery.data
    }

    async fn next(&mut self) -> Result<Option<BasicGetMessage>> {
        self.channel
            .basic_get(&self.queue_name, BasicGetOptions { no_ack: false })
            .await
            .context("Error reading from the electoral log queue")
    }

    async fn ack(&mut self, delivery: &BasicGetMessage) -> Result<()> {
        self.channel
            .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
            .await
            .context("Error acknowledging message")
    }

    async fn dead_letter(&mut self, delivery: &BasicGetMessage) -> Result<()> {
        let confirmation = self
            .channel
            .basic_publish(
                DEFAULT_EXCHANGE,
                &self.dead_letter_queue_name,
                BasicPublishOptions {
                    mandatory: true,
                    ..Default::default()
                },
                &delivery.data,
                delivery
                    .properties
                    .clone()
                    .with_delivery_mode(PERSISTENT_DELIVERY_MODE),
            )
            .await
            .context("Error publishing to the electoral log dead-letter queue")?
            .await
            .context("Error confirming the electoral log dead-letter publish")?;
        match confirmation {
            Confirmation::Ack(None) => Ok(()),
            other => Err(anyhow!(
                "RabbitMQ did not store the message in {}: {other:?}",
                self.dead_letter_queue_name
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::electoral_log::{LogEventBody, LogMessageType};
    use std::collections::{HashSet, VecDeque};

    const GONE_ELECTION_EVENT: &str = "gone";

    #[derive(Default)]
    struct FakeQueue {
        ready: VecDeque<(u64, Vec<u8>)>,
        unacked: Vec<u64>,
        acked: Vec<u64>,
        dead_lettered: Vec<Vec<u8>>,
    }

    impl FakeQueue {
        fn with(payloads: Vec<Vec<u8>>) -> Self {
            Self {
                ready: payloads
                    .into_iter()
                    .enumerate()
                    .map(|(index, payload)| (index as u64 + 1, payload))
                    .collect(),
                ..Default::default()
            }
        }
    }

    #[async_trait]
    impl LogEventQueue for FakeQueue {
        type Delivery = (u64, Vec<u8>);

        fn payload(delivery: &(u64, Vec<u8>)) -> &[u8] {
            &delivery.1
        }

        async fn next(&mut self) -> Result<Option<(u64, Vec<u8>)>> {
            let delivery = self.ready.pop_front();
            if let Some((tag, _)) = &delivery {
                self.unacked.push(*tag);
            }
            Ok(delivery)
        }

        async fn ack(&mut self, delivery: &(u64, Vec<u8>)) -> Result<()> {
            self.unacked.retain(|tag| *tag != delivery.0);
            self.acked.push(delivery.0);
            Ok(())
        }

        async fn dead_letter(&mut self, delivery: &(u64, Vec<u8>)) -> Result<()> {
            self.dead_lettered.push(delivery.1.clone());
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeStore {
        unavailable: bool,
        unavailable_boards: HashSet<String>,
        persisted: Vec<String>,
    }

    fn board_of(election_event_id: &str) -> String {
        format!("board-{election_event_id}")
    }

    #[async_trait]
    impl LogEventStore for FakeStore {
        type Message = String;

        async fn prepare(
            &mut self,
            events: &[LogEventInput],
        ) -> Result<Vec<LogEventPreparation<String>>> {
            ensure!(!self.unavailable, "database unavailable");
            Ok(events
                .iter()
                .map(|event| {
                    if event.election_event_id == GONE_ELECTION_EVENT {
                        LogEventPreparation::Rejected(anyhow!("election event not found"))
                    } else {
                        LogEventPreparation::Ready {
                            board: board_of(&event.election_event_id),
                            messages: vec![event.body.as_raw()],
                        }
                    }
                })
                .collect())
        }

        async fn persist(&mut self, board: &str, messages: Vec<String>) -> Result<()> {
            ensure!(
                !self.unavailable_boards.contains(board),
                "board {board} unavailable"
            );
            self.persisted.extend(messages);
            Ok(())
        }
    }

    fn event(election_event_id: &str, body: &str) -> Vec<u8> {
        let input = LogEventInput {
            election_event_id: election_event_id.to_string(),
            message_type: LogMessageType::KeycloakEvent("LOGIN".to_string()),
            user_id: None,
            username: None,
            tenant_id: "tenant".to_string(),
            body: LogEventBody::Plain(body.to_string()),
        };
        serde_json::to_vec(&serde_json::json!([[], { "input": input }, {}]))
            .expect("serializable test event")
    }

    #[tokio::test]
    async fn undecodable_deliveries_are_dead_lettered_and_rest_persisted() {
        let mut queue = FakeQueue::with(vec![
            event("ee1", "a"),
            b"not json".to_vec(),
            b"[[], {}]".to_vec(),
            event("ee1", "b"),
        ]);
        let mut store = FakeStore::default();

        drain_log_event_queue(&mut queue, &mut store, 10)
            .await
            .expect("drained");

        assert_eq!(store.persisted, vec!["a", "b"]);
        assert_eq!(
            queue.dead_lettered,
            vec![b"not json".to_vec(), b"[[], {}]".to_vec()]
        );
        assert!(queue.ready.is_empty());
        assert!(queue.unacked.is_empty());
        assert_eq!(queue.acked.len(), 4);
    }

    #[tokio::test]
    async fn rejected_event_is_dead_lettered_and_rest_persisted() {
        let rejected = event(GONE_ELECTION_EVENT, "b");
        let mut queue =
            FakeQueue::with(vec![event("ee1", "a"), rejected.clone(), event("ee2", "c")]);
        let mut store = FakeStore::default();

        drain_log_event_queue(&mut queue, &mut store, 10)
            .await
            .expect("drained");

        store.persisted.sort();
        assert_eq!(store.persisted, vec!["a", "c"]);
        assert_eq!(queue.dead_lettered, vec![rejected]);
        assert!(queue.unacked.is_empty());
        assert_eq!(queue.acked.len(), 3);
    }

    #[tokio::test]
    async fn unavailable_store_leaves_batch_unacknowledged() {
        let mut queue = FakeQueue::with(vec![event("ee1", "a"), event("ee2", "b")]);
        let mut store = FakeStore {
            unavailable: true,
            ..Default::default()
        };

        assert!(drain_log_event_queue(&mut queue, &mut store, 10)
            .await
            .is_err());

        assert!(store.persisted.is_empty());
        assert!(queue.acked.is_empty());
        assert!(queue.dead_lettered.is_empty());
        assert_eq!(queue.unacked, vec![1, 2]);
    }

    #[tokio::test]
    async fn failed_board_write_leaves_only_its_events_unacknowledged() {
        let mut queue = FakeQueue::with(vec![
            event("ee1", "a"),
            event("ee2", "b"),
            event("ee1", "c"),
        ]);
        let mut store = FakeStore {
            unavailable_boards: HashSet::from([board_of("ee1")]),
            ..Default::default()
        };

        assert!(drain_log_event_queue(&mut queue, &mut store, 10)
            .await
            .is_err());

        assert_eq!(store.persisted, vec!["b"]);
        assert_eq!(queue.acked, vec![2]);
        assert_eq!(queue.unacked, vec![1, 3]);
        assert!(queue.dead_lettered.is_empty());
    }

    #[tokio::test]
    async fn queue_is_drained_in_batches() {
        let mut queue = FakeQueue::with(vec![
            event("ee1", "a"),
            event("ee1", "b"),
            event("ee1", "c"),
        ]);
        let mut store = FakeStore::default();

        drain_log_event_queue(&mut queue, &mut store, 2)
            .await
            .expect("drained");

        assert_eq!(store.persisted, vec!["a", "b", "c"]);
        assert_eq!(queue.acked, vec![1, 2, 3]);
        assert!(queue.ready.is_empty());
    }
}
