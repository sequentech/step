// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Election event ids are unique across tenants: creating or importing an
//! event checks that the id is free, and holds it until the transaction ends.

#![recursion_limit = "256"]

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Transaction;
use serde_json::json;
use tokio_postgres::error::SqlState;
use uuid::Uuid;
use windmill::postgres::election_event::{election_event_id_exists, lock_election_event_id};
use windmill::services::import::import_election_event::ensure_new_election_event;

async fn insert_event(tx: &Transaction<'_>, tenant: Uuid, event: Uuid) {
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &format!("tenant-{tenant}")],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election_event
             (id, tenant_id, encryption_protocol, status, presentation)
         VALUES ($1, $2, 'RSA256', $3, $4)",
        &[
            &event,
            &tenant,
            &json!({"is_published": true, "voting_status": "NOT_STARTED"}),
            &json!({}),
        ],
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn an_id_used_in_any_tenant_exists() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let used = Uuid::new_v4();
    insert_event(&tx, Uuid::new_v4(), used).await;

    assert!(election_event_id_exists(&tx, &used.to_string())
        .await
        .unwrap());
    assert!(!election_event_id_exists(&tx, &Uuid::new_v4().to_string())
        .await
        .unwrap());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn an_id_that_is_not_a_uuid_is_refused() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();

    assert!(election_event_id_exists(&tx, "not-a-uuid").await.is_err());
    assert!(lock_election_event_id(&tx, "not-a-uuid").await.is_err());
    assert!(ensure_new_election_event(&tx, "not-a-uuid").await.is_err());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn only_an_unused_id_is_new() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let used = Uuid::new_v4();
    insert_event(&tx, Uuid::new_v4(), used).await;

    let error = ensure_new_election_event(&tx, &used.to_string())
        .await
        .expect_err("an id used in another tenant is not new");
    assert!(format!("{error:#}").contains("already exists"), "{error:#}");
    ensure_new_election_event(&tx, &Uuid::new_v4().to_string())
        .await
        .unwrap();
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn the_id_stays_locked_until_the_transaction_ends() {
    let pool = schema::pool().await;
    let mut first_client = pool.get().await.unwrap();
    let mut second_client = pool.get().await.unwrap();
    let id = Uuid::new_v4().to_string();

    let first = first_client.transaction().await.unwrap();
    ensure_new_election_event(&first, &id).await.unwrap();

    let second = second_client.transaction().await.unwrap();
    second
        .batch_execute("SET LOCAL lock_timeout = '200ms'")
        .await
        .unwrap();
    let error = ensure_new_election_event(&second, &id)
        .await
        .expect_err("a second setup of the same id waits for the first");
    assert_eq!(
        error
            .downcast_ref::<tokio_postgres::Error>()
            .and_then(tokio_postgres::Error::code),
        Some(&SqlState::LOCK_NOT_AVAILABLE),
        "{error:#}"
    );

    second.rollback().await.unwrap();
    first.rollback().await.unwrap();
}
