// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Per-tenant databases against a real server. Needs the `ELECTORAL_LOG_PG_*`
//! connection variables and `ELECTORAL_LOG_PG_PROVISIONING_USER` and `_PASSWORD`.
//! It creates a database for a random tenant and drops it at the end.

use anyhow::Result;
use electoral_log::ports::ElectoralLogStore;
use electoral_log::{
    adapters::router::{tenant_database_name, tenant_prefix, StoreRouter, LAYOUT_ENV},
    ElectoralLogMessage, LogEntry, LogQuery,
};

const SLUG: &str = "routertest";

fn entry(delivery: &str) -> LogEntry {
    LogEntry {
        delivery_id: delivery.into(),
        message: ElectoralLogMessage {
            id: 0,
            created: 1,
            sender_pk: "sender".into(),
            statement_timestamp: 1,
            statement_kind: "CastVote".into(),
            message: vec![1, 2, 3],
            version: "2".into(),
            user_id: Some("voter".into()),
            username: None,
            election_id: None,
            area_id: None,
            ballot_id: None,
        },
    }
}

#[tokio::test]
#[ignore = "requires an electoral-log server and a provisioning role"]
async fn tenant_boards_get_their_own_database_and_existing_boards_stay_shared() -> Result<()> {
    std::env::set_var(LAYOUT_ENV, "per-tenant");
    std::env::set_var("ENV_SLUG", SLUG);
    let router = StoreRouter::from_env()?;
    let tenant = uuid::Uuid::new_v4().to_string();
    let prefix = tenant_prefix(&tenant);
    let shared_name = router
        .shared()
        .client()
        .await?
        .query_one("SELECT current_database()", &[])
        .await?
        .get::<_, String>(0);
    let database = tenant_database_name(&shared_name, SLUG, &prefix)?;
    let board = format!("{SLUG}tenant{prefix}event{}", uuid::Uuid::new_v4().simple());
    let legacy = format!("{SLUG}tenant{prefix}event{}", uuid::Uuid::new_v4().simple());

    // A board that already exists in the shared database stays there.
    router.shared().create_board(&legacy).await?;

    router.create_board(&board).await?;
    router
        .append(
            &board,
            &mut [entry("a"), entry("b")].into_iter().map(anyhow::Ok),
        )
        .await?;
    router
        .append(&legacy, &mut [entry("c")].into_iter().map(anyhow::Ok))
        .await?;

    let all = LogQuery::default();
    assert_eq!(router.count(&board, &all).await?, 2);
    assert_eq!(router.count(&legacy, &all).await?, 1);
    assert!(!router.shared().has_board(&board).await?);
    let tenant_store = router.database_store(&database).await?;
    assert!(tenant_store.has_board(&board).await?);
    assert!(!tenant_store.has_board(&legacy).await?);
    assert!(router.tenant_databases().await?.contains(&database));

    // Provisioning again is harmless.
    router.provision_tenant(&tenant).await?;

    router.delete_board(&board).await?;
    router.delete_board(&legacy).await?;
    drop(tenant_store);
    drop(router);
    let shared = StoreRouter::from_env()?.shared();
    shared
        .client()
        .await?
        .batch_execute(&format!("DROP DATABASE \"{database}\" WITH (FORCE)"))
        .await?;
    Ok(())
}
