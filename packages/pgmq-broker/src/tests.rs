// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::setup::{setup, Installation, Roles, SetupError};
use super::*;
use celery::prelude::*;
use futures::StreamExt;
use std::sync::atomic::{AtomicUsize, Ordering};

static EXECUTIONS: AtomicUsize = AtomicUsize::new(0);
/// The environment the disposable test database is set up for, shared with Windmill's tests.
const TEST_ENVIRONMENT: &str = "pgmq-test";

#[celery::task(max_retries = 1, min_retry_delay = 1, max_retry_delay = 1)]
async fn retry_once() -> TaskResult<()> {
    if EXECUTIONS.fetch_add(1, Ordering::SeqCst) == 0 {
        Err(TaskError::ExpectedError("retry contract".into()))
    } else {
        Ok(())
    }
}

#[celery::task(max_retries = 0)]
async fn always_fails() -> TaskResult<()> {
    Err(TaskError::ExpectedError("failure contract".into()))
}

#[test]
fn queue_names_are_safe_table_suffixes() {
    for valid in [
        "short_queue",
        "electoral_log_dead_letter_queue",
        &"q".repeat(MAX_QUEUE_NAME_LEN),
    ] {
        assert!(validate_queue_name(valid).is_ok(), "{valid}");
    }
    for invalid in [
        "",
        &"q".repeat(MAX_QUEUE_NAME_LEN + 1),
        "Short_queue",
        "tenant-a_short_queue",
        "short queue",
        "q;drop",
        "q\u{e9}",
    ] {
        assert!(validate_queue_name(invalid).is_err(), "{invalid}");
    }
}

fn test_pool() -> Arc<Pool> {
    let url = std::env::var("PGMQ_TEST_DATABASE_URL")
        .expect("set PGMQ_TEST_DATABASE_URL to a disposable database");
    let config = deadpool_postgres::Config {
        url: Some(url),
        ..Default::default()
    };
    Arc::new(
        config
            .create_pool(
                Some(deadpool_postgres::Runtime::Tokio1),
                tokio_postgres::NoTls,
            )
            .unwrap(),
    )
}

fn unique_queue(prefix: &str) -> String {
    format!("{prefix}_{}", uuid::Uuid::new_v4().simple())
}

/// Set up the test database, as its owner would, with `queues` and no role grants.
async fn set_up(pool: &Pool, queues: &[&str]) {
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    setup(
        &tx,
        &Installation {
            environment: TEST_ENVIRONMENT,
            queues,
            roles: None,
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

fn test_broker(pool: &Arc<Pool>) -> Box<PgmqBrokerBuilder> {
    Box::new(PgmqBrokerBuilder::from_pool(pool.clone()).environment(TEST_ENVIRONMENT))
}

#[tokio::test]
#[ignore = "requires a disposable PostgreSQL database"]
async fn setup_is_idempotent_and_brokers_refuse_other_databases() {
    let pool = test_pool();
    let queue = unique_queue("setup");
    set_up(&pool, &[&queue]).await;
    set_up(&pool, &[&queue]).await;

    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let error = setup(
        &tx,
        &Installation {
            environment: "another-environment",
            queues: &[&queue],
            roles: None,
        },
    )
    .await
    .unwrap_err();
    tx.rollback().await.unwrap();
    let SetupError::Database(error) = error else {
        panic!("{error}")
    };
    assert!(error
        .as_db_error()
        .unwrap()
        .message()
        .contains("belongs to environment"));
    for invalid in [
        Installation {
            environment: "",
            queues: &[&queue],
            roles: None,
        },
        Installation {
            environment: TEST_ENVIRONMENT,
            queues: &["Invalid-Name"],
            roles: None,
        },
    ] {
        let tx = client.transaction().await.unwrap();
        assert!(setup(&tx, &invalid).await.is_err());
        tx.rollback().await.unwrap();
    }

    assert!(test_broker(&pool)
        .declare_queue(&queue)
        .build(5)
        .await
        .is_ok());
    let other = Box::new(PgmqBrokerBuilder::from_pool(pool.clone()).environment("other"))
        .declare_queue(&queue)
        .build(5)
        .await;
    assert!(matches!(other, Err(BrokerError::InvalidBrokerUrl(_))));
    let unset = Box::new(PgmqBrokerBuilder::from_pool(pool.clone()))
        .declare_queue(&queue)
        .build(5)
        .await;
    assert!(matches!(unset, Err(BrokerError::InvalidBrokerUrl(_))));
    let missing = unique_queue("missing");
    let unknown = test_broker(&pool).declare_queue(&missing).build(5).await;
    assert!(matches!(unknown, Err(BrokerError::UnknownQueue(name)) if name == missing));
    let invalid = test_broker(&pool)
        .declare_queue("Not-A-Queue")
        .build(5)
        .await;
    assert!(matches!(invalid, Err(BrokerError::UnknownQueue(_))));
    client
        .query_one("SELECT pgmq.drop_queue($1)", &[&queue])
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires a disposable PostgreSQL database and a role that may create roles"]
async fn roles_get_only_the_privileges_of_their_component() {
    let pool = test_pool();
    let queue = unique_queue("roles");
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let roles = Roles {
        worker: format!("worker_{suffix}"),
        producer: format!("producer_{suffix}"),
        reader: format!("reader_{suffix}"),
    };
    let mut client = pool.get().await.unwrap();
    for role in [&roles.worker, &roles.producer, &roles.reader] {
        client
            .batch_execute(&format!("CREATE ROLE {role} NOLOGIN"))
            .await
            .unwrap();
    }
    let tx = client.transaction().await.unwrap();
    setup(
        &tx,
        &Installation {
            environment: TEST_ENVIRONMENT,
            queues: &[&queue],
            roles: Some(&roles),
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let allowed = |role: &str, statement: String| {
        let client = &client;
        let role = role.to_string();
        async move {
            client
                .batch_execute(&format!("SET ROLE {role}"))
                .await
                .unwrap();
            let result = client.batch_execute(&statement).await;
            client.batch_execute("RESET ROLE").await.unwrap();
            match result {
                Ok(()) => true,
                Err(error) => {
                    assert_eq!(
                        error.code(),
                        Some(&tokio_postgres::error::SqlState::INSUFFICIENT_PRIVILEGE),
                        "{role}: {statement}: {error:?}"
                    );
                    false
                }
            }
        }
    };
    let send = format!("SELECT pgmq.send('{queue}', '{{}}')");
    let read = format!("SELECT * FROM pgmq.read('{queue}', 1, 1)");
    let archive = format!("SELECT pgmq.archive('{queue}', msg_id) FROM pgmq.q_{queue}");
    let metrics = format!("SELECT * FROM pgmq.metrics('{queue}')");
    let purge = format!("DELETE FROM pgmq.a_{queue} WHERE archived_at < now()");
    let environment = "SELECT environment FROM step_queue.installation".to_string();

    assert!(allowed(&roles.producer, send.clone()).await);
    assert!(allowed(&roles.producer, environment.clone()).await);
    assert!(!allowed(&roles.producer, read.clone()).await);
    assert!(!allowed(&roles.producer, metrics.clone()).await);
    assert!(!allowed(&roles.producer, purge.clone()).await);

    assert!(allowed(&roles.reader, metrics.clone()).await);
    assert!(allowed(&roles.reader, format!("SELECT * FROM pgmq.a_{queue}")).await);
    assert!(!allowed(&roles.reader, send.clone()).await);
    assert!(!allowed(&roles.reader, read.clone()).await);

    for statement in [send, read, archive, metrics, purge, environment] {
        assert!(allowed(&roles.worker, statement).await);
    }

    client
        .query_one("SELECT pgmq.drop_queue($1)", &[&queue])
        .await
        .unwrap();
    for role in [&roles.worker, &roles.producer, &roles.reader] {
        client
            .batch_execute(&format!("DROP OWNED BY {role}; DROP ROLE {role}"))
            .await
            .unwrap();
    }
}

#[tokio::test]
#[ignore = "requires a disposable PostgreSQL database"]
async fn postgres_delivery_contract() {
    let pool = test_pool();
    let queue = unique_queue("contract");
    set_up(&pool, &[&queue]).await;
    let broker = test_broker(&pool)
        .prefetch_count(1)
        .declare_queue(&queue)
        .build(5)
        .await
        .unwrap();
    let client = pool.get().await.unwrap();
    // Preserve useful SQLSTATE diagnostics without exposing server error payloads.
    let error = client
        .simple_query(
            "DO $$ BEGIN RAISE EXCEPTION 'private-voter-payload' USING ERRCODE = '53100'; END $$",
        )
        .await
        .unwrap_err();
    let diagnostic = db_error(deadpool_postgres::PoolError::Backend(error)).to_string();
    assert!(diagnostic.contains("53100"));
    assert!(!diagnostic.contains("private-voter-payload"));
    let message = Message::try_from(retry_once::new()).unwrap();
    // A producer rollback must not leave a job behind.
    let mut tx_client = pool.get().await.unwrap();
    let tx = tx_client.transaction().await.unwrap();
    send(&*tx, &queue, &message).await.unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(
        client
            .query_one("SELECT queue_length FROM pgmq.metrics($1)", &[&queue])
            .await
            .unwrap()
            .get::<_, i64>(0),
        0
    );

    let mut delayed = message.clone();
    delayed.headers.eta = Some(Utc::now() + chrono::Duration::seconds(2));
    broker.send(&delayed, &queue).await.unwrap();
    let (tag, mut stream) = broker
        .consume(&queue, Box::new(|e| panic!("{e}")))
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(400), stream.next())
            .await
            .is_err()
    );
    let delivery = tokio::time::timeout(Duration::from_secs(4), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap_or_else(|error| panic!("{error}"));
    let received = delivery.try_deserialize_message().unwrap();
    assert_eq!(received.headers.id, message.headers.id);
    assert_eq!(received.raw_body, message.raw_body);
    let first: DateTime<Utc> = client
        .query_one(&format!("SELECT vt FROM pgmq.q_{queue}"), &[])
        .await
        .unwrap()
        .get(0);
    // Renewal must cover execution beyond the initial claim, including local semaphore waits.
    tokio::time::sleep(Duration::from_secs(11)).await;
    let renewed: DateTime<Utc> = client
        .query_one(&format!("SELECT vt FROM pgmq.q_{queue}"), &[])
        .await
        .unwrap()
        .get(0);
    assert!(renewed > first);
    broker.retry(delivery.as_ref(), None).await.unwrap();
    broker.ack(delivery.as_ref()).await.unwrap();
    drop(delivery);
    let delivery = tokio::time::timeout(Duration::from_secs(2), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(
        delivery.try_deserialize_message().unwrap().headers.retries,
        Some(1)
    );
    // A newer claim fences off the old owner's ack and retry.
    client
        .execute(
            &format!("UPDATE pgmq.q_{queue} SET read_ct = read_ct + 1"),
            &[],
        )
        .await
        .unwrap();
    assert!(broker.ack(delivery.as_ref()).await.is_err());
    assert!(broker.retry(delivery.as_ref(), None).await.is_err());
    drop(delivery); // Stops renewal, as process death would.
    client
        .execute(
            &format!("UPDATE pgmq.q_{queue} SET vt = clock_timestamp() - interval '1 second'"),
            &[],
        )
        .await
        .unwrap();
    let recovered = tokio::time::timeout(Duration::from_secs(2), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap_or_else(|error| panic!("{error}"));
    broker.ack(recovered.as_ref()).await.unwrap();
    drop(recovered);
    assert_eq!(
        client
            .query_one(&format!("SELECT count(*) FROM pgmq.a_{queue}"), &[])
            .await
            .unwrap()
            .get::<_, i64>(0),
        1
    );
    broker.cancel(&tag).await.unwrap();
    drop(stream);
    // Malformed payloads remain inspectable after terminal acknowledgement.
    client
        .query_one(
            "SELECT pgmq.send($1, $2)",
            &[&queue, &serde_json::json!({"invalid":true})],
        )
        .await
        .unwrap();
    let (_, mut stream) = broker
        .consume(&queue, Box::new(|e| panic!("{e}")))
        .await
        .unwrap();
    let malformed = tokio::time::timeout(Duration::from_secs(2), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(malformed.try_deserialize_message().is_err());
    broker.ack(malformed.as_ref()).await.unwrap();
    drop(malformed);
    broker.close().await.unwrap();
    drop(stream);
    client
        .query_one("SELECT pgmq.drop_queue($1)", &[&queue])
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires a disposable PostgreSQL database"]
async fn celery_worker_and_beat_contract() {
    tokio::task::LocalSet::new().run_until(async {

    EXECUTIONS.store(0, Ordering::SeqCst);
    let pool = test_pool();
    let logical = unique_queue("worker");
    set_up(&pool, &[&logical]).await;
    let app = celery::app!(
        broker_builder = test_broker(&pool),
        tasks = [retry_once, always_fails],
        task_routes = [retry_once::NAME => &logical, always_fails::NAME => &logical],
        default_queue = &logical,
        acks_late = true,
        prefetch_count = 1,
    ).await.unwrap();
    let mut beat = celery::beat!(
        broker_builder = Box::new(PgmqBrokerBuilder::from_pool(pool.clone()).environment(TEST_ENVIRONMENT).publisher_connection(Arc::new(pool.get().await.unwrap()))),
        tasks = ["retry" => { retry_once, schedule = celery::beat::DeltaSchedule::new(Duration::from_secs(60)), args = () }],
        task_routes = [retry_once::NAME => &logical],
        default_queue = &logical,
    ).await.unwrap();
    // Expired deliveries must be archived without entering the handler.
    app.send_task(retry_once::new().with_expires(Utc::now() - chrono::Duration::seconds(1))).await.unwrap();
    app.send_task(always_fails::new()).await.unwrap();
    pool.get().await.unwrap().query_one("SELECT pgmq.send($1, $2)", &[&logical, &serde_json::json!({"invalid": true})]).await.unwrap();
    let worker_app = app.clone();
    let worker_queue = logical.clone();
    let worker = tokio::task::spawn_local(async move { worker_app.consume_from(&[&worker_queue]).await });
    let scheduler = tokio::task::spawn_local(async move { beat.start().await });
    tokio::time::timeout(Duration::from_secs(10), async {
        while EXECUTIONS.load(Ordering::SeqCst) < 2 { tokio::time::sleep(Duration::from_millis(100)).await; }
    }).await.unwrap();
    scheduler.abort();
    // Wait for late ack before closing consumers.
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let count: i64 = pool.get().await.unwrap().query_one("SELECT queue_length FROM pgmq.metrics($1)", &[&logical]).await.unwrap().get(0);
            if count == 0 { break; }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }).await.unwrap();
    assert_eq!(EXECUTIONS.load(Ordering::SeqCst), 2);
    // Every archived message records how its processing ended; a retry keeps the message.
    let outcomes: Vec<String> = pool.get().await.unwrap().query(
        &format!("SELECT headers->>'{OUTCOME_HEADER}' FROM pgmq.a_{logical} ORDER BY 1"), &[],
    ).await.unwrap().iter().map(|row| row.get(0)).collect();
    assert_eq!(outcomes, ["expired", "failed", "rejected", "succeeded"]);
    app.close().await.unwrap();
    worker.abort();
    pool.get().await.unwrap().query_one("SELECT pgmq.drop_queue($1)", &[&logical]).await.unwrap();
    }).await;
}

#[tokio::test]
#[ignore = "requires a disposable PostgreSQL database"]
async fn purge_deletes_only_archived_messages_past_the_retention() {
    let pool = test_pool();
    let queue = unique_queue("purge");
    set_up(&pool, &[&queue]).await;
    let client = pool.get().await.unwrap();
    for _ in 0..4 {
        client
            .query_one("SELECT pgmq.send($1, '{}')", &[&queue])
            .await
            .unwrap();
    }
    client
        .query(
            &format!("SELECT pgmq.archive($1, msg_id) FROM pgmq.q_{queue} WHERE msg_id <= 3"),
            &[&queue],
        )
        .await
        .unwrap();
    client
        .execute(
            &format!(
                "UPDATE pgmq.a_{queue} SET archived_at = now() - interval '2 hours' WHERE msg_id <= 2"
            ),
            &[],
        )
        .await
        .unwrap();

    let deleted = purge_archive(&**client, &queue, Duration::from_secs(3600), 1)
        .await
        .unwrap();
    assert_eq!(deleted, 2);
    let count = |table: String| {
        let client = &client;
        async move {
            client
                .query_one(&format!("SELECT count(*) FROM pgmq.{table}"), &[])
                .await
                .unwrap()
                .get::<_, i64>(0)
        }
    };
    assert_eq!(count(format!("a_{queue}")).await, 1);
    assert_eq!(count(format!("q_{queue}")).await, 1);
    assert_eq!(
        purge_archive(&**client, &queue, Duration::from_secs(3600), 100)
            .await
            .unwrap(),
        0
    );
    assert!(
        purge_archive(&**client, "Not-A-Queue", Duration::from_secs(1), 1)
            .await
            .is_err()
    );
    client
        .query_one("SELECT pgmq.drop_queue($1)", &[&queue])
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires a disposable PostgreSQL database"]
async fn inspection_summarizes_queues_without_task_arguments() {
    use super::inspect::{messages, overview, throughput, InspectError, MessageState};
    let pool = test_pool();
    let queue = unique_queue("inspect");
    set_up(&pool, &[&queue]).await;
    let client = pool.get().await.unwrap();
    let message = Message::try_from(retry_once::new()).unwrap();
    send(&**client, &queue, &message).await.unwrap();
    send(&**client, &queue, &message).await.unwrap();
    client
        .query_one("SELECT pgmq.send($1, '{\"not\": \"celery\"}')", &[&queue])
        .await
        .unwrap();
    client
        .execute(
            &format!(
                "UPDATE pgmq.q_{queue} SET headers = \
                 jsonb_build_object('{OUTCOME_HEADER}', 'succeeded', 'private', 'x') \
                 WHERE msg_id = 2"
            ),
            &[],
        )
        .await
        .unwrap();
    client
        .query_one("SELECT pgmq.archive($1, 2::bigint)", &[&queue])
        .await
        .unwrap();

    let queues = overview(&**client, &[&queue]).await.unwrap();
    assert_eq!(queues.len(), 1);
    assert_eq!(queues[0].ready, 2);
    assert_eq!(queues[0].running_or_scheduled, 0);
    assert_eq!(queues[0].total_sent, 3);
    assert_eq!(queues[0].last_hour.get("succeeded"), Some(&1));

    let queued = messages(&**client, &queue, MessageState::Queued, None, 10)
        .await
        .unwrap();
    assert_eq!(
        queued
            .iter()
            .map(|message| message.msg_id)
            .collect::<Vec<_>>(),
        [3, 1]
    );
    assert_eq!(queued[0].task, None);
    assert!(queued[0].size_bytes > 0);
    assert_eq!(queued[1].task.as_deref(), Some(retry_once::NAME));
    assert_eq!(
        queued[1].task_id.as_deref(),
        Some(message.headers.id.as_str())
    );
    let older = messages(&**client, &queue, MessageState::Queued, Some(3), 10)
        .await
        .unwrap();
    assert_eq!(older.len(), 1);
    let archived = messages(&**client, &queue, MessageState::Archived, None, 10)
        .await
        .unwrap();
    assert_eq!(archived.len(), 1);
    assert!(archived[0].archived_at.is_some());
    assert_eq!(
        archived[0].headers.keys().collect::<Vec<_>>(),
        [OUTCOME_HEADER]
    );

    let buckets = throughput(
        &**client,
        &queue,
        Duration::from_secs(3600),
        Duration::from_secs(60),
    )
    .await
    .unwrap();
    assert_eq!(buckets.len(), 1);
    assert_eq!(buckets[0].outcomes.get("succeeded"), Some(&1));

    assert!(matches!(
        overview(&**client, &["Not-A-Queue"]).await,
        Err(InspectError::InvalidQueueName(_))
    ));
    assert!(matches!(
        messages(&**client, &queue, MessageState::Queued, None, 0).await,
        Err(InspectError::InvalidRange(_))
    ));
    assert!(matches!(
        throughput(
            &**client,
            &queue,
            Duration::from_secs(60),
            Duration::from_secs(3600)
        )
        .await,
        Err(InspectError::InvalidRange(_))
    ));
    client
        .query_one("SELECT pgmq.drop_queue($1)", &[&queue])
        .await
        .unwrap();
}
