// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The tenant's Google service account key against the migrated schema: it
//! lives in the vault, not in the tenant settings, and a tenant import moves
//! the key of an older export there. Every test writes its own rows in a
//! transaction and rolls it back.

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Transaction;
use serde_json::{json, Value};
use std::sync::Once;
use uuid::Uuid;
use windmill::postgres::tenant::remove_tenant_setting;
use windmill::services::google_meet::{
    service_account_key_secret_name, store_service_account_key, GoogleMeetError,
};
use windmill::services::import::import_tenant::process_record;
use windmill::services::vault::read_secret;

const CLIENT_EMAIL: &str = "meet@project.iam.gserviceaccount.com";

fn use_master_secret() {
    static SET: Once = Once::new();
    SET.call_once(|| std::env::set_var("MASTER_SECRET", "5a".repeat(32)));
}

fn key(client_email: &str) -> Value {
    json!({
        "type": "service_account",
        "private_key": "-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----\n",
        "client_email": client_email,
        "token_uri": "https://oauth2.googleapis.com/token",
    })
}

async fn tenant_with_settings(tx: &Transaction<'_>, settings: &Value) -> String {
    let tenant = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug, settings) VALUES ($1, $2, $3)",
        &[&tenant, &tenant.to_string(), settings],
    )
    .await
    .unwrap();
    tenant.to_string()
}

async fn settings_of(tx: &Transaction<'_>, tenant: &str) -> Value {
    tx.query_one(
        "SELECT settings FROM sequent_backend.tenant WHERE id = $1::text::uuid",
        &[&tenant],
    )
    .await
    .unwrap()
    .get(0)
}

async fn stored_key(tx: &Transaction<'_>, tenant: &str) -> Option<Value> {
    read_secret(tx, tenant, None, &service_account_key_secret_name(tenant))
        .await
        .unwrap()
        .map(|stored| serde_json::from_str(&stored).unwrap())
}

#[tokio::test]
async fn store_service_account_key_moves_the_key_from_the_settings_to_the_vault() {
    use_master_secret();
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = tenant_with_settings(
        &tx,
        &json!({"gapi_key": key("old@project.iam.gserviceaccount.com"), "gapi_email": "organizer@example.org"}),
    )
    .await;

    let stored = store_service_account_key(&tx, &tenant, key(CLIENT_EMAIL))
        .await
        .unwrap();

    assert_eq!(stored.client_email, CLIENT_EMAIL);
    assert_eq!(stored_key(&tx, &tenant).await, Some(key(CLIENT_EMAIL)));
    assert_eq!(
        settings_of(&tx, &tenant).await,
        json!({"gapi_email": "organizer@example.org"})
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn store_service_account_key_keeps_the_keys_of_two_tenants_apart() {
    use_master_secret();
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let first = tenant_with_settings(&tx, &json!({})).await;
    let second = tenant_with_settings(&tx, &json!({})).await;

    store_service_account_key(&tx, &first, key("first@project.iam.gserviceaccount.com"))
        .await
        .unwrap();
    store_service_account_key(&tx, &second, key(CLIENT_EMAIL))
        .await
        .unwrap();

    assert_eq!(
        stored_key(&tx, &first).await,
        Some(key("first@project.iam.gserviceaccount.com"))
    );
    assert_eq!(stored_key(&tx, &second).await, Some(key(CLIENT_EMAIL)));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn store_service_account_key_replaces_the_previous_key() {
    use_master_secret();
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = tenant_with_settings(&tx, &json!({})).await;

    store_service_account_key(&tx, &tenant, key("first@project.iam.gserviceaccount.com"))
        .await
        .unwrap();
    store_service_account_key(&tx, &tenant, key(CLIENT_EMAIL))
        .await
        .unwrap();

    assert_eq!(stored_key(&tx, &tenant).await, Some(key(CLIENT_EMAIL)));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn store_service_account_key_refuses_a_key_it_cannot_use_and_writes_nothing() {
    use_master_secret();
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let settings = json!({"gapi_key": key("old@project.iam.gserviceaccount.com")});
    let tenant = tenant_with_settings(&tx, &settings).await;

    let unreadable = store_service_account_key(&tx, &tenant, json!({"type": "service_account"}))
        .await
        .unwrap_err();
    assert!(
        matches!(unreadable, GoogleMeetError::Json(_)),
        "{unreadable}"
    );

    let mut elsewhere = key(CLIENT_EMAIL);
    elsewhere["token_uri"] = json!("https://example.org/token");
    let refused = store_service_account_key(&tx, &tenant, elsewhere)
        .await
        .unwrap_err();
    assert!(matches!(refused, GoogleMeetError::TokenUri(_)), "{refused}");

    assert_eq!(stored_key(&tx, &tenant).await, None);
    assert_eq!(settings_of(&tx, &tenant).await, settings);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn store_service_account_key_fails_for_an_invalid_tenant() {
    use_master_secret();
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();

    let error = store_service_account_key(&tx, "not-a-uuid", key(CLIENT_EMAIL))
        .await
        .unwrap_err();

    assert!(matches!(error, GoogleMeetError::Other(_)), "{error}");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn remove_tenant_setting_removes_only_the_named_setting_of_that_tenant() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let settings = json!({"gapi_key": {"private_key": "x"}, "gapi_email": "organizer@example.org"});
    let tenant = tenant_with_settings(&tx, &settings).await;
    let other = tenant_with_settings(&tx, &settings).await;

    remove_tenant_setting(&tx, &tenant, "gapi_key")
        .await
        .unwrap();
    remove_tenant_setting(&tx, &tenant, "missing")
        .await
        .unwrap();

    assert_eq!(
        settings_of(&tx, &tenant).await,
        json!({"gapi_email": "organizer@example.org"})
    );
    assert_eq!(settings_of(&tx, &other).await, settings);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn remove_tenant_setting_rejects_an_invalid_tenant_id() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();

    let error = remove_tenant_setting(&tx, "not-a-uuid", "gapi_key")
        .await
        .unwrap_err();

    assert!(
        error.to_string().starts_with("Error parsing tenant UUID"),
        "{error}"
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn importing_a_tenant_moves_the_exported_key_to_the_vault() {
    use_master_secret();
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = tenant_with_settings(&tx, &json!({})).await;
    let settings = json!({"gapi_key": key(CLIENT_EMAIL), "gapi_email": "organizer@example.org"});
    let record = csv::StringRecord::from(vec![
        tenant.as_str(),
        "imported-tenant",
        "2026-01-01T00:00:00+00:00",
        "2026-01-01T00:00:00+00:00",
        "{}",
        "{}",
        "true",
        "{}",
        settings.to_string().as_str(),
    ]);

    process_record(&tx, &tenant, &record).await.unwrap();

    assert_eq!(stored_key(&tx, &tenant).await, Some(key(CLIENT_EMAIL)));
    assert_eq!(
        settings_of(&tx, &tenant).await,
        json!({"gapi_email": "organizer@example.org"})
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn importing_a_tenant_with_an_unusable_key_fails() {
    use_master_secret();
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = tenant_with_settings(&tx, &json!({})).await;
    let settings = json!({"gapi_key": {"type": "service_account"}});
    let record = csv::StringRecord::from(vec![
        tenant.as_str(),
        "imported-tenant",
        "2026-01-01T00:00:00+00:00",
        "2026-01-01T00:00:00+00:00",
        "{}",
        "{}",
        "true",
        "{}",
        settings.to_string().as_str(),
    ]);

    let error = process_record(&tx, &tenant, &record).await.unwrap_err();

    assert!(
        error
            .to_string()
            .starts_with("Error storing the Google service account key"),
        "{error}"
    );
    tx.rollback().await.unwrap();
}
