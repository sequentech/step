// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Database invariants used equally by Hasura writes and server imports.
#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Transaction;
use serde_json::{json, Value};
use tokio_postgres::error::SqlState;
use uuid::Uuid;

async fn event(tx: &Transaction<'_>, zones: Value) -> (Uuid, Uuid) {
    let tenant = Uuid::new_v4();
    let event = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &tenant.to_string()],
    )
    .await
    .unwrap();
    tx.execute("INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol, presentation) VALUES ($1, $2, 'RSA256', $3)", &[&event, &tenant, &zones]).await.unwrap();
    (tenant, event)
}

async fn rejects(
    tx: &Transaction<'_>,
    sql: &str,
    params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
) {
    tx.batch_execute("SAVEPOINT rejected_zone").await.unwrap();
    let err = tx.execute(sql, params).await.unwrap_err();
    assert_eq!(err.code(), Some(&SqlState::CHECK_VIOLATION), "{err}");
    tx.batch_execute("ROLLBACK TO SAVEPOINT rejected_zone; RELEASE SAVEPOINT rejected_zone")
        .await
        .unwrap();
}

#[tokio::test]
async fn event_zones_are_canonical_and_invalid_settings_are_rejected() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let (_, event) = event(&tx, json!({"timezones": {"configured": ["UTC", "US/Eastern", "America/New_York"], "primary": "US/Eastern"}})).await;
    let presentation: Value = tx
        .query_one(
            "SELECT presentation FROM sequent_backend.election_event WHERE id = $1",
            &[&event],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(
        presentation["timezones"]["configured"],
        json!(["UTC", "America/New_York"])
    );
    assert_eq!(
        presentation["timezones"]["primary"],
        json!("America/New_York")
    );
    for zones in [
        json!({"configured": [], "primary": "UTC"}),
        json!({"configured": ["UTC"], "primary": "Asia/Manila"}),
        json!({"configured": ["Imaginary/Place"], "primary": "Imaginary/Place"}),
        json!({"configured": ["EST"], "primary": "EST"}),
        json!({"configured": [12], "primary": "UTC"}),
        json!({"configured": "UTC", "primary": "UTC"}),
        json!({"configured": ["UTC"], "primary": null}),
        json!({"configured": ["UTC"], "primary": "UTC", "logs": null}),
    ] {
        rejects(&tx, "UPDATE sequent_backend.election_event SET presentation = jsonb_build_object('timezones', $2::jsonb) WHERE id = $1", &[&event, &zones]).await;
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn post_membership_and_used_zone_removal_are_enforced_for_raw_writes() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let (tenant, event) = event(
        &tx,
        json!({"timezones": {"configured": ["UTC", "Asia/Kolkata"], "primary": "UTC"}}),
    )
    .await;
    let post = Uuid::new_v4();
    tx.execute("INSERT INTO sequent_backend.election (id, tenant_id, election_event_id, presentation) VALUES ($1, $2, $3, $4)", &[&post, &tenant, &event, &json!({"timezone": "Asia/Calcutta"})]).await.unwrap();
    let presentation: Value = tx
        .query_one(
            "SELECT presentation FROM sequent_backend.election WHERE id = $1",
            &[&post],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(presentation["timezone"], json!("Asia/Kolkata"));
    rejects(&tx, "UPDATE sequent_backend.election SET presentation = '{\"timezone\": \"Europe/London\"}' WHERE id = $1", &[&post]).await;
    rejects(&tx, "UPDATE sequent_backend.election_event SET presentation = '{\"timezones\": {\"configured\": [\"UTC\"], \"primary\": \"UTC\"}}' WHERE id = $1", &[&event]).await;
    rejects(
        &tx,
        "UPDATE sequent_backend.election_event SET presentation = '{}' WHERE id = $1",
        &[&event],
    )
    .await;
    tx.execute(
        "UPDATE sequent_backend.election SET presentation = '{}' WHERE id = $1",
        &[&post],
    )
    .await
    .unwrap();
    tx.execute(
        "UPDATE sequent_backend.election_event SET presentation = '{}' WHERE id = $1",
        &[&event],
    )
    .await
    .unwrap();
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn legacy_absent_zones_keep_utc_and_unrelated_updates_work() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let (tenant, event) = event(&tx, json!({"timezones": null})).await;
    let post = Uuid::new_v4();
    tx.execute("INSERT INTO sequent_backend.election (id, tenant_id, election_event_id, presentation) VALUES ($1, $2, $3, '{\"timezone\": \"UTC\"}')", &[&post, &tenant, &event]).await.unwrap();
    tx.execute("UPDATE sequent_backend.election_event SET presentation = '{\"unrelated\": true}' WHERE id = $1", &[&event]).await.unwrap();
    tx.execute("UPDATE sequent_backend.election SET presentation = '{\"timezone\": null, \"unrelated\": true}' WHERE id = $1", &[&post]).await.unwrap();
    rejects(&tx, "UPDATE sequent_backend.election SET presentation = '{\"timezone\": \"Asia/Manila\"}' WHERE id = $1", &[&post]).await;
    tx.rollback().await.unwrap();
}
