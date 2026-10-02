// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use celery::prelude::*;
use futures::StreamExt;
use std::sync::atomic::{AtomicUsize, Ordering};

static EXECUTIONS: AtomicUsize = AtomicUsize::new(0);

#[celery::task(max_retries = 1, min_retry_delay = 1, max_retry_delay = 1)]
async fn retry_once() -> TaskResult<()> {
    if EXECUTIONS.fetch_add(1, Ordering::SeqCst) == 0 {
        Err(TaskError::ExpectedError("retry contract".into()))
    } else {
        Ok(())
    }
}

#[test]
fn queue_names_preserve_namespace_and_fit_postgres() {
    assert_eq!(queue_name("tenant-a_reports_queue").len(), 45);
    assert_ne!(
        queue_name("Tenant_a_reports_queue"),
        queue_name("tenant_a_reports_queue")
    );
    assert_ne!(
        queue_name("tenant-a_reports_queue"),
        queue_name("tenant_a_reports_queue")
    );
    assert!(queue_name(&"x".repeat(200))
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_'));
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

#[tokio::test]
#[ignore = "requires a disposable PostgreSQL database initialized with PGMQ 1.13.0"]
async fn postgres_delivery_contract() {
    let pool = test_pool();
    let logical = format!("contract_{}", uuid::Uuid::new_v4());
    let queue = queue_name(&logical);
    let broker = Box::new(PgmqBrokerBuilder::from_pool(pool.clone()))
        .prefetch_count(1)
        .declare_queue(&logical)
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
    send(&*tx, &logical, &message).await.unwrap();
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
    broker.send(&delayed, &logical).await.unwrap();
    let (tag, mut stream) = broker
        .consume(&logical, Box::new(|e| panic!("{e}")))
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
        .consume(&logical, Box::new(|e| panic!("{e}")))
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
#[ignore = "requires a disposable PostgreSQL database initialized with PGMQ 1.13.0"]
async fn celery_worker_and_beat_contract() {
    tokio::task::LocalSet::new().run_until(async {

    EXECUTIONS.store(0, Ordering::SeqCst);
    let pool = test_pool();
    let logical = format!("worker_{}", uuid::Uuid::new_v4());
    let app = celery::app!(
        broker_builder = Box::new(PgmqBrokerBuilder::from_pool(pool.clone())),
        tasks = [retry_once],
        task_routes = [retry_once::NAME => &logical],
        default_queue = &logical,
        acks_late = true,
        prefetch_count = 1,
    ).await.unwrap();
    let mut beat = celery::beat!(
        broker_builder = Box::new(PgmqBrokerBuilder::from_pool(pool.clone()).publisher_connection(Arc::new(pool.get().await.unwrap()))),
        tasks = ["retry" => { retry_once, schedule = celery::beat::DeltaSchedule::new(Duration::from_secs(60)), args = () }],
        task_routes = [retry_once::NAME => &logical],
        default_queue = &logical,
    ).await.unwrap();
    // Expired deliveries must be archived without entering the handler.
    app.send_task(retry_once::new().with_expires(Utc::now() - chrono::Duration::seconds(1))).await.unwrap();
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
            let count: i64 = pool.get().await.unwrap().query_one("SELECT queue_length FROM pgmq.metrics($1)", &[&queue_name(&logical)]).await.unwrap().get(0);
            if count == 0 { break; }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }).await.unwrap();
    assert_eq!(EXECUTIONS.load(Ordering::SeqCst), 2);
    app.close().await.unwrap();
    worker.abort();
    pool.get().await.unwrap().query_one("SELECT pgmq.drop_queue($1)", &[&queue_name(&logical)]).await.unwrap();
    }).await;
}
