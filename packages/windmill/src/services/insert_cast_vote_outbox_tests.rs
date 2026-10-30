// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Delivery of an accepted vote's signed CastVote against real PostgreSQL,
//! RabbitMQ and immudb services, with synthetic data only. Kept apart from
//! `tests`, whose ignored tests need only PostgreSQL. Run with
//! `CAST_VOTE_TEST_DATABASE_URL=… ELECTORAL_LOG_TEST_AMQP_URL=…
//! ELECTORAL_LOG_TEST_IMMUDB_URL=… cargo test -p windmill --lib
//! services::insert_cast_vote::outbox_tests -- --include-ignored`.

use super::tests::database::electoral_log_outbox::{cast, count, open_election, remove};
use super::tests::database::test_client;
use crate::services::electoral_log_outbox::{
    publish_and_remove, publish_log_event, relay_log_events,
};
use crate::services::electoral_log_queue::{
    drain_log_event_queue, AmqpLogEventQueue, BoardDelivery, LogEventPreparation, LogEventStore,
};
use crate::tasks::electoral_log::{persist_board_deliveries, LogEventInput};
use async_trait::async_trait;
use electoral_log::client::board_client::{BoardClient, DeliveryReceipt, ElectoralLogMessage};
use lapin::{options::QueueDeclareOptions, types::FieldTable, Connection, ConnectionProperties};
use strand::signature::StrandSignatureSk;
use uuid::Uuid;

struct SyntheticBoard(String);

#[async_trait]
impl LogEventStore for SyntheticBoard {
    type Message = ElectoralLogMessage;

    async fn prepare(
        &mut self,
        events: &[LogEventInput],
    ) -> anyhow::Result<Vec<LogEventPreparation<ElectoralLogMessage>>> {
        events
            .iter()
            .map(|event| {
                Ok(LogEventPreparation::Ready {
                    board: self.0.clone(),
                    messages: vec![serde_json::from_str(&event.body.as_raw())?],
                })
            })
            .collect()
    }

    async fn persist(
        &mut self,
        board: &str,
        deliveries: Vec<BoardDelivery<ElectoralLogMessage>>,
    ) -> anyhow::Result<Vec<DeliveryReceipt>> {
        persist_board_deliveries(&mut immudb().await, board, &deliveries).await
    }
}

async fn immudb() -> BoardClient {
    let url =
        std::env::var("ELECTORAL_LOG_TEST_IMMUDB_URL").expect("ELECTORAL_LOG_TEST_IMMUDB_URL");
    BoardClient::new(&url, "immudb", "immudb").await.unwrap()
}

async fn queue_ready(connection: &Connection, queue: &str) -> u32 {
    connection
        .create_channel()
        .await
        .unwrap()
        .queue_declare(
            queue,
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .unwrap()
        .message_count()
}

async fn rabbitmq() -> Connection {
    let url = std::env::var("ELECTORAL_LOG_TEST_AMQP_URL").expect("ELECTORAL_LOG_TEST_AMQP_URL");
    Connection::connect(&url, ConnectionProperties::default())
        .await
        .unwrap()
}

#[tokio::test]
#[ignore = "requires the disposable database fixture, RabbitMQ and immudb"]
async fn vote_accepted_while_rabbitmq_is_down_reaches_its_board_once() {
    let mut client = test_client().await;
    let fixture = open_election(&mut client).await;
    let signing_key = StrandSignatureSk::gen().unwrap();
    let suffix = Uuid::new_v4().simple().to_string();
    let board = format!("synthetic{suffix}");
    let queue_name = format!("synthetic-{suffix}_electoral_log_event_queue");
    let dead_letter = format!("{queue_name}_dead_letter");
    immudb()
        .await
        .upsert_electoral_log_db(&board)
        .await
        .unwrap();

    let (_, _, pending) = cast(&mut client, &fixture, &signing_key).await.unwrap();
    // The broker closes the connection before the vote's event is published,
    // as with a forced connection close. No database client is taken for a
    // publish that RabbitMQ did not confirm.
    let lost = rabbitmq().await;
    lost.close(320, "CONNECTION_FORCED").await.unwrap();
    let mut client_taken = false;
    let delivered = publish_and_remove(&lost, &queue_name, &pending, || {
        client_taken = true;
        async { Ok(test_client().await) }
    })
    .await;
    println!(
        "after the vote, broker connection closed: delivery {:?}; database client taken {client_taken}; votes {}, outbox events {}",
        delivered.as_ref().map_err(|err| err.to_string()),
        count(&client, "cast_vote", &fixture).await,
        count(&client, "electoral_log_outbox", &fixture).await
    );
    assert!(delivered.is_err());
    assert!(!client_taken);
    assert_eq!(count(&client, "cast_vote", &fixture).await, 1);
    assert_eq!(count(&client, "electoral_log_outbox", &fixture).await, 1);

    // RabbitMQ is back. A delivery is confirmed by RabbitMQ, but its process
    // stops before removing the row, which stays pending.
    let connection = rabbitmq().await;
    publish_log_event(&connection, &queue_name, &pending)
        .await
        .unwrap();
    let pending_after_publish = count(&client, "electoral_log_outbox", &fixture).await;
    // The relay publishes the pending row again, under the same ID, and
    // removes it.
    let relayed = relay_log_events(
        &mut client,
        &connection,
        &queue_name,
        std::time::Duration::ZERO,
        500,
    )
    .await
    .unwrap();
    println!(
        "publish confirmed, process stopped before removal: outbox events {pending_after_publish}; \
         relayed {relayed}; outbox events {}",
        count(&client, "electoral_log_outbox", &fixture).await
    );
    assert_eq!(pending_after_publish, 1);
    assert_eq!(relayed, 1);
    assert_eq!(count(&client, "electoral_log_outbox", &fixture).await, 0);

    let mut queue = AmqpLogEventQueue::new(
        connection.create_channel().await.unwrap(),
        queue_name.clone(),
        dead_letter.clone(),
    )
    .await
    .unwrap();
    drain_log_event_queue(&mut queue, &mut SyntheticBoard(board.clone()), 10)
        .await
        .unwrap();
    let queued_after_drain = queue_ready(&connection, &queue_name).await;
    let rows = immudb()
        .await
        .get_electoral_log_messages(&board)
        .await
        .unwrap();
    let signed: ElectoralLogMessage = serde_json::from_str(&pending.input.body.as_raw()).unwrap();
    println!(
        "board rows {}: kinds {:?}; signed statement preserved {}",
        rows.len(),
        rows.iter()
            .map(|row| row.statement_kind.as_str())
            .collect::<Vec<_>>(),
        rows.first()
            .map(|row| row.message == signed.message)
            .unwrap_or(false)
    );
    // Both copies were consumed and only one was written.
    assert_eq!(queued_after_drain, 0);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].statement_kind, "CastVote");
    assert_eq!(rows[0].message, signed.message);

    immudb().await.delete_database(&board).await.unwrap();
    let channel = connection.create_channel().await.unwrap();
    for queue in [queue_name, dead_letter] {
        channel
            .queue_delete(&queue, Default::default())
            .await
            .unwrap();
    }
    remove(&client, &fixture).await;
}
