// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

#[path = "support/rabbitmq.rs"]
mod rabbitmq;

use anyhow::{bail, Result};
use lapin::{
    options::{BasicGetOptions, BasicPublishOptions, QueueDeclareOptions},
    types::FieldTable,
    BasicProperties,
};
use rabbitmq::RabbitMq;
use windmill::services::electoral_log_queue::drain_electoral_log_queue;

#[tokio::test]
async fn sink_failure_preserves_the_original_delivery_for_redelivery() -> Result<()> {
    let server = RabbitMq::start().await?;
    let publisher = server.connection.create_channel().await?;
    publisher
        .queue_declare(
            "audit",
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;
    publisher
        .basic_publish(
            "",
            "audit",
            BasicPublishOptions::default(),
            b"immutable-original",
            BasicProperties::default()
                .with_delivery_mode(2)
                .with_correlation_id("stable-event-id".into()),
        )
        .await?
        .await?;
    let result = drain_electoral_log_queue(
        server.connection.create_channel().await?,
        "audit",
        10,
        |deliveries| async move {
            assert_eq!(deliveries.len(), 1);
            assert_eq!(deliveries[0].data, b"immutable-original");
            bail!("injected permanent sink error")
        },
    )
    .await;
    assert_eq!(
        result.unwrap_err().to_string(),
        "injected permanent sink error"
    );
    let delivery = publisher
        .basic_get("audit", BasicGetOptions { no_ack: false })
        .await?
        .expect("sink failure must preserve the original broker message");
    assert!(delivery.redelivered);
    assert_eq!(delivery.data, b"immutable-original");
    assert_eq!(
        delivery
            .properties
            .correlation_id()
            .as_ref()
            .unwrap()
            .as_str(),
        "stable-event-id"
    );
    Ok(())
}

#[tokio::test]
async fn cancellation_requeues_unacknowledged_input_and_a_successful_retry_acknowledges_it(
) -> Result<()> {
    let server = RabbitMq::start().await?;
    let publisher = server.connection.create_channel().await?;
    publisher
        .queue_declare(
            "cancel",
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;
    publisher
        .basic_publish(
            "",
            "cancel",
            BasicPublishOptions::default(),
            b"cancelled-original",
            BasicProperties::default()
                .with_delivery_mode(2)
                .with_correlation_id("cancelled-id".into()),
        )
        .await?
        .await?;
    let channel = server.connection.create_channel().await?;
    let entered = std::sync::Arc::new(tokio::sync::Notify::new());
    let signal = entered.clone();
    let worker = tokio::spawn(async move {
        drain_electoral_log_queue(channel, "cancel", 10, move |deliveries| {
            let signal = signal.clone();
            async move {
                assert_eq!(deliveries[0].data, b"cancelled-original");
                signal.notify_one();
                std::future::pending::<Result<()>>().await
            }
        })
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(10), entered.notified()).await?;
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    // Channel closure is scheduled by lapin's Drop. Observe its actual broker
    // effect before proceeding, rather than depending on a fixed sleep.
    let delivery = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            if let Some(delivery) = publisher
                .basic_get("cancel", BasicGetOptions { no_ack: false })
                .await?
            {
                break Result::<_>::Ok(delivery);
            }
            tokio::task::yield_now().await;
        }
    })
    .await??;
    assert!(delivery.redelivered);
    assert_eq!(delivery.data, b"cancelled-original");
    publisher
        .basic_nack(
            delivery.delivery_tag,
            lapin::options::BasicNackOptions {
                requeue: true,
                ..Default::default()
            },
        )
        .await?;
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = calls.clone();
    drain_electoral_log_queue(
        server.connection.create_channel().await?,
        "cancel",
        10,
        move |deliveries| {
            let observed = observed.clone();
            async move {
                observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                assert_eq!(deliveries[0].data, b"cancelled-original");
                Ok(())
            }
        },
    )
    .await?;
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(publisher
        .basic_get("cancel", BasicGetOptions { no_ack: false })
        .await?
        .is_none());
    Ok(())
}

async fn database() -> Result<electoral_log::BoardClient> {
    let config = std::env::var("ELECTORAL_LOG_TEST_DATABASE_URL")?.parse()?;
    let store = electoral_log::adapters::postgres::PostgresStore::new(config)?;
    store.initialize().await?;
    Ok(electoral_log::BoardClient::new(std::sync::Arc::new(store)))
}

fn audit_message(ballot: &str) -> electoral_log::ElectoralLogMessage {
    electoral_log::ElectoralLogMessage {
        id: 0,
        created: 1_700_000_000_000_000,
        statement_timestamp: 1_700_000_000_000_000,
        sender_pk: "synthetic".into(),
        statement_kind: "CastVote".into(),
        message: vec![1, 2, 3],
        version: "2".into(),
        user_id: Some("voter".into()),
        username: None,
        election_id: Some("election".into()),
        area_id: None,
        ballot_id: Some(ballot.into()),
    }
}

async fn persist_test_delivery(
    client: &electoral_log::BoardClient,
    input: windmill::tasks::electoral_log::IdentifiedLogEvent,
) -> Result<()> {
    let board = input.input.election_event_id.clone();
    windmill::tasks::electoral_log::persist_electoral_log_board(
        &board,
        &[(input, vec![audit_message("original")])],
        || async { Ok(client.clone()) },
    )
    .await
}

#[tokio::test]
async fn legacy_batch_redelivery_recovers_partial_and_uncertain_commits() -> Result<()> {
    use windmill::tasks::electoral_log::decode_electoral_log_delivery;
    let broker = RabbitMq::start().await?;
    let client = database().await?;
    let publisher = broker.connection.create_channel().await?;
    publisher
        .queue_declare(
            "legacy",
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;
    for failure in ["partial", "uncertain"] {
        let prefix = uuid::Uuid::new_v4();
        let boards = [format!("{prefix}-a"), format!("{prefix}-b")];
        for board in &boards {
            client.create_board(board).await?;
        }
        let body = serde_json::to_vec(&serde_json::json!([[], {"events": [
            {"tenant_id":"tenant", "election_event_id":boards[0], "message_type":"LOGIN", "user_id":"voter", "username":null, "body":"null"},
            {"tenant_id":"tenant", "election_event_id":boards[1], "message_type":"LOGIN", "user_id":"voter", "username":null, "body":"null"}
        ]}, {"callbacks":null}]))?;
        publisher
            .basic_publish(
                "",
                "legacy",
                BasicPublishOptions::default(),
                &body,
                BasicProperties::default()
                    .with_delivery_mode(2)
                    .with_correlation_id(format!("legacy-{prefix}").into()),
            )
            .await?
            .await?;
        let result = drain_electoral_log_queue(
            broker.connection.create_channel().await?,
            "legacy",
            10,
            |deliveries| {
                let client = &client;
                async move {
                    for delivery in deliveries {
                        let inputs = decode_electoral_log_delivery(&delivery, true)?;
                        assert_ne!(inputs[0].delivery_id, inputs[1].delivery_id);
                        for (index, input) in inputs.into_iter().enumerate() {
                            persist_test_delivery(client, input).await?;
                            if failure == "partial" && index == 0 {
                                bail!("second board unavailable");
                            }
                        }
                    }
                    bail!("commit response lost")
                }
            },
        )
        .await;
        assert_eq!(
            result.unwrap_err().to_string(),
            if failure == "partial" {
                "second board unavailable"
            } else {
                "commit response lost"
            }
        );
        assert_eq!(
            client
                .count_electoral_log_messages(&boards[0], None)
                .await?,
            1
        );
        assert_eq!(
            client
                .count_electoral_log_messages(&boards[1], None)
                .await?,
            i64::from(failure == "uncertain")
        );
        drain_electoral_log_queue(
            broker.connection.create_channel().await?,
            "legacy",
            10,
            |deliveries| {
                let client = &client;
                let body = &body;
                async move {
                    assert_eq!(deliveries.len(), 1);
                    assert!(deliveries[0].redelivered);
                    assert_eq!(&deliveries[0].data, body);
                    for delivery in deliveries {
                        for input in decode_electoral_log_delivery(&delivery, true)? {
                            persist_test_delivery(client, input).await?;
                        }
                    }
                    Ok(())
                }
            },
        )
        .await?;
        for board in &boards {
            assert_eq!(client.count_electoral_log_messages(board, None).await?, 1);
            client.delete_board(board).await?;
        }
        assert!(publisher
            .basic_get("legacy", BasicGetOptions { no_ack: false })
            .await?
            .is_none());
    }
    Ok(())
}

#[tokio::test]
async fn current_and_invalid_wire_envelopes_keep_stable_identity_and_fail_closed() -> Result<()> {
    use windmill::tasks::electoral_log::{decode_electoral_log_delivery, LogMessageType};
    let server = RabbitMq::start().await?;
    let publisher = server.connection.create_channel().await?;
    publisher
        .queue_declare(
            "wire",
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;
    let current = br#"[[],{"input":{"tenant_id":"tenant","election_event_id":"event","message_type":"LOGIN","user_id":"voter","username":null,"body":"null"}},{"callbacks":null}]"#;
    publisher
        .basic_publish(
            "",
            "wire",
            BasicPublishOptions::default(),
            current,
            BasicProperties::default()
                .with_delivery_mode(2)
                .with_correlation_id("current-id".into()),
        )
        .await?
        .await?;
    drain_electoral_log_queue(
        server.connection.create_channel().await?,
        "wire",
        1,
        |deliveries| async move {
            let decoded = decode_electoral_log_delivery(&deliveries[0], false)?;
            assert_eq!(decoded.len(), 1);
            assert_eq!(
                decoded[0].delivery_id,
                "cbeb44da08682b8edf9bb80da04f2794830f526edba206fe85b059c793a3147a"
            );
            assert_eq!(
                decoded[0].input.message_type,
                LogMessageType::KeycloakEvent("LOGIN".into())
            );
            assert_eq!(decoded[0].input.tenant_id, "tenant");
            assert_eq!(decoded[0].input.election_event_id, "event");
            assert_eq!(decoded[0].input.user_id.as_deref(), Some("voter"));
            assert!(decoded[0].input.username.is_none());
            Ok(())
        },
    )
    .await?;
    for (body, id, expected_error) in [
        (
            current.as_slice(),
            None,
            "electoral log delivery has no stable correlation ID",
        ),
        (
            br#"[[],{},{}]"#.as_slice(),
            Some("missing-input"),
            "Missing input in electoral log event",
        ),
        (
            br#"{}"#.as_slice(),
            Some("bad-envelope"),
            "Invalid Celery message: expected arguments array",
        ),
    ] {
        let mut properties = BasicProperties::default().with_delivery_mode(2);
        if let Some(id) = id {
            properties = properties.with_correlation_id(id.into());
        }
        publisher
            .basic_publish("", "wire", BasicPublishOptions::default(), body, properties)
            .await?
            .await?;
        let error = drain_electoral_log_queue(
            server.connection.create_channel().await?,
            "wire",
            1,
            |deliveries| async move {
                decode_electoral_log_delivery(&deliveries[0], false)?;
                bail!("invalid input unexpectedly decoded")
            },
        )
        .await
        .unwrap_err();
        assert_eq!(error.to_string(), expected_error);
        let retained = publisher
            .basic_get("wire", BasicGetOptions { no_ack: true })
            .await?
            .expect("invalid original must remain for investigation");
        assert!(retained.redelivered);
        assert_eq!(retained.data, body);
    }
    Ok(())
}

#[tokio::test]
async fn a_full_queue_batch_is_atomic_and_replays_without_duplicates() -> Result<()> {
    use windmill::tasks::electoral_log::{
        persist_electoral_log_board, IdentifiedLogEvent, LogEventBody, LogEventInput,
        LogMessageType,
    };
    tokio::time::timeout(std::time::Duration::from_secs(120), async {
        let client = database().await?;
        let board = format!("batch-{}", uuid::Uuid::new_v4());
        client.create_board(&board).await?;
        let mut deliveries: Vec<_> = (0..1000)
            .map(|index| {
                (
                    IdentifiedLogEvent {
                        delivery_id: format!("{index:064x}"),
                        payload_hash: format!("{:064x}", index + 1000),
                        input: LogEventInput {
                            election_event_id: board.clone(),
                            tenant_id: "tenant".into(),
                            message_type: LogMessageType::Internal,
                            user_id: None,
                            username: None,
                            body: LogEventBody::Plain("synthetic".into()),
                            event_time_ms: None,
                        },
                    },
                    vec![
                        audit_message(&format!("send-{index}")),
                        audit_message(&format!("event-{index}")),
                    ],
                )
            })
            .collect();
        persist_electoral_log_board(&board, &deliveries, || async { Ok(client.clone()) }).await?;
        persist_electoral_log_board(&board, &deliveries, || async { Ok(client.clone()) }).await?;
        let rows = client
            .get_electoral_log_messages_batch(&board, 2500, 0)
            .await?;
        assert_eq!(rows.len(), 2000);
        let ballots: std::collections::HashSet<_> = rows
            .iter()
            .map(|row| row.ballot_id.as_deref().unwrap())
            .collect();
        for index in 0..1000 {
            assert!(ballots.contains(format!("send-{index}").as_str()));
            assert!(ballots.contains(format!("event-{index}").as_str()));
        }
        // New rows preceding a conflicting delivery must be rolled back together.
        deliveries[0].0.delivery_id = "new-input".into();
        deliveries[1].0.payload_hash = "changed-input".into();
        let error =
            persist_electoral_log_board(&board, &deliveries, || async { Ok(client.clone()) })
                .await
                .unwrap_err();
        assert!(format!("{error:#}").contains("different payload"));
        assert_eq!(
            client.count_electoral_log_messages(&board, None).await?,
            2000
        );
        client.delete_board(&board).await?;
        Ok::<(), anyhow::Error>(())
    })
    .await??;
    Ok(())
}
