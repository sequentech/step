// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Opening a tenant's log against the migrated schema and the electoral-log
//! server: its signing key is created once, as a secret of the tenant with no
//! election event, and its board gets one first entry however often it is
//! opened. Key tests write their rows in a transaction and roll it back.

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Transaction;
use electoral_log::messages::message::Message;
use electoral_log::LogQuery;
use std::sync::Once;
use uuid::Uuid;
use voting_crypto::signing::{SigningKey, VerificationKey};
use voting_crypto::RecordDeserialize;
use windmill::services::electoral_log_board::get_protocol_manager_secret_path;
use windmill::services::tenant_log::{
    ensure_tenant_board, get_tenant_board, get_tenant_log_client, tenant_log_signing_key,
    TenantLog, TENANT_LOG_OPENED_DELIVERY_ID,
};

fn use_master_secret() {
    static SET: Once = Once::new();
    SET.call_once(|| std::env::set_var("MASTER_SECRET", "5a".repeat(32)));
}

async fn tenant(tx: &Transaction<'_>) -> String {
    let tenant = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &tenant.to_string()],
    )
    .await
    .unwrap();
    tenant.to_string()
}

#[tokio::test]
async fn the_logs_key_is_created_once_as_a_secret_of_the_tenant() {
    use_master_secret();
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant_id = tenant(&tx).await;
    let board = get_tenant_board(&tenant_id, "test");

    let created = tenant_log_signing_key(&tx, &tenant_id, &board)
        .await
        .unwrap()
        .expect("the tenant exists");
    let read_back = tenant_log_signing_key(&tx, &tenant_id, &board)
        .await
        .unwrap()
        .expect("the tenant exists");
    assert_eq!(
        created.to_b64_string().unwrap(),
        read_back.to_b64_string().unwrap()
    );

    // One secret, of the tenant and of no election event, stored encrypted.
    let rows = tx
        .query(
            "SELECT election_event_id::text, value FROM sequent_backend.secret
             WHERE tenant_id = $1::text::uuid AND key = $2",
            &[&tenant_id, &get_protocol_manager_secret_path(&board)],
        )
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<_, Option<String>>(0), None);
    let stored: Vec<u8> = rows[0].get(1);
    let key = created.to_b64_string().unwrap();
    assert!(!String::from_utf8_lossy(&stored).contains(&key));
}

#[tokio::test]
async fn a_tenant_that_does_not_exist_gets_no_key() {
    use_master_secret();
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let unknown = Uuid::new_v4().to_string();
    let board = get_tenant_board(&unknown, "test");

    assert!(tenant_log_signing_key(&tx, &unknown, &board)
        .await
        .unwrap()
        .is_none());
    let secrets: i64 = tx
        .query_one(
            "SELECT count(*) FROM sequent_backend.secret WHERE key = $1",
            &[&get_protocol_manager_secret_path(&board)],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(secrets, 0);
    // A tenant id that is not a UUID is an error, not a missing tenant.
    assert!(tenant_log_signing_key(&tx, "not-a-uuid", &board)
        .await
        .is_err());
}

/// Each tenant has its own key: a log's entries verify with no other tenant's.
#[tokio::test]
async fn two_tenants_get_different_keys() {
    use_master_secret();
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let (first, second) = (tenant(&tx).await, tenant(&tx).await);

    let first_key = tenant_log_signing_key(&tx, &first, &get_tenant_board(&first, "test"))
        .await
        .unwrap()
        .unwrap();
    let second_key = tenant_log_signing_key(&tx, &second, &get_tenant_board(&second, "test"))
        .await
        .unwrap()
        .unwrap();
    assert_ne!(
        first_key.to_b64_string().unwrap(),
        second_key.to_b64_string().unwrap()
    );
}

#[tokio::test]
#[ignore = "requires a disposable ELECTORAL_LOG_PG_* database"]
async fn a_board_is_opened_once_with_one_first_entry() -> anyhow::Result<()> {
    let tenant_id = Uuid::new_v4().to_string();
    let board = get_tenant_board(&tenant_id, "test");
    let key = SigningKey::generate()?;
    let public = VerificationKey::from_sk(&key)?;
    let log = TenantLog::with_signing_key(&tenant_id, &board, &key);
    let client = get_tenant_log_client().await?;
    assert!(!client.has_board(&board).await?);

    ensure_tenant_board(&client, &log).await?;
    ensure_tenant_board(&client, &log).await?;

    let entries = client.query(&board, &LogQuery::default()).await?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].statement_kind, "TenantLogOpened");
    Message::record_deserialize(&entries[0].message)?.verify(&public)?;

    // A board left without its first entry gets it the next time it is opened.
    client.delete_board(&board).await?;
    client.create_board(&board).await?;
    assert_eq!(client.count(&board, &LogQuery::default()).await?, 0);
    ensure_tenant_board(&client, &log).await?;
    assert_eq!(client.count(&board, &LogQuery::default()).await?, 1);
    // Storing the first entry again, as a second worker would, adds nothing.
    let mut again = log.opened_entry()?;
    assert_eq!(again.delivery_id, TENANT_LOG_OPENED_DELIVERY_ID);
    again.message.created += 1;
    client.append(&board, &[again]).await?;
    assert_eq!(client.count(&board, &LogQuery::default()).await?, 1);

    client.delete_board(&board).await?;
    Ok(())
}
