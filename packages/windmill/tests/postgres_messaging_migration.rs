// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Migration `1791200000000_messaging_second_pass` against rows the first
//! release left behind. This binary's database holds only this test's
//! rows: the migration runs down and up, and its writes are committed.

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Object;
use messaging::providers::Endpoints;
use uuid::Uuid;
use windmill::services::messaging::dispatch::Dispatcher;
use windmill::services::messaging::reconcile::reconcile_messages;

const UP: &str = include_str!(
    "../../../hasura/migrations/backend-db/1791200000000_messaging_second_pass/up.sql"
);
const DOWN: &str = include_str!(
    "../../../hasura/migrations/backend-db/1791200000000_messaging_second_pass/down.sql"
);

async fn run(client: &mut Object, sql: &str) {
    let tx = client.transaction().await.unwrap();
    tx.batch_execute(sql).await.unwrap();
    tx.commit().await.unwrap();
}

async fn state(client: &Object, key: &str) -> (String, bool) {
    let row = client
        .query_one(
            "SELECT state, next_retry_at IS NULL FROM sequent_backend.message
             WHERE logical_key = $1",
            &[&key],
        )
        .await
        .unwrap();
    (row.get(0), row.get(1))
}

#[tokio::test]
async fn the_second_pass_migration_keeps_first_release_rows_safe() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tenant = Uuid::new_v4();
    let event = Uuid::new_v4();
    client
        .execute(
            "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
            &[&tenant, &format!("tenant-{tenant}")],
        )
        .await
        .unwrap();
    client
        .execute(
            "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
             VALUES ($1, $2, 'RSA256')",
            &[&event, &tenant],
        )
        .await
        .unwrap();

    run(&mut client, DOWN).await;
    // A bulk notice the first release recorded as queued before calling
    // the provider, when a deploy stopped its worker: it may have been
    // sent.
    client
        .execute(
            r#"
            INSERT INTO sequent_backend.message
                (tenant_id, election_event_id, channel, direction, purpose, template_alias,
                 masked_destination, destination_digest, logical_key, attempt, state,
                 created_at)
            VALUES ($1, $2, 'SMS', 'OUTBOUND', 'NOTICE', 'reminder', '-', '-',
                    'send-template:upgrade:voter-1', 1, 'QUEUED',
                    now() - interval '5 minutes')
            "#,
            &[&tenant, &event],
        )
        .await
        .unwrap();
    // An attempt the first release already logged (or never will).
    client
        .execute(
            r#"
            INSERT INTO sequent_backend.message
                (tenant_id, election_event_id, channel, direction, purpose,
                 masked_destination, destination_digest, logical_key, attempt, state)
            VALUES ($1, $2, 'SMS', 'OUTBOUND', 'NOTICE', '-', '-',
                    'keycloak:upgrade-logged', 1, 'ACCEPTED')
            "#,
            &[&tenant, &event],
        )
        .await
        .unwrap();
    run(&mut client, UP).await;

    // R2-08: rows from before the upgrade count as logged; the sweep never
    // writes them to the electoral log again.
    let logged: bool = client
        .query_one(
            "SELECT logged_at IS NOT NULL FROM sequent_backend.message
             WHERE logical_key = 'keycloak:upgrade-logged'",
            &[],
        )
        .await
        .unwrap()
        .get(0);
    assert!(logged);
    // New rows start unlogged, and a row may have no channel.
    client
        .execute(
            r#"
            INSERT INTO sequent_backend.message
                (tenant_id, election_event_id, channel, direction, purpose,
                 masked_destination, destination_digest, logical_key, attempt, state)
            VALUES ($1, $2, NULL, 'OUTBOUND', 'NOTICE', '-', '-',
                    'send-template:upgrade:nobody', 1, 'FAILED')
            "#,
            &[&tenant, &event],
        )
        .await
        .unwrap();

    // R2-06: maybe sent, so never sent again: UNKNOWN, no retry.
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    reconcile_messages(&mut client, &dispatcher).await.unwrap();
    assert_eq!(
        state(&client, "send-template:upgrade:voter-1").await,
        ("UNKNOWN".to_string(), true)
    );

    // Down puts the channel-less row back on email, as the first release
    // needs a channel, and up again works.
    run(&mut client, DOWN).await;
    let channel: String = client
        .query_one(
            "SELECT channel FROM sequent_backend.message
             WHERE logical_key = 'send-template:upgrade:nobody'",
            &[],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(channel, "EMAIL");
    run(&mut client, UP).await;
}
