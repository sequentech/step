// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The database gate before a queued enrollment task contacts Keycloak.
#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Transaction;
use serde_json::json;
use uuid::Uuid;
use windmill::tasks::manage_election_event_enrollment::load_enrollment_task;

async fn fixture(tx: &Transaction<'_>, processor: &str) -> (String, String, String) {
    let tenant = Uuid::new_v4();
    let event = Uuid::new_v4();
    let row = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &tenant.to_string()],
    )
    .await
    .unwrap();
    tx.execute("INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol) VALUES ($1, $2, 'RSA256')", &[&event, &tenant]).await.unwrap();
    tx.execute("INSERT INTO sequent_backend.scheduled_event (id, tenant_id, election_event_id, event_processor, cron_config, created_by) VALUES ($1, $2, $3, $4, $5, 'test')", &[&row, &tenant, &event, &processor, &json!({"scheduled_date": "2020-01-01T00:00:00Z"})]).await.unwrap();
    (tenant.to_string(), event.to_string(), row.to_string())
}

#[tokio::test]
async fn archived_event_cannot_change_enrollment() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let (tenant, event, row) = fixture(&tx, "START_ENROLLMENT_PERIOD").await;
    tx.execute(
        "UPDATE sequent_backend.election_event SET is_archived = true WHERE id = $1",
        &[&Uuid::parse_str(&event).unwrap()],
    )
    .await
    .unwrap();
    assert!(
        load_enrollment_task(&tx, &tenant, &event, &row, chrono::Utc::now())
            .await
            .unwrap()
            .is_none()
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_non_enrollment_processor_is_refused_instead_of_closing_registration() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let (tenant, event, row) = fixture(&tx, "END_VOTING_PERIOD").await;
    let err = load_enrollment_task(&tx, &tenant, &event, &row, chrono::Utc::now())
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("is not an enrollment processor"),
        "{err}"
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn active_due_enrollment_is_loaded_but_a_future_or_inactive_row_is_not() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let (tenant, event, row) = fixture(&tx, "START_ENROLLMENT_PERIOD").await;
    assert!(
        load_enrollment_task(&tx, &tenant, &event, &row, chrono::Utc::now())
            .await
            .unwrap()
            .is_some()
    );
    // A loaded task holds the writer lock until its effect transaction ends.
    let mut competing_client = pool.get().await.unwrap();
    let competing = competing_client.transaction().await.unwrap();
    let acquired: bool = competing
        .query_one(
            "SELECT pg_try_advisory_xact_lock(hashtextextended($1, 0))",
            &[&format!("signing-event:{tenant}:{event}")],
        )
        .await
        .unwrap()
        .get(0);
    assert!(
        !acquired,
        "The task must serialize with enrollment policy writers"
    );
    competing.rollback().await.unwrap();
    tx.execute(
        "UPDATE sequent_backend.scheduled_event SET cron_config = $2 WHERE id = $1",
        &[
            &Uuid::parse_str(&row).unwrap(),
            &json!({"scheduled_date": "2099-01-01T00:00:00Z"}),
        ],
    )
    .await
    .unwrap();
    assert!(
        load_enrollment_task(&tx, &tenant, &event, &row, chrono::Utc::now())
            .await
            .unwrap()
            .is_none()
    );
    tx.execute(
        "UPDATE sequent_backend.scheduled_event SET stopped_at = now() WHERE id = $1",
        &[&Uuid::parse_str(&row).unwrap()],
    )
    .await
    .unwrap();
    assert!(
        load_enrollment_task(&tx, &tenant, &event, &row, chrono::Utc::now())
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_row_from_a_different_event_cannot_change_the_requested_realm() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let (tenant, event, row) = fixture(&tx, "START_ENROLLMENT_PERIOD").await;
    assert!(load_enrollment_task(
        &tx,
        &tenant,
        &Uuid::new_v4().to_string(),
        &row,
        chrono::Utc::now()
    )
    .await
    .is_err());
    assert!(load_enrollment_task(
        &tx,
        &Uuid::new_v4().to_string(),
        &event,
        &row,
        chrono::Utc::now()
    )
    .await
    .is_err());
    tx.rollback().await.unwrap();
}
