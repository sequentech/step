// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Messaging persistence against the real schema: sent-message statistics,
//! sending accounts, the message ledger and Messenger links.

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Transaction;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use uuid::Uuid;
use windmill::services::election_event_statistics::update_election_event_statistics;
use windmill::services::election_statistics::update_election_statistics;

/// A fixed v4 UUID per test (`seed`) and call (`n`).
fn id(seed: u32, n: u32) -> Uuid {
    Uuid::parse_str(&format!("{seed:08x}-0000-4000-8000-{n:012x}")).unwrap()
}

struct World {
    tenant: Uuid,
    event: Uuid,
    election: Uuid,
}

async fn world(tx: &Transaction<'_>, seed: u32) -> World {
    let world = World {
        tenant: id(seed, 1),
        event: id(seed, 2),
        election: id(seed, 3),
    };
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&world.tenant, &format!("tenant-{}", world.tenant)],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&world.event, &world.tenant],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id)
         VALUES ($1, $2, $3)",
        &[&world.election, &world.tenant, &world.event],
    )
    .await
    .unwrap();
    world
}

fn counters(pairs: &[(&str, i64)]) -> BTreeMap<String, i64> {
    pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

#[tokio::test]
async fn statistics_add_per_channel_counters_and_keep_other_keys() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0x5747).await;
    tx.execute(
        "UPDATE sequent_backend.election_event
         SET statistics = '{\"num_emails_sent\": 4, \"num_users\": 9}'::jsonb
         WHERE id = $1",
        &[&world.event],
    )
    .await
    .unwrap();

    let tenant = world.tenant.to_string();
    let event = world.event.to_string();
    update_election_event_statistics(
        &tx,
        &tenant,
        &event,
        &counters(&[("num_emails_sent", 2), ("num_whatsapp_sent", 3)]),
    )
    .await
    .unwrap();
    update_election_event_statistics(&tx, &tenant, &event, &counters(&[("num_whatsapp_sent", 1)]))
        .await
        .unwrap();
    update_election_statistics(
        &tx,
        &tenant,
        &event,
        &world.election.to_string(),
        &counters(&[("num_viber_sent", 5)]),
    )
    .await
    .unwrap();

    let event_statistics: Value = tx
        .query_one(
            "SELECT statistics FROM sequent_backend.election_event WHERE id = $1",
            &[&world.event],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(
        event_statistics,
        json!({"num_emails_sent": 6, "num_users": 9, "num_whatsapp_sent": 4})
    );
    let election_statistics: Value = tx
        .query_one(
            "SELECT statistics FROM sequent_backend.election WHERE id = $1",
            &[&world.election],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(election_statistics["num_viber_sent"], json!(5));
    tx.rollback().await.unwrap();
}
