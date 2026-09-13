// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Real PostgreSQL rows exercise names, SQL types and nullability at the
//! Keycloak User boundary. These tests do not certify the application's schema
//! or its database queries; they require only a disposable synthetic cluster.

#![cfg(all(feature = "keycloak", feature = "default_features"))]

#[path = "support/postgres.rs"]
mod postgres;

use sequent_core::types::keycloak::User;
use serde_json::json;

const VALID_ROW: &str = "SELECT 'voter-7'::text AS id,
    'alice@example.test'::text AS email, true AS email_verified,
    false AS enabled, 'Alice'::text AS first_name,
    'Example'::text AS last_name, 'alice'::text AS username,
    $1::jsonb AS attributes";

#[rocket::async_test]
async fn database_user_mapping_preserves_identity_flags_and_multivalue_attributes(
) {
    let server = postgres::Postgres::start();
    let (client, connection) = server.connect().await;
    let attributes = json!({"tenant-id": "north", "roles": ["voter", "auditor"],
        "mixed": ["retained", 7, null, false], "unset": null});
    let row = client.query_one(VALID_ROW, &[&attributes]).await.unwrap();
    let user = User::try_from(row).unwrap();
    assert_eq!(user.id.as_deref(), Some("voter-7"));
    assert_eq!(user.email.as_deref(), Some("alice@example.test"));
    assert_eq!(user.username.as_deref(), Some("alice"));
    assert_eq!(user.first_name.as_deref(), Some("Alice"));
    assert_eq!(user.last_name.as_deref(), Some("Example"));
    assert_eq!(user.email_verified, Some(true));
    assert_eq!(user.enabled, Some(false));
    assert_eq!(
        user.attributes.unwrap(),
        [
            ("tenant-id".into(), vec!["north".into()]),
            ("roles".into(), vec!["voter".into(), "auditor".into()]),
            ("mixed".into(), vec!["retained".into()]),
            ("unset".into(), vec![]),
        ]
        .into()
    );
    assert!(user.area.is_none());
    assert!(user.votes_info.is_none());
    drop(client);
    connection.await.unwrap().unwrap();
}

#[rocket::async_test]
async fn nullable_database_fields_remain_absent_without_defaulting_flags() {
    let server = postgres::Postgres::start();
    let (client, connection) = server.connect().await;
    let row = client
        .query_one(
            "SELECT NULL::text AS id, NULL::text AS email,
        NULL::boolean AS email_verified, NULL::boolean AS enabled,
        NULL::text AS first_name, NULL::text AS last_name,
        NULL::text AS username, '{}'::jsonb AS attributes",
            &[],
        )
        .await
        .unwrap();
    let user = User::try_from(row).unwrap();
    assert!(user.id.is_none());
    assert!(user.email.is_none());
    assert!(user.username.is_none());
    assert!(user.first_name.is_none());
    assert!(user.last_name.is_none());
    assert!(user.email_verified.is_none());
    assert!(user.enabled.is_none());
    assert_eq!(user.attributes, Some(Default::default()));
    drop(client);
    connection.await.unwrap().unwrap();
}

#[rocket::async_test]
async fn invalid_database_rows_return_conversion_errors_instead_of_partial_users(
) {
    let server = postgres::Postgres::start();
    let (client, connection) = server.connect().await;
    // Keep all other fields valid, so each rejected shape reaches the attribute
    // conversion rather than failing for an unrelated missing column.
    for attributes in [json!(null), json!([]), json!("not an object"), json!(7)]
    {
        let row = client.query_one(VALID_ROW, &[&attributes]).await.unwrap();
        let error = User::try_from(row).unwrap_err();
        assert!(error.downcast_ref::<serde_json::Error>().is_some());
    }
    for query in [
        "SELECT NULL::jsonb AS attributes",
        "SELECT '{}'::text AS attributes",
        "SELECT '{}'::jsonb AS attributes",
        "SELECT '{}'::jsonb AS attributes, 7::int4 AS id",
    ] {
        let row = client.query_one(query, &[]).await.unwrap();
        let error = User::try_from(row).unwrap_err();
        assert!(error.downcast_ref::<tokio_postgres::Error>().is_some());
    }
    let valid = client.query_one(VALID_ROW, &[&json!({})]).await.unwrap();
    assert_eq!(
        User::try_from(valid).unwrap().username.as_deref(),
        Some("alice")
    );
    drop(client);
    connection.await.unwrap().unwrap();
}
