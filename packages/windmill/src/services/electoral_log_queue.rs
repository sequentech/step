// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::tasks::electoral_log::LogEventInput;
use anyhow::{anyhow, ensure, Context, Result};
use async_trait::async_trait;
use electoral_log::client::board_client::DeliveryReceipt;
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
use sha2::{Digest, Sha256};
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

/// An event with the identity that makes writing it again harmless.
#[derive(Debug, Clone)]
pub struct IdentifiedLogEvent {
    pub delivery_id: String,
    pub payload_hash: String,
    pub input: LogEventInput,
}

/// The entries of one event, bound for one board under its identity.
pub struct BoardDelivery<M> {
    pub delivery_id: String,
    pub payload_hash: String,
    pub messages: Vec<M>,
}

/// What became of one event after a write attempt.
#[derive(Debug)]
pub enum EventOutcome {
    /// Its entries are on the board, now or from an earlier delivery.
    Stored,
    /// It can never be written.
    Rejected(anyhow::Error),
    /// It could not be written now and must be retried.
    Retained,
}

/// The queue the electoral log events are read from.
#[async_trait]
pub trait LogEventQueue {
    type Delivery: Send + Sync;

    fn payload(delivery: &Self::Delivery) -> &[u8];

    /// The ID the publisher gave the message, kept across redeliveries.
    fn correlation_id(delivery: &Self::Delivery) -> Option<&str>;

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

    /// Writes the deliveries of one board with their receipts and returns,
    /// once they are committed, one receipt per delivery in order.
    async fn persist(
        &mut self,
        board: &str,
        deliveries: Vec<BoardDelivery<Self::Message>>,
    ) -> Result<Vec<DeliveryReceipt>>;
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Gives each event of a message a stable identity. Events of a batch queued
/// by an earlier version are numbered within it, in a namespace of their own.
pub fn identify_log_events(
    correlation_id: &str,
    legacy_batch: bool,
    events: Vec<LogEventInput>,
) -> Result<Vec<IdentifiedLogEvent>> {
    ensure!(
        !correlation_id.is_empty(),
        "electoral log delivery has no stable correlation ID"
    );
    events
        .into_iter()
        .enumerate()
        .map(|(index, input)| {
            let identity = if legacy_batch {
                format!("batch:{correlation_id}:{index}")
            } else {
                format!("event:{correlation_id}")
            };
            Ok(IdentifiedLogEvent {
                delivery_id: sha256_hex(identity.as_bytes()),
                payload_hash: sha256_hex(&serde_json::to_vec(&input)?),
                input,
            })
        })
        .collect()
}

/// Reads the events of an `enqueue_electoral_log_event` message, or of a
/// `process_electoral_log_events_batch` message queued by an earlier version.
pub fn decode_log_events(
    payload: &[u8],
    correlation_id: Option<&str>,
) -> Result<Vec<IdentifiedLogEvent>> {
    let message: serde_json::Value =
        serde_json::from_slice(payload).context("Error parsing Celery message as JSON")?;
    let arguments = message
        .as_array()
        .and_then(|arguments| arguments.get(1))
        .ok_or_else(|| anyhow!("Invalid message format: expected a Celery arguments array"))?;
    let (legacy_batch, events) = if let Some(input) = arguments.get("input") {
        let input: LogEventInput = serde_json::from_value(input.clone())
            .context("Error deserializing LogEventInput from input field")?;
        (false, vec![input])
    } else if let Some(events) = arguments.get("events") {
        let events: Vec<LogEventInput> = serde_json::from_value(events.clone())
            .context("Error deserializing LogEventInput list from events field")?;
        (true, events)
    } else {
        return Err(anyhow!(
            "Invalid message format: expected an 'input' or 'events' field"
        ));
    };
    let correlation_id = correlation_id
        .ok_or_else(|| anyhow!("electoral log delivery has no stable correlation ID"))?;
    identify_log_events(correlation_id, legacy_batch, events)
}

/// Writes the events to their boards and returns one outcome per event, in
/// order. A board whose write fails leaves all of its events retained.
pub async fn store_log_events<S>(
    store: &mut S,
    events: Vec<IdentifiedLogEvent>,
) -> Result<Vec<EventOutcome>>
where
    S: LogEventStore + Send,
{
    let inputs: Vec<LogEventInput> = events.iter().map(|event| event.input.clone()).collect();
    let preparations = store.prepare(&inputs).await?;
    ensure!(
        preparations.len() == events.len(),
        "Expected {} electoral log preparations, got {}",
        events.len(),
        preparations.len()
    );

    let mut outcomes: Vec<Option<EventOutcome>> = Vec::with_capacity(events.len());
    let mut by_board: HashMap<String, (Vec<usize>, Vec<BoardDelivery<S::Message>>)> =
        HashMap::new();
    // Copies of one message read in the same batch share its write, so a
    // board transaction never holds one receipt twice.
    let mut first_copies: HashMap<String, (usize, String)> = HashMap::new();
    let mut copies: Vec<(usize, usize)> = Vec::new();
    for (index, (event, preparation)) in events.into_iter().zip(preparations).enumerate() {
        match preparation {
            LogEventPreparation::Ready { board, messages } => {
                if let Some((first, payload_hash)) = first_copies.get(&event.delivery_id) {
                    outcomes.push(Some(if *payload_hash == event.payload_hash {
                        copies.push((index, *first));
                        EventOutcome::Retained
                    } else {
                        EventOutcome::Rejected(anyhow!(
                            "Electoral log delivery {} was read twice with different inputs",
                            event.delivery_id
                        ))
                    }));
                    continue;
                }
                first_copies.insert(
                    event.delivery_id.clone(),
                    (index, event.payload_hash.clone()),
                );
                let (indexes, deliveries) = by_board.entry(board).or_default();
                indexes.push(index);
                deliveries.push(BoardDelivery {
                    delivery_id: event.delivery_id,
                    payload_hash: event.payload_hash,
                    messages,
                });
                outcomes.push(None);
            }
            LogEventPreparation::Rejected(err) => outcomes.push(Some(EventOutcome::Rejected(err))),
        }
    }

    for (board, (indexes, deliveries)) in by_board {
        let delivery_ids: Vec<String> = deliveries
            .iter()
            .map(|delivery| delivery.delivery_id.clone())
            .collect();
        let receipts = match store.persist(&board, deliveries).await {
            Ok(receipts) if receipts.len() == indexes.len() => receipts,
            Ok(receipts) => {
                error!(
                    %board,
                    "Expected {} electoral log receipts, got {}",
                    indexes.len(),
                    receipts.len()
                );
                vec![]
            }
            Err(err) => {
                error!(%board, error = ?err, "Error writing electoral log entries");
                vec![]
            }
        };
        if receipts.is_empty() {
            for index in indexes {
                outcomes[index] = Some(EventOutcome::Retained);
            }
            continue;
        }
        for ((index, receipt), delivery_id) in indexes.into_iter().zip(receipts).zip(delivery_ids)
        {
            outcomes[index] = Some(match receipt {
                DeliveryReceipt::Stored | DeliveryReceipt::AlreadyStored => EventOutcome::Stored,
                DeliveryReceipt::Conflicting => EventOutcome::Rejected(anyhow!(
                    "Electoral log delivery {delivery_id} was already stored with a different input"
                )),
            });
        }
    }

    for (copy, first) in copies {
        outcomes[copy] = Some(match &outcomes[first] {
            Some(EventOutcome::Stored) => EventOutcome::Stored,
            Some(EventOutcome::Rejected(err)) => EventOutcome::Rejected(anyhow!("{err:#}")),
            _ => EventOutcome::Retained,
        });
    }

    outcomes
        .into_iter()
        .map(|outcome| outcome.ok_or_else(|| anyhow!("Electoral log event has no outcome")))
        .collect()
}

/// Writes the queued events in batches until the queue is empty. A delivery is
/// acknowledged only once all of its events are committed or a copy of it is
/// in the dead-letter queue. A delivery with a retained event stays
/// unacknowledged and RabbitMQ returns it to the queue when the channel
/// closes; receipts make writing its other events again harmless.
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
        let mut events = Vec::new();
        let mut owners = Vec::new();
        for delivery in deliveries {
            match decode_log_events(Q::payload(&delivery), Q::correlation_id(&delivery)) {
                Ok(delivery_events) => {
                    owners.extend(std::iter::repeat(decoded.len()).take(delivery_events.len()));
                    events.extend(delivery_events);
                    decoded.push(delivery);
                }
                Err(err) => dead_letter(queue, &delivery, err).await?,
            }
        }

        let outcomes = store_log_events(store, events).await?;
        let mut retained = vec![false; decoded.len()];
        let mut rejections: Vec<Vec<anyhow::Error>> = decoded.iter().map(|_| Vec::new()).collect();
        for (owner, outcome) in owners.into_iter().zip(outcomes) {
            match outcome {
                EventOutcome::Stored => {}
                EventOutcome::Retained => retained[owner] = true,
                EventOutcome::Rejected(err) => rejections[owner].push(err),
            }
        }

        let mut retained_count = 0;
        for ((delivery, retained), rejections) in decoded.iter().zip(retained).zip(rejections) {
            if retained {
                retained_count += 1;
            } else if rejections.is_empty() {
                queue.ack(delivery).await?;
            } else {
                let reasons = rejections
                    .iter()
                    .map(|err| format!("{err:#}"))
                    .collect::<Vec<_>>()
                    .join("; ");
                dead_letter(queue, delivery, anyhow!(reasons)).await?;
            }
        }
        ensure!(
            retained_count == 0,
            "{retained_count} electoral log deliveries were left in the queue"
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

    fn correlation_id(delivery: &BasicGetMessage) -> Option<&str> {
        delivery
            .properties
            .correlation_id()
            .as_ref()
            .map(|id| id.as_str())
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

    type FakeDelivery = (u64, Option<String>, Vec<u8>);

    #[derive(Default)]
    struct FakeQueue {
        ready: VecDeque<FakeDelivery>,
        unacked: Vec<FakeDelivery>,
        acked: Vec<u64>,
        dead_lettered: Vec<Vec<u8>>,
        fail_acks: bool,
    }

    impl FakeQueue {
        fn with(payloads: Vec<Vec<u8>>) -> Self {
            Self {
                ready: payloads
                    .into_iter()
                    .enumerate()
                    .map(|(index, payload)| {
                        let tag = index as u64 + 1;
                        (tag, Some(format!("task-{tag}")), payload)
                    })
                    .collect(),
                ..Default::default()
            }
        }

        /// What RabbitMQ does when the channel closes: unacknowledged
        /// deliveries go back to the queue with their properties.
        fn requeue_unacked(&mut self) {
            for delivery in self.unacked.drain(..).rev() {
                self.ready.push_front(delivery);
            }
        }

        fn unacked_tags(&self) -> Vec<u64> {
            self.unacked.iter().map(|delivery| delivery.0).collect()
        }
    }

    #[async_trait]
    impl LogEventQueue for FakeQueue {
        type Delivery = FakeDelivery;

        fn payload(delivery: &FakeDelivery) -> &[u8] {
            &delivery.2
        }

        fn correlation_id(delivery: &FakeDelivery) -> Option<&str> {
            delivery.1.as_deref()
        }

        async fn next(&mut self) -> Result<Option<FakeDelivery>> {
            let delivery = self.ready.pop_front();
            if let Some(delivery) = &delivery {
                self.unacked.push(delivery.clone());
            }
            Ok(delivery)
        }

        async fn ack(&mut self, delivery: &FakeDelivery) -> Result<()> {
            ensure!(!self.fail_acks, "channel closed");
            self.unacked.retain(|unacked| unacked.0 != delivery.0);
            self.acked.push(delivery.0);
            Ok(())
        }

        async fn dead_letter(&mut self, delivery: &FakeDelivery) -> Result<()> {
            self.dead_lettered.push(delivery.2.clone());
            Ok(())
        }
    }

    /// Commits a board's deliveries with receipts, as a board transaction does.
    #[derive(Default)]
    struct FakeStore {
        unavailable: bool,
        unavailable_boards: HashSet<String>,
        receipts: HashMap<String, String>,
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

        async fn persist(
            &mut self,
            board: &str,
            deliveries: Vec<BoardDelivery<String>>,
        ) -> Result<Vec<DeliveryReceipt>> {
            ensure!(
                !self.unavailable_boards.contains(board),
                "board {board} unavailable"
            );
            Ok(deliveries
                .into_iter()
                .map(
                    |delivery| match self.receipts.get(&delivery.delivery_id) {
                        Some(hash) if *hash == delivery.payload_hash => {
                            DeliveryReceipt::AlreadyStored
                        }
                        Some(_) => DeliveryReceipt::Conflicting,
                        None => {
                            self.receipts
                                .insert(delivery.delivery_id, delivery.payload_hash);
                            self.persisted.extend(delivery.messages);
                            DeliveryReceipt::Stored
                        }
                    },
                )
                .collect())
        }
    }

    fn input(election_event_id: &str, body: &str) -> LogEventInput {
        LogEventInput {
            election_event_id: election_event_id.to_string(),
            message_type: LogMessageType::KeycloakEvent("LOGIN".to_string()),
            user_id: None,
            username: None,
            tenant_id: "tenant".to_string(),
            body: LogEventBody::Plain(body.to_string()),
        }
    }

    fn event(election_event_id: &str, body: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!([
            [],
            { "input": input(election_event_id, body) },
            {}
        ]))
        .expect("serializable test event")
    }

    fn legacy_batch(events: &[(&str, &str)]) -> Vec<u8> {
        let events: Vec<LogEventInput> = events
            .iter()
            .map(|(election_event_id, body)| input(election_event_id, body))
            .collect();
        serde_json::to_vec(&serde_json::json!([[], { "events": events }, {}]))
            .expect("serializable test batch")
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
        assert_eq!(queue.unacked_tags(), vec![1, 2]);
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
        assert_eq!(queue.unacked_tags(), vec![1, 3]);
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

    #[tokio::test]
    async fn redelivery_after_commit_does_not_write_again() {
        let mut queue = FakeQueue::with(vec![event("ee1", "a"), event("ee2", "b")]);
        let mut store = FakeStore::default();

        // The board commits, then the channel closes before the ack.
        queue.fail_acks = true;
        assert!(drain_log_event_queue(&mut queue, &mut store, 10)
            .await
            .is_err());
        queue.requeue_unacked();
        queue.fail_acks = false;

        drain_log_event_queue(&mut queue, &mut store, 10)
            .await
            .expect("drained");

        store.persisted.sort();
        assert_eq!(store.persisted, vec!["a", "b"]);
        queue.acked.sort();
        assert_eq!(queue.acked, vec![1, 2]);
        assert!(queue.dead_lettered.is_empty());
    }

    #[tokio::test]
    async fn legacy_batch_is_acknowledged_only_once_every_board_is_stored() {
        let mut queue = FakeQueue::with(vec![legacy_batch(&[
            ("ee1", "a"),
            ("ee2", "b"),
            ("ee1", "c"),
        ])]);
        let mut store = FakeStore {
            unavailable_boards: HashSet::from([board_of("ee2")]),
            ..Default::default()
        };

        assert!(drain_log_event_queue(&mut queue, &mut store, 10)
            .await
            .is_err());
        assert!(queue.acked.is_empty());
        assert_eq!(queue.unacked_tags(), vec![1]);

        queue.requeue_unacked();
        store.unavailable_boards.clear();
        drain_log_event_queue(&mut queue, &mut store, 10)
            .await
            .expect("drained");

        store.persisted.sort();
        assert_eq!(store.persisted, vec!["a", "b", "c"]);
        assert_eq!(queue.acked, vec![1]);
        assert!(queue.dead_lettered.is_empty());
    }

    #[tokio::test]
    async fn legacy_batch_with_a_rejected_event_stores_the_rest_and_keeps_a_copy() {
        let batch = legacy_batch(&[("ee1", "a"), (GONE_ELECTION_EVENT, "b")]);
        let mut queue = FakeQueue::with(vec![batch.clone()]);
        let mut store = FakeStore::default();

        drain_log_event_queue(&mut queue, &mut store, 10)
            .await
            .expect("drained");

        assert_eq!(store.persisted, vec!["a"]);
        assert_eq!(queue.dead_lettered, vec![batch]);
        assert_eq!(queue.acked, vec![1]);
    }

    #[tokio::test]
    async fn delivery_without_correlation_id_is_dead_lettered() {
        let payload = event("ee1", "a");
        let mut queue = FakeQueue::default();
        queue.ready.push_back((1, None, payload.clone()));
        let mut store = FakeStore::default();

        drain_log_event_queue(&mut queue, &mut store, 10)
            .await
            .expect("drained");

        assert!(store.persisted.is_empty());
        assert_eq!(queue.dead_lettered, vec![payload]);
        assert_eq!(queue.acked, vec![1]);
    }

    #[tokio::test]
    async fn reused_delivery_id_with_other_input_is_dead_lettered() {
        let mut queue = FakeQueue::default();
        queue
            .ready
            .push_back((1, Some("same".into()), event("ee1", "a")));
        let changed = event("ee1", "changed");
        queue
            .ready
            .push_back((2, Some("same".into()), changed.clone()));
        let mut store = FakeStore::default();

        drain_log_event_queue(&mut queue, &mut store, 1)
            .await
            .expect("drained");

        assert_eq!(store.persisted, vec!["a"]);
        assert_eq!(queue.dead_lettered, vec![changed]);
        assert_eq!(queue.acked, vec![1, 2]);
    }

    #[tokio::test]
    async fn copies_of_one_message_in_a_batch_are_written_once() {
        let payload = event("ee1", "a");
        let mut queue = FakeQueue::default();
        queue.ready.push_back((1, Some("same".into()), payload.clone()));
        queue.ready.push_back((2, Some("same".into()), payload));
        let mut store = FakeStore::default();

        drain_log_event_queue(&mut queue, &mut store, 10)
            .await
            .expect("drained");

        assert_eq!(store.persisted, vec!["a"]);
        assert_eq!(queue.acked, vec![1, 2]);
        assert!(queue.dead_lettered.is_empty());
    }

    #[test]
    fn delivery_ids_are_stable_and_namespaced() {
        let events = vec![input("ee1", "a"), input("ee1", "a")];
        let first = identify_log_events("id", true, events.clone()).expect("identified");
        let again = identify_log_events("id", true, events.clone()).expect("identified");
        let single = identify_log_events("id", false, vec![events[0].clone()]).expect("identified");

        assert_eq!(first[0].delivery_id, again[0].delivery_id);
        assert_ne!(first[0].delivery_id, first[1].delivery_id);
        assert_eq!(first[0].payload_hash, first[1].payload_hash);
        assert_ne!(first[0].delivery_id, single[0].delivery_id);
        assert!(identify_log_events("", false, events).is_err());
    }
}
