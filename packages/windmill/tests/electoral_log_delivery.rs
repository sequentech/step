// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Electoral log delivery against real RabbitMQ and immudb services, with
//! synthetic boards and events only. The dispatcher's queue, drain and board
//! writes are the production ones; only the Hasura lookup that resolves an
//! event's board is replaced by a fixed board.
//!
//! Run with:
//! `ELECTORAL_LOG_TEST_AMQP_URL=amqp://… ELECTORAL_LOG_TEST_IMMUDB_URL=http://…
//! cargo test -p windmill --test electoral_log_delivery -- --include-ignored`

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use electoral_log::client::board_client::{BoardClient, DeliveryReceipt, ElectoralLogMessage};
use immudb_rs::TxMode;
use lapin::{
    message::BasicGetMessage,
    options::{BasicPublishOptions, ConfirmSelectOptions, QueueDeclareOptions},
    types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties,
};
use windmill::services::electoral_log_queue::{
    drain_log_event_queue, identify_log_events, AmqpLogEventQueue, BoardDelivery,
    LogEventPreparation, LogEventQueue, LogEventStore,
};
use windmill::tasks::electoral_log::{
    persist_board_deliveries, LogEventBody, LogEventInput, LogMessageType,
};

const IMMUDB_USER: &str = "immudb";
const IMMUDB_PASSWORD: &str = "immudb";
/// How long RabbitMQ may take to requeue the deliveries of a closed channel.
const REQUEUE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
const REQUEUE_POLL: std::time::Duration = std::time::Duration::from_millis(50);
/// Length of the board's `user_id` column.
const USER_ID_COLUMN_LENGTH: usize = 40;

fn amqp_url() -> String {
    std::env::var("ELECTORAL_LOG_TEST_AMQP_URL").expect("ELECTORAL_LOG_TEST_AMQP_URL")
}

fn immudb_url() -> String {
    std::env::var("ELECTORAL_LOG_TEST_IMMUDB_URL").expect("ELECTORAL_LOG_TEST_IMMUDB_URL")
}

async fn board_client() -> Result<BoardClient> {
    BoardClient::new(&immudb_url(), IMMUDB_USER, IMMUDB_PASSWORD).await
}

/// A fresh synthetic board, its queues and an AMQP connection.
struct Fixture {
    board: String,
    queue: String,
    dead_letter_queue: String,
    connection: Connection,
}

impl Fixture {
    async fn new() -> Result<Self> {
        let suffix = uuid::Uuid::new_v4().simple().to_string();
        let board = format!("synthetic{suffix}");
        board_client()
            .await?
            .upsert_electoral_log_db(&board)
            .await?;
        let connection = Connection::connect(&amqp_url(), ConnectionProperties::default()).await?;
        Ok(Self {
            board,
            queue: format!("synthetic-{suffix}_electoral_log_event_queue"),
            dead_letter_queue: format!("synthetic-{suffix}_electoral_log_dead_letter_queue"),
            connection,
        })
    }

    async fn queue_on(&self, name: &str) -> Result<AmqpLogEventQueue> {
        AmqpLogEventQueue::new(
            self.connection.create_channel().await?,
            name.to_string(),
            self.dead_letter_queue.clone(),
        )
        .await
    }

    async fn publish(&self, queue: &str, correlation_id: &str, payload: &[u8]) -> Result<()> {
        let channel = self.connection.create_channel().await?;
        declare(&channel, queue).await?;
        channel
            .confirm_select(ConfirmSelectOptions::default())
            .await?;
        channel
            .basic_publish(
                "",
                queue,
                BasicPublishOptions::default(),
                payload,
                BasicProperties::default()
                    .with_correlation_id(correlation_id.into())
                    .with_delivery_mode(2),
            )
            .await?
            .await?;
        Ok(())
    }

    async fn ready(&self, queue: &str) -> Result<u32> {
        let channel = self.connection.create_channel().await?;
        Ok(declare(&channel, queue).await?.message_count())
    }

    /// Waits, for a bounded time, until RabbitMQ holds `expected` ready
    /// messages in the queue, as after a closed channel returns its
    /// unacknowledged deliveries.
    async fn wait_until_ready(&self, queue: &str, expected: u32) -> Result<()> {
        let deadline = tokio::time::Instant::now() + REQUEUE_TIMEOUT;
        loop {
            let ready = self.ready(queue).await?;
            if ready == expected {
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(anyhow!(
                    "{queue} holds {ready} ready messages after {REQUEUE_TIMEOUT:?}, expected {expected}"
                ));
            }
            tokio::time::sleep(REQUEUE_POLL).await;
        }
    }

    async fn rows(&self) -> Result<Vec<ElectoralLogMessage>> {
        board_client()
            .await?
            .get_electoral_log_messages(&self.board)
            .await
    }

    async fn drain(&self, queue: &str) -> Result<()> {
        let mut queue = self.queue_on(queue).await?;
        drain_log_event_queue(&mut queue, &mut SyntheticBoards(self.board.clone()), 10).await
    }

    async fn tear_down(self) -> Result<()> {
        board_client().await?.delete_database(&self.board).await?;
        let channel = self.connection.create_channel().await?;
        for queue in [&self.queue, &self.dead_letter_queue] {
            channel
                .queue_delete(queue, Default::default())
                .await
                .with_context(|| format!("Error deleting {queue}"))?;
        }
        Ok(())
    }
}

async fn declare(channel: &Channel, queue: &str) -> Result<lapin::Queue> {
    Ok(channel
        .queue_declare(
            queue,
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?)
}

/// Resolves every event to one synthetic board; the board writes are the
/// production ones.
struct SyntheticBoards(String);

#[async_trait]
impl LogEventStore for SyntheticBoards {
    type Message = ElectoralLogMessage;

    async fn prepare(
        &mut self,
        events: &[LogEventInput],
    ) -> Result<Vec<LogEventPreparation<ElectoralLogMessage>>> {
        Ok(events
            .iter()
            .map(|event| {
                let message = serde_json::from_str::<ElectoralLogMessage>(&event.body.as_raw())
                    .map_err(anyhow::Error::from)
                    .and_then(|message| {
                        message.check_column_limits()?;
                        Ok(message)
                    });
                match message {
                    Ok(message) => LogEventPreparation::Ready {
                        board: self.0.clone(),
                        messages: vec![message],
                    },
                    Err(err) => LogEventPreparation::Rejected(err),
                }
            })
            .collect())
    }

    async fn persist(
        &mut self,
        board: &str,
        deliveries: Vec<BoardDelivery<ElectoralLogMessage>>,
    ) -> Result<Vec<DeliveryReceipt>> {
        persist_board_deliveries(&mut board_client().await?, board, &deliveries).await
    }
}

/// Fails each acknowledgement, as when the worker stops or its connection
/// drops right after the board transaction is committed.
struct AcknowledgementLost(AmqpLogEventQueue);

#[async_trait]
impl LogEventQueue for AcknowledgementLost {
    type Delivery = BasicGetMessage;

    fn payload(delivery: &BasicGetMessage) -> &[u8] {
        AmqpLogEventQueue::payload(delivery)
    }

    fn correlation_id(delivery: &BasicGetMessage) -> Option<&str> {
        AmqpLogEventQueue::correlation_id(delivery)
    }

    async fn next(&mut self) -> Result<Option<BasicGetMessage>> {
        self.0.next().await
    }

    async fn ack(&mut self, _delivery: &BasicGetMessage) -> Result<()> {
        Err(anyhow!("connection lost before the acknowledgement"))
    }

    async fn dead_letter(&mut self, delivery: &BasicGetMessage) -> Result<()> {
        self.0.dead_letter(delivery).await
    }
}

fn cast_vote(created: i64, user_id: &str) -> ElectoralLogMessage {
    ElectoralLogMessage {
        id: 0,
        created,
        sender_pk: "synthetic-sender".to_string(),
        statement_timestamp: created,
        statement_kind: "CastVote".to_string(),
        message: format!("synthetic-signed-statement-{created}").into_bytes(),
        version: "synthetic".to_string(),
        user_id: Some(user_id.to_string()),
        username: None,
        election_id: None,
        area_id: None,
        ballot_id: None,
    }
}

fn input_for(message: &ElectoralLogMessage) -> LogEventInput {
    LogEventInput {
        election_event_id: "00000000-0000-4000-8000-000000000001".to_string(),
        message_type: LogMessageType::Internal,
        user_id: message.user_id.clone(),
        username: None,
        tenant_id: "00000000-0000-4000-8000-000000000002".to_string(),
        body: LogEventBody::Plain(serde_json::to_string(message).expect("serializable")),
    }
}

/// An `enqueue_electoral_log_event` Celery message, as Windmill publishes it.
fn event_message(message: &ElectoralLogMessage) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!([[], { "input": input_for(message) }, {}]))
        .expect("serializable")
}

/// A `process_electoral_log_events_batch` message queued by an earlier version.
fn legacy_batch_message(messages: &[ElectoralLogMessage]) -> Vec<u8> {
    let events: Vec<LogEventInput> = messages.iter().map(input_for).collect();
    serde_json::to_vec(&serde_json::json!([[], { "events": events }, {}])).expect("serializable")
}

fn created_of(rows: &[ElectoralLogMessage]) -> Vec<i64> {
    let mut created: Vec<i64> = rows.iter().map(|row| row.created).collect();
    created.sort();
    created
}

#[tokio::test]
#[ignore]
async fn cast_votes_committed_before_a_lost_acknowledgement_are_written_once() -> Result<()> {
    let fixture = Fixture::new().await?;
    for created in 1..=3 {
        let message = cast_vote(created, "synthetic-voter");
        fixture
            .publish(
                &fixture.queue,
                &format!("task-{created}"),
                &event_message(&message),
            )
            .await?;
    }

    let mut queue = AcknowledgementLost(fixture.queue_on(&fixture.queue).await?);
    let first =
        drain_log_event_queue(&mut queue, &mut SyntheticBoards(fixture.board.clone()), 10).await;
    println!("first drain (acknowledgement lost): {first:?}");
    assert!(first.is_err());
    // Closing the channel returns the unacknowledged deliveries to the queue.
    drop(queue);
    fixture.wait_until_ready(&fixture.queue, 3).await?;
    println!(
        "after first drain: board rows {:?}, queued {}",
        created_of(&fixture.rows().await?),
        fixture.ready(&fixture.queue).await?
    );
    assert_eq!(created_of(&fixture.rows().await?), vec![1, 2, 3]);
    assert_eq!(fixture.ready(&fixture.queue).await?, 3);

    fixture.drain(&fixture.queue).await?;
    let rows = fixture.rows().await?;
    println!(
        "after redelivery: board rows {:?}, queued {}, dead-lettered {}",
        created_of(&rows),
        fixture.ready(&fixture.queue).await?,
        fixture.ready(&fixture.dead_letter_queue).await?
    );
    assert_eq!(created_of(&rows), vec![1, 2, 3]);
    assert_eq!(fixture.ready(&fixture.queue).await?, 0);
    assert_eq!(fixture.ready(&fixture.dead_letter_queue).await?, 0);
    fixture.tear_down().await
}

#[tokio::test]
#[ignore]
async fn legacy_batch_is_stored_once_and_acknowledged() -> Result<()> {
    let fixture = Fixture::new().await?;
    let batch_queue = fixture.queue.replace("event_queue", "batch_queue");
    let batch = legacy_batch_message(&[
        cast_vote(1, "synthetic-voter-1"),
        cast_vote(2, "synthetic-voter-2"),
    ]);
    // The same batch delivered twice, as after a lost acknowledgement.
    fixture.publish(&batch_queue, "legacy-task", &batch).await?;
    fixture.publish(&batch_queue, "legacy-task", &batch).await?;

    fixture.drain(&batch_queue).await?;
    let rows = fixture.rows().await?;
    println!(
        "legacy batch: board rows {:?}, queued {}",
        created_of(&rows),
        fixture.ready(&batch_queue).await?
    );
    assert_eq!(created_of(&rows), vec![1, 2]);
    assert_eq!(fixture.ready(&batch_queue).await?, 0);
    fixture
        .connection
        .create_channel()
        .await?
        .queue_delete(&batch_queue, Default::default())
        .await?;
    fixture.tear_down().await
}

#[tokio::test]
#[ignore]
async fn over_limit_event_is_dead_lettered_without_blocking_its_board() -> Result<()> {
    let fixture = Fixture::new().await?;
    let over_limit = cast_vote(2, &"u".repeat(USER_ID_COLUMN_LENGTH + 1));
    for (id, message) in [
        ("task-1", cast_vote(1, &"u".repeat(USER_ID_COLUMN_LENGTH))),
        ("task-2", over_limit.clone()),
        ("task-3", cast_vote(3, "synthetic-voter")),
    ] {
        fixture
            .publish(&fixture.queue, id, &event_message(&message))
            .await?;
    }

    fixture.drain(&fixture.queue).await?;
    let rows = fixture.rows().await?;
    println!(
        "over-limit: board rows {:?}, queued {}, dead-lettered {}",
        created_of(&rows),
        fixture.ready(&fixture.queue).await?,
        fixture.ready(&fixture.dead_letter_queue).await?
    );
    assert_eq!(created_of(&rows), vec![1, 3]);
    assert_eq!(fixture.ready(&fixture.queue).await?, 0);
    assert_eq!(fixture.ready(&fixture.dead_letter_queue).await?, 1);

    // The board itself refuses the row the check rejected.
    let mut client = board_client().await?;
    client.open_session(&fixture.board).await?;
    let tx = client.new_tx(TxMode::ReadWrite).await?;
    let written = match client
        .insert_electoral_log_messages_batch(&tx, &[over_limit])
        .await
    {
        Ok(()) => client.commit(&tx).await.map(|_| ()),
        Err(err) => Err(err),
    };
    println!("direct write of the over-limit row: {written:?}");
    assert!(written.is_err());
    client.close_session().await.ok();
    assert_eq!(created_of(&fixture.rows().await?), vec![1, 3]);
    fixture.tear_down().await
}

#[tokio::test]
#[ignore]
async fn concurrent_writes_of_one_delivery_commit_once() -> Result<()> {
    let fixture = Fixture::new().await?;
    let message = cast_vote(1, "synthetic-voter");
    let identified = identify_log_events("task-1", false, vec![input_for(&message)])?;
    let (delivery_id, payload_hash) = (
        identified[0].delivery_id.clone(),
        identified[0].payload_hash.clone(),
    );

    let mut first = board_client().await?;
    let mut second = board_client().await?;
    for client in [&mut first, &mut second] {
        client.open_session(&fixture.board).await?;
        client.ensure_electoral_log_delivery_receipts().await?;
    }
    let tx1 = first.new_tx(TxMode::ReadWrite).await?;
    let tx2 = second.new_tx(TxMode::ReadWrite).await?;
    let receipt1 = first
        .insert_electoral_log_delivery(&tx1, &delivery_id, &payload_hash, &[message.clone()])
        .await?;
    let receipt2 = second
        .insert_electoral_log_delivery(
            &tx2,
            &delivery_id,
            &payload_hash,
            &[cast_vote(2, "synthetic-voter")],
        )
        .await?;
    let commit1 = first.commit(&tx1).await.map(|_| ());
    let commit2 = second.commit(&tx2).await.map(|_| ());
    println!("receipts {receipt1:?} {receipt2:?}; commits {commit1:?} {commit2:?}");
    assert_eq!(
        (receipt1, receipt2),
        (DeliveryReceipt::Stored, DeliveryReceipt::Stored)
    );
    assert!(commit1.is_ok());
    assert!(commit2.is_err());
    first.close_session().await.ok();
    second.close_session().await.ok();

    // The losing delivery stays queued; its retry finds the receipt.
    let retry = persist_board_deliveries(
        &mut board_client().await?,
        &fixture.board,
        &[BoardDelivery {
            delivery_id,
            payload_hash,
            messages: vec![cast_vote(3, "synthetic-voter")],
        }],
    )
    .await?;
    let rows = fixture.rows().await?;
    println!(
        "retry receipts {retry:?}; board rows {:?}",
        created_of(&rows)
    );
    assert_eq!(retry, vec![DeliveryReceipt::AlreadyStored]);
    assert_eq!(created_of(&rows), vec![1]);
    fixture.tear_down().await
}

#[test]
fn column_limits_are_checked_in_bytes() {
    let at_limit = cast_vote(1, &"u".repeat(USER_ID_COLUMN_LENGTH));
    assert!(at_limit.check_column_limits().is_ok());
    let over_limit = cast_vote(1, &"u".repeat(USER_ID_COLUMN_LENGTH + 1));
    assert!(over_limit.check_column_limits().is_err());
    // 21 two-byte characters: 21 characters, 42 bytes.
    let multibyte = cast_vote(1, &"é".repeat(USER_ID_COLUMN_LENGTH / 2 + 1));
    assert!(multibyte.check_column_limits().is_err());
    let mut ballot = cast_vote(1, "synthetic-voter");
    ballot.ballot_id = Some("b".repeat(71));
    assert!(ballot.check_column_limits().is_err());
    let mut kind = cast_vote(1, "synthetic-voter");
    kind.statement_kind = "k".repeat(41);
    assert!(kind.check_column_limits().is_err());
}

/// Commits like the production store, then reports a failure, as when the
/// commit's response is lost.
struct CommitResponseLost(String);

#[async_trait]
impl LogEventStore for CommitResponseLost {
    type Message = ElectoralLogMessage;

    async fn prepare(
        &mut self,
        events: &[LogEventInput],
    ) -> Result<Vec<LogEventPreparation<ElectoralLogMessage>>> {
        SyntheticBoards(self.0.clone()).prepare(events).await
    }

    async fn persist(
        &mut self,
        board: &str,
        deliveries: Vec<BoardDelivery<ElectoralLogMessage>>,
    ) -> Result<Vec<DeliveryReceipt>> {
        persist_board_deliveries(&mut board_client().await?, board, &deliveries).await?;
        Err(anyhow!("commit response lost"))
    }
}

#[tokio::test]
#[ignore]
async fn cast_votes_whose_commit_response_was_lost_are_written_once() -> Result<()> {
    let fixture = Fixture::new().await?;
    for created in 1..=2 {
        let message = cast_vote(created, "synthetic-voter");
        fixture
            .publish(
                &fixture.queue,
                &format!("task-{created}"),
                &event_message(&message),
            )
            .await?;
    }

    let mut queue = fixture.queue_on(&fixture.queue).await?;
    let first = drain_log_event_queue(
        &mut queue,
        &mut CommitResponseLost(fixture.board.clone()),
        10,
    )
    .await;
    drop(queue);
    fixture.wait_until_ready(&fixture.queue, 2).await?;
    println!(
        "first drain (commit response lost): {:?}; board rows {:?}, queued {}",
        first.as_ref().map_err(|err| err.to_string()),
        created_of(&fixture.rows().await?),
        fixture.ready(&fixture.queue).await?
    );
    assert!(first.is_err());
    assert_eq!(fixture.ready(&fixture.queue).await?, 2);

    fixture.drain(&fixture.queue).await?;
    let rows = fixture.rows().await?;
    println!(
        "after redelivery: board rows {:?}, queued {}",
        created_of(&rows),
        fixture.ready(&fixture.queue).await?
    );
    assert_eq!(created_of(&rows), vec![1, 2]);
    assert_eq!(fixture.ready(&fixture.queue).await?, 0);
    fixture.tear_down().await
}

fn deliveries_of(
    messages: &[ElectoralLogMessage],
) -> Result<Vec<BoardDelivery<ElectoralLogMessage>>> {
    messages
        .iter()
        .enumerate()
        .map(|(index, message)| {
            let event =
                identify_log_events(&format!("task-{index}"), false, vec![input_for(message)])?
                    .remove(0);
            Ok(BoardDelivery {
                delivery_id: event.delivery_id,
                payload_hash: event.payload_hash,
                messages: vec![message.clone()],
            })
        })
        .collect()
}

#[tokio::test]
#[ignore]
async fn replay_of_only_stored_deliveries_writes_nothing() -> Result<()> {
    let fixture = Fixture::new().await?;
    let deliveries = deliveries_of(&[
        cast_vote(1, "synthetic-voter"),
        cast_vote(2, "synthetic-voter"),
    ])?;

    let first =
        persist_board_deliveries(&mut board_client().await?, &fixture.board, &deliveries).await?;
    let replay =
        persist_board_deliveries(&mut board_client().await?, &fixture.board, &deliveries).await?;
    let rows = fixture.rows().await?;
    println!(
        "first {first:?}; replay {replay:?}; board rows {:?}",
        created_of(&rows)
    );
    assert_eq!(first, vec![DeliveryReceipt::Stored; 2]);
    assert_eq!(replay, vec![DeliveryReceipt::AlreadyStored; 2]);
    assert_eq!(created_of(&rows), vec![1, 2]);
    fixture.tear_down().await
}

#[tokio::test]
#[ignore]
async fn deliveries_after_a_committed_chunk_are_recovered_on_retry() -> Result<()> {
    let fixture = Fixture::new().await?;
    // 16 deliveries fill the first transaction; the 17th, in the second, is
    // refused by the board until it is fixed.
    let mut messages: Vec<ElectoralLogMessage> = (1..=17)
        .map(|created| cast_vote(created, "synthetic-voter"))
        .collect();
    let fixed = messages[16].clone();
    messages[16].user_id = Some("u".repeat(USER_ID_COLUMN_LENGTH + 1));
    let mut deliveries = deliveries_of(&messages)?;

    let first =
        persist_board_deliveries(&mut board_client().await?, &fixture.board, &deliveries).await;
    let after_first = created_of(&fixture.rows().await?);
    println!(
        "first attempt: {:?}; board rows {after_first:?}",
        first.as_ref().map_err(|err| err.to_string())
    );
    assert!(first.is_err());
    assert_eq!(after_first, (1..=16).collect::<Vec<i64>>());

    deliveries[16].messages = vec![fixed];
    let retry =
        persist_board_deliveries(&mut board_client().await?, &fixture.board, &deliveries).await?;
    let rows = created_of(&fixture.rows().await?);
    println!("retry receipts {retry:?}; board rows {rows:?}");
    let mut expected = vec![DeliveryReceipt::AlreadyStored; 16];
    expected.push(DeliveryReceipt::Stored);
    assert_eq!(retry, expected);
    assert_eq!(rows, (1..=17).collect::<Vec<i64>>());
    fixture.tear_down().await
}
