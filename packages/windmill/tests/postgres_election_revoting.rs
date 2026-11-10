// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Disabling re-voting against a private migrated Hasura database: the value it
//! writes, the lifecycle notice it queues and each refusal. Every test works in
//! one transaction and rolls it back.
//!
//! Needs `HASURA_DB__*`.

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Transaction;
use serde_json::{json, Value};
use uuid::Uuid;
use windmill::postgres::trusted_write;
use windmill::services::election_revoting::{disable_revoting, RevotingRefusal, REVOTING_DISABLED};

const USER_ID: &str = "0c6f3d1e-6a4b-4c1e-9d7a-3f2b8e5c1a90";
const USERNAME: &str = "administrator";

/// A tenant and an election event in Preparation.
struct Event {
    tenant: String,
    event: String,
}

async fn event(tx: &Transaction<'_>) -> Event {
    let (tenant, event) = (Uuid::new_v4(), Uuid::new_v4());
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &format!("tenant-{tenant}")],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&event, &tenant],
    )
    .await
    .unwrap();
    Event {
        tenant: tenant.to_string(),
        event: event.to_string(),
    }
}

/// An election of `e` with the given number of allowed votes.
async fn election(tx: &Transaction<'_>, e: &Event, allowed: Option<i32>) -> String {
    let id = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id, num_allowed_revotes)
         VALUES ($1, $2::text::uuid, $3::text::uuid, $4)",
        &[&id, &e.tenant, &e.event, &allowed],
    )
    .await
    .unwrap();
    id.to_string()
}

async fn disable(tx: &Transaction<'_>, e: &Event, election: &str) -> anyhow::Result<i32> {
    disable_revoting(tx, &e.tenant, &e.event, election, USER_ID, Some(USERNAME)).await
}

fn refusal(result: anyhow::Result<i32>) -> String {
    result
        .unwrap_err()
        .downcast::<RevotingRefusal>()
        .expect("a refusal")
        .to_string()
}

async fn allowed(tx: &Transaction<'_>, election: &str) -> Option<i32> {
    tx.query_one(
        "SELECT num_allowed_revotes FROM sequent_backend.election WHERE id = $1::text::uuid",
        &[&election],
    )
    .await
    .unwrap()
    .get(0)
}

async fn notices(tx: &Transaction<'_>, e: &Event) -> Vec<Value> {
    tx.query(
        "SELECT details FROM sequent_backend.lifecycle_notice
         WHERE election_event_id = $1::text::uuid AND kind = $2 ORDER BY created_at",
        &[&e.event, &REVOTING_DISABLED],
    )
    .await
    .unwrap()
    .iter()
    .map(|row| row.get(0))
    .collect()
}

#[tokio::test]
async fn an_administrator_disables_revoting_once_and_the_log_names_them() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let e = event(&tx).await;
    let unlimited = election(&tx, &e, Some(0)).await;
    let three = election(&tx, &e, Some(3)).await;

    assert_eq!(disable(&tx, &e, &unlimited).await.unwrap(), 0);
    assert_eq!(disable(&tx, &e, &three).await.unwrap(), 3);
    assert_eq!(allowed(&tx, &unlimited).await, Some(1));
    assert_eq!(allowed(&tx, &three).await, Some(1));
    assert_eq!(
        notices(&tx, &e).await[0],
        json!({
            "election_id": unlimited,
            "num_allowed_revotes": {"from": 0, "to": 1},
            "disabled_by": USER_ID,
            "username": USERNAME,
        })
    );
    let queued: i64 = tx
        .query_one(
            "SELECT count(*) FROM sequent_backend.security_notification
             WHERE election_event_id = $1::text::uuid AND kind = $2",
            &[&e.event, &REVOTING_DISABLED],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(queued, 2);

    assert_eq!(
        refusal(disable(&tx, &e, &unlimited).await),
        "Re-voting is already disabled for this election."
    );
    assert_eq!(notices(&tx, &e).await.len(), 2);
}

#[tokio::test]
async fn an_election_with_a_single_vote_is_refused() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let e = event(&tx).await;
    for value in [None, Some(1)] {
        let single = election(&tx, &e, value).await;
        assert_eq!(
            refusal(disable(&tx, &e, &single).await),
            "Re-voting is already disabled for this election."
        );
        assert_eq!(allowed(&tx, &single).await, value);
    }
    assert!(notices(&tx, &e).await.is_empty());
}

#[tokio::test]
async fn voting_started_on_any_channel_of_the_event_refuses() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let e = event(&tx).await;
    let unlimited = election(&tx, &e, Some(0)).await;
    let kiosk = election(&tx, &e, Some(0)).await;
    // A kiosk opening leaves the event in Preparation.
    trusted_write(&tx).await.unwrap();
    tx.execute(
        "UPDATE sequent_backend.election SET status = '{\"kiosk_voting_status\":\"OPEN\"}'
         WHERE id = $1::text::uuid",
        &[&kiosk],
    )
    .await
    .unwrap();

    assert_eq!(
        refusal(disable(&tx, &e, &unlimited).await),
        "Voting has started on a channel of this election event; re-voting can no longer be disabled."
    );
    assert_eq!(allowed(&tx, &unlimited).await, Some(0));
    assert!(notices(&tx, &e).await.is_empty());
}

#[tokio::test]
async fn outside_preparation_is_refused() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let e = event(&tx).await;
    let unlimited = election(&tx, &e, Some(0)).await;
    trusted_write(&tx).await.unwrap();
    tx.execute(
        "UPDATE sequent_backend.election_lifecycle SET phase = 'execution'
         WHERE election_event_id = $1::text::uuid",
        &[&e.event],
    )
    .await
    .unwrap();

    assert_eq!(
        refusal(disable(&tx, &e, &unlimited).await),
        "This operation requires electoral phase Preparation; current phase is Execution"
    );
    assert_eq!(allowed(&tx, &unlimited).await, Some(0));
}

#[tokio::test]
async fn a_lockdown_refuses() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let e = event(&tx).await;
    let unlimited = election(&tx, &e, Some(0)).await;
    trusted_write(&tx).await.unwrap();
    tx.execute(
        "UPDATE sequent_backend.election_event SET presentation = '{\"locked_down\":\"locked-down\"}'
         WHERE id = $1::text::uuid",
        &[&e.event],
    )
    .await
    .unwrap();

    assert_eq!(
        refusal(disable(&tx, &e, &unlimited).await),
        "The election event is locked down; its re-voting setting can't change."
    );
    assert_eq!(allowed(&tx, &unlimited).await, Some(0));
}

#[tokio::test]
async fn an_election_of_another_event_is_refused() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let e = event(&tx).await;
    let other = event(&tx).await;
    let foreign = election(&tx, &other, Some(0)).await;

    assert_eq!(
        refusal(disable(&tx, &e, &foreign).await),
        "The election is not part of this election event."
    );
    assert_eq!(allowed(&tx, &foreign).await, Some(0));
}
