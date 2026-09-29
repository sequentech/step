// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Counting sign-in attempts from the electoral log's Keycloak events: what
//! an event counts as, the 15-minute bucket it falls in, and that a delivery
//! counts once however often it is delivered.

#[path = "support/schema.rs"]
mod schema;

use chrono::{DateTime, TimeZone, Utc};
use deadpool_postgres::{Pool, Transaction};
use uuid::Uuid;
use windmill::postgres::election_event::delete_election_event;
use windmill::services::monitoring::login_counter::{
    bucket_start, count_login_attempts, count_login_attempts_apart, login_attempt,
    prune_login_counter_receipts, LoginAttempt, Registration,
};
use windmill::tasks::electoral_log::{LogEventBody, LogEventInput, LogMessageType};

const TENANT: &str = "0e1b3a0c-1f7e-4d3c-9d62-2f2a2b7c0a01";
const EVENT: &str = "6f1d9d4e-8a8b-4d7e-b5a4-0c9a1f2e3d04";
const AREA: &str = "a3c1e2d4-5b6f-4a7b-8c9d-0e1f2a3b4c05";

fn at(hour: u32, minute: u32, second: u32, milli: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 11, hour, minute, second)
        .unwrap()
        + chrono::Duration::milliseconds(milli.into())
}

fn keycloak(event_type: &str, user_id: Option<&str>, event_time_ms: Option<i64>) -> LogEventInput {
    LogEventInput {
        election_event_id: EVENT.to_string(),
        message_type: LogMessageType::KeycloakEvent(event_type.to_string()),
        user_id: user_id.map(str::to_string),
        username: Some("voter".to_string()),
        tenant_id: TENANT.to_string(),
        body: LogEventBody::Plain("null".to_string()),
        event_time_ms,
    }
}

#[test]
fn a_bucket_is_the_quarter_hour_of_utc_an_event_falls_in() {
    assert_eq!(bucket_start(at(14, 59, 59, 999)), at(14, 45, 0, 0));
    assert_eq!(bucket_start(at(15, 0, 0, 0)), at(15, 0, 0, 0));
    assert_eq!(bucket_start(at(15, 0, 0, 1)), at(15, 0, 0, 0));
    assert_eq!(bucket_start(at(15, 14, 59, 999)), at(15, 0, 0, 0));
    assert_eq!(bucket_start(at(15, 15, 0, 0)), at(15, 15, 0, 0));
    let before_1970 = Utc.with_ymd_and_hms(1969, 12, 31, 23, 59, 0).unwrap();
    assert_eq!(
        bucket_start(before_1970),
        Utc.with_ymd_and_hms(1969, 12, 31, 23, 45, 0).unwrap(),
        "floored, not truncated towards zero"
    );
}

#[test]
fn a_sign_in_counts_in_the_bucket_of_when_it_happened() {
    let received = at(16, 3, 0, 0);
    let happened = at(15, 59, 30, 0).timestamp_millis();
    let attempt = login_attempt(
        &keycloak("LOGIN", Some("user-1"), Some(happened)),
        Some(AREA),
        received,
    )
    .expect("a sign-in counts");
    assert_eq!(
        attempt,
        LoginAttempt {
            tenant_id: Uuid::parse_str(TENANT).unwrap(),
            election_event_id: Uuid::parse_str(EVENT).unwrap(),
            bucket_start: at(15, 45, 0, 0),
            event_type: "LOGIN".to_string(),
            registration: Registration::Registered,
            area_id: Some(Uuid::parse_str(AREA).unwrap()),
        }
    );
    // Sent before the listener said when, it counts when it was received.
    let untimed = login_attempt(&keycloak("LOGIN", Some("user-1"), None), None, received)
        .expect("a sign-in counts");
    assert_eq!(untimed.bucket_start, at(16, 0, 0, 0));
    // A time no calendar holds counts when it was received too.
    let unreadable = login_attempt(
        &keycloak("LOGIN", Some("user-1"), Some(i64::MAX)),
        None,
        received,
    )
    .expect("a sign-in counts");
    assert_eq!(unreadable.bucket_start, at(16, 0, 0, 0));
    // Nor does the listener's stand-in for no time.
    let unset = login_attempt(&keycloak("LOGIN", Some("user-1"), Some(0)), None, received)
        .expect("a sign-in counts");
    assert_eq!(unset.bucket_start, at(16, 0, 0, 0));
}

#[test]
fn an_attempt_without_an_account_counts_as_unregistered_and_in_no_area() {
    let received = at(9, 0, 0, 0);
    for user_id in [None, Some("null"), Some(""), Some("  ")] {
        let attempt = login_attempt(
            &keycloak("LOGIN_ERROR", user_id, None),
            Some(AREA),
            received,
        )
        .expect("a failed sign-in counts");
        assert_eq!(
            (attempt.registration, attempt.area_id),
            (Registration::Unregistered, None),
            "{user_id:?}"
        );
    }
    for area in [
        None,
        Some(""),
        Some("not-an-area"),
        Some("00000000-0000-0000-0000-000000000000"),
    ] {
        let attempt = login_attempt(&keycloak("LOGIN", Some("user-1"), None), area, received)
            .expect("a sign-in counts");
        assert_eq!(
            (attempt.registration, attempt.area_id),
            (Registration::Registered, None),
            "{area:?}"
        );
    }
}

#[test]
fn only_keycloak_events_of_a_real_event_are_counted() {
    let received = at(9, 0, 0, 0);
    let internal = LogEventInput {
        message_type: LogMessageType::Internal,
        ..keycloak("LOGIN", Some("user-1"), None)
    };
    assert_eq!(login_attempt(&internal, None, received), None);
    for event_type in ["", "null", "login", "LOGIN ERROR", &"A".repeat(65)] {
        assert_eq!(
            login_attempt(&keycloak(event_type, Some("user-1"), None), None, received),
            None,
            "{event_type:?}"
        );
    }
    let no_event = LogEventInput {
        election_event_id: "null".to_string(),
        ..keycloak("LOGIN", Some("user-1"), None)
    };
    assert_eq!(login_attempt(&no_event, None, received), None);
    let no_tenant = LogEventInput {
        tenant_id: "".to_string(),
        ..keycloak("LOGIN", Some("user-1"), None)
    };
    assert_eq!(login_attempt(&no_tenant, None, received), None);
    assert!(login_attempt(
        &keycloak(&"A".repeat(64), Some("user-1"), None),
        None,
        received
    )
    .is_some());
}

#[test]
fn the_event_time_is_optional_on_the_wire_and_leaves_older_messages_as_they_were() {
    let legacy = r#"{"tenant_id":"t","election_event_id":"e","message_type":"LOGIN","user_id":"null","username":"u","body":"null"}"#;
    let input: LogEventInput = serde_json::from_str(legacy).unwrap();
    assert_eq!(input.event_time_ms, None);
    let written = serde_json::to_value(&input).unwrap();
    assert!(
        written.get("event_time_ms").is_none(),
        "a message without a time is written, and so hashed, as before: {written}"
    );
    let legacy_value: serde_json::Value = serde_json::from_str(legacy).unwrap();
    assert_eq!(written, legacy_value);

    let timed = r#"{"tenant_id":"t","election_event_id":"e","message_type":"LOGIN","user_id":"u1","username":"u","body":"null","event_time_ms":1778511600123}"#;
    let input: LogEventInput = serde_json::from_str(timed).unwrap();
    assert_eq!(input.event_time_ms, Some(1_778_511_600_123));
    assert_eq!(
        serde_json::to_value(&input).unwrap()["event_time_ms"],
        serde_json::json!(1_778_511_600_123_i64)
    );
    let null_time = r#"{"tenant_id":"t","election_event_id":"e","message_type":"LOGIN","user_id":"u1","username":"u","body":"null","event_time_ms":null}"#;
    let input: LogEventInput = serde_json::from_str(null_time).unwrap();
    assert_eq!(input.event_time_ms, None);
}

/// A tenant with an election event, committed.
async fn event(pool: &Pool, seed: u32) -> (Uuid, Uuid) {
    let tenant = Uuid::parse_str(&format!("{seed:08x}-0000-4000-8000-000000000001")).unwrap();
    let election_event =
        Uuid::parse_str(&format!("{seed:08x}-0000-4000-8000-000000000002")).unwrap();
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &format!("tenant-{tenant}")],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&election_event, &tenant],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    (tenant, election_event)
}

async fn remove(pool: &Pool, (tenant, election_event): (Uuid, Uuid)) {
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    delete_election_event(&tx, &tenant.to_string(), &election_event.to_string())
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

fn attempt(
    (tenant_id, election_event_id): (Uuid, Uuid),
    bucket: DateTime<Utc>,
    event_type: &str,
    registration: Registration,
    area_id: Option<Uuid>,
) -> LoginAttempt {
    LoginAttempt {
        tenant_id,
        election_event_id,
        bucket_start: bucket,
        event_type: event_type.to_string(),
        registration,
        area_id,
    }
}

fn delivery(n: u32) -> String {
    format!("{n:064x}")
}

/// Each counter of the event, in key order.
async fn counters(
    tx: &Transaction<'_>,
    (tenant, election_event): (Uuid, Uuid),
) -> Vec<(DateTime<Utc>, String, String, Option<Uuid>, i64)> {
    tx.query(
        "SELECT bucket_start, event_type, registration, area_id, attempts
         FROM sequent_backend.monitoring_login_counter
         WHERE tenant_id = $1 AND election_event_id = $2
         ORDER BY bucket_start, event_type, registration, area_key",
        &[&tenant, &election_event],
    )
    .await
    .unwrap()
    .iter()
    .map(|row| (row.get(0), row.get(1), row.get(2), row.get(3), row.get(4)))
    .collect()
}

/// Counts one batch: how many of its deliveries counted.
async fn count(tx: &Transaction<'_>, batch: &[(u32, &LoginAttempt)]) -> u64 {
    let batch: Vec<(String, LoginAttempt)> = batch
        .iter()
        .map(|(n, attempt)| (delivery(*n), (*attempt).clone()))
        .collect();
    count_login_attempts(tx, &batch).await.unwrap()
}

#[tokio::test]
async fn a_delivery_counts_once_however_often_it_is_delivered() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let area = Uuid::parse_str(AREA).unwrap();
    let mut client = pool.get().await.unwrap();
    let bucket = at(10, 0, 0, 0);
    let sign_in = attempt(event, bucket, "LOGIN", Registration::Registered, Some(area));

    let tx = client.transaction().await.unwrap();
    assert_eq!(
        count(&tx, &[(1, &sign_in), (1, &sign_in)]).await,
        1,
        "twice in one batch"
    );
    assert_eq!(
        count(&tx, &[(1, &sign_in)]).await,
        0,
        "and again in the transaction"
    );
    tx.commit().await.unwrap();

    let tx = client.transaction().await.unwrap();
    let unassigned = attempt(event, bucket, "LOGIN", Registration::Registered, None);
    let failed = attempt(
        event,
        bucket,
        "LOGIN_ERROR",
        Registration::Unregistered,
        None,
    );
    let later = attempt(
        event,
        at(10, 15, 0, 0),
        "LOGIN",
        Registration::Registered,
        Some(area),
    );
    assert_eq!(
        count(
            &tx,
            &[
                (1, &sign_in),
                (2, &sign_in),
                (3, &unassigned),
                (4, &failed),
                (5, &failed),
                (6, &later),
            ]
        )
        .await,
        5,
        "all but the one redelivered later"
    );
    assert_eq!(
        counters(&tx, event).await,
        vec![
            (bucket, "LOGIN".into(), "REGISTERED".into(), None, 1),
            (bucket, "LOGIN".into(), "REGISTERED".into(), Some(area), 2),
            (bucket, "LOGIN_ERROR".into(), "UNREGISTERED".into(), None, 2),
            (
                at(10, 15, 0, 0),
                "LOGIN".into(),
                "REGISTERED".into(),
                Some(area),
                1
            ),
        ]
    );
    assert_eq!(count(&tx, &[]).await, 0, "an empty batch counts nothing");
    assert_eq!(count(&tx, &[(40, &failed), (41, &failed)]).await, 2);
    assert_eq!(
        counters(&tx, event).await[2],
        (bucket, "LOGIN_ERROR".into(), "UNREGISTERED".into(), None, 4),
        "a batch adds all it counted to a counter"
    );
    tx.commit().await.unwrap();
    remove(&pool, event).await;
}

#[tokio::test]
async fn a_delivery_whose_transaction_rolled_back_counts_when_it_is_delivered_again() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let mut client = pool.get().await.unwrap();
    let sign_in = attempt(
        event,
        at(10, 0, 0, 0),
        "LOGIN",
        Registration::Registered,
        None,
    );

    let tx = client.transaction().await.unwrap();
    assert_eq!(count(&tx, &[(7, &sign_in)]).await, 1);
    tx.rollback().await.unwrap();

    let tx = client.transaction().await.unwrap();
    assert_eq!(count(&tx, &[(7, &sign_in)]).await, 1);
    assert_eq!(counters(&tx, event).await.len(), 1);
    assert_eq!(counters(&tx, event).await[0].4, 1);
    tx.commit().await.unwrap();
    remove(&pool, event).await;
}

async fn total(pool: &Pool, event: (Uuid, Uuid)) -> i64 {
    pool.get()
        .await
        .unwrap()
        .query_one(
            "SELECT coalesce(sum(attempts), 0)::bigint FROM sequent_backend.monitoring_login_counter
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&event.0, &event.1],
        )
        .await
        .unwrap()
        .get(0)
}

#[tokio::test]
async fn two_workers_counting_one_delivery_count_it_once() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let sign_in = attempt(
        event,
        at(10, 0, 0, 0),
        "LOGIN",
        Registration::Registered,
        None,
    );
    let mut first = pool.get().await.unwrap();
    let mut second = pool.get().await.unwrap();
    let one = first.transaction().await.unwrap();
    assert_eq!(count(&one, &[(8, &sign_in)]).await, 1);
    let racing = tokio::spawn({
        let sign_in = sign_in.clone();
        async move {
            let other = second.transaction().await.unwrap();
            let counted = count(&other, &[(8, &sign_in)]).await;
            other.commit().await.unwrap();
            counted
        }
    });
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert!(
        !racing.is_finished(),
        "the second waits on the first's receipt"
    );
    one.commit().await.unwrap();
    assert_eq!(racing.await.unwrap(), 0, "and then finds it counted");
    assert_eq!(total(&pool, event).await, 1);
    remove(&pool, event).await;
}

#[tokio::test]
async fn batches_counting_the_same_counters_in_another_order_do_not_deadlock() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let early = attempt(
        event,
        at(10, 0, 0, 0),
        "LOGIN",
        Registration::Registered,
        None,
    );
    let late = attempt(
        event,
        at(10, 15, 0, 0),
        "LOGIN",
        Registration::Registered,
        None,
    );
    let mut first = pool.get().await.unwrap();
    let mut second = pool.get().await.unwrap();
    let one = first.transaction().await.unwrap();
    assert_eq!(count(&one, &[(20, &early)]).await, 1);
    // The other batch has the late counter first; it waits on the early one
    // before it holds the late one.
    let other = tokio::spawn({
        let (early, late) = (early.clone(), late.clone());
        async move {
            let other = second.transaction().await.unwrap();
            let counted = count(&other, &[(21, &late), (22, &early)]).await;
            other.commit().await.unwrap();
            counted
        }
    });
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert!(
        !other.is_finished(),
        "the other batch waits on the early counter"
    );
    let counted = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        count(&one, &[(23, &late)]),
    )
    .await
    .expect("the late counter is not held");
    assert_eq!(counted, 1);
    one.commit().await.unwrap();
    assert_eq!(other.await.unwrap(), 2);
    assert_eq!(total(&pool, event).await, 4);
    remove(&pool, event).await;
}

#[tokio::test]
async fn a_batch_that_cannot_be_counted_leaves_the_rest_of_its_transaction_alone() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let mut client = pool.get().await.unwrap();
    let sign_in = attempt(
        event,
        at(10, 0, 0, 0),
        "LOGIN",
        Registration::Registered,
        None,
    );
    // An event that is not there: its receipt is refused.
    let elsewhere = attempt(
        (
            event.0,
            Uuid::parse_str("6f1d9d4e-8a8b-4d7e-b5a4-0c9a1f2e3dff").unwrap(),
        ),
        at(10, 0, 0, 0),
        "LOGIN",
        Registration::Registered,
        None,
    );
    let mut tx = client.transaction().await.unwrap();
    tx.execute(
        "UPDATE sequent_backend.election_event SET annotations = '{\"kept\": true}'
         WHERE id = $1",
        &[&event.1],
    )
    .await
    .unwrap();
    let batch = vec![(delivery(30), sign_in.clone()), (delivery(31), elsewhere)];
    assert_eq!(count_login_attempts_apart(&mut tx, &batch).await, 0);
    tx.commit().await.unwrap();

    let kept: Option<String> = client
        .query_one(
            "SELECT annotations->>'kept' FROM sequent_backend.election_event WHERE id = $1",
            &[&event.1],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(
        kept.as_deref(),
        Some("true"),
        "what the transaction wrote besides commits"
    );
    assert_eq!(
        total(&pool, event).await,
        0,
        "and nothing of the batch counted"
    );

    let mut tx = client.transaction().await.unwrap();
    assert_eq!(
        count_login_attempts_apart(&mut tx, &[(delivery(30), sign_in)]).await,
        1,
        "so it counts when it is delivered again"
    );
    tx.commit().await.unwrap();
    assert_eq!(total(&pool, event).await, 1);
    remove(&pool, event).await;
}

#[tokio::test]
async fn receipts_are_kept_for_the_redelivery_window_and_no_longer() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let mut client = pool.get().await.unwrap();
    let sign_in = attempt(
        event,
        at(10, 0, 0, 0),
        "LOGIN",
        Registration::Registered,
        None,
    );
    let tx = client.transaction().await.unwrap();
    assert_eq!(count(&tx, &[(10, &sign_in), (11, &sign_in)]).await, 2);
    tx.execute(
        "UPDATE sequent_backend.monitoring_login_counter_receipt
         SET received_at = now() - interval '8 days'
         WHERE delivery_id = $1",
        &[&delivery(10)],
    )
    .await
    .unwrap();
    let pruned = prune_login_counter_receipts(&tx, chrono::Duration::days(7))
        .await
        .unwrap();
    assert!(pruned >= 1);
    let kept: Vec<String> = tx
        .query(
            "SELECT delivery_id FROM sequent_backend.monitoring_login_counter_receipt
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&event.0, &event.1],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| row.get(0))
        .collect();
    assert_eq!(kept, vec![delivery(11)]);
    assert_eq!(
        counters(&tx, event).await[0].4,
        2,
        "pruning a receipt keeps what it counted"
    );
    tx.commit().await.unwrap();
    remove(&pool, event).await;
}
