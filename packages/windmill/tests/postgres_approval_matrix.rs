// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The approval matrix versions against the migrated schema. Every test
//! writes its own rows in a transaction and rolls it back.

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::{Object, Transaction};
use serde_json::{json, Value};
use uuid::Uuid;
use windmill::services::approval_matrix::evaluate::{MatrixSource, MatrixVersion};
use windmill::services::approval_matrix::store::{
    get_current_approval_matrix, get_latest_approval_matrix, import_approval_matrix,
    insert_approval_matrix, matrix_sha256,
};
use windmill::services::approval_matrix::ApprovalMatrix;

const USER_ID: &str = "3f2b8c1e-6d4a-4c1b-9e7f-2a5d8b0c9e1a";

async fn connect() -> Object {
    schema::pool().await.get().await.unwrap()
}

/// A new tenant with one election event.
async fn event(tx: &Transaction<'_>) -> (String, String) {
    let tenant = Uuid::new_v4();
    let event = Uuid::new_v4();
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
    (tenant.to_string(), event.to_string())
}

fn association() -> Value {
    json!({
        "compared_fields": ["firstName", "lastName", "dateOfBirth"],
        "rules": [
            {"when": {"already_enrolled": true}, "then": {"decision": "REJECTED", "reason": "ALREADY_APPROVED"}},
            {"when": {"identity": "MANUAL_ENTRY"}, "then": {"decision": "PENDING", "reason": "IDENTITY_NOT_VERIFIED"}},
            {"when": {"differing": "none"}, "then": {"decision": "ACCEPTED"}}
        ],
        "otherwise": {"decision": "PENDING", "reason": "NO_VOTER"}
    })
}

fn matrix(value: Value) -> ApprovalMatrix {
    serde_json::from_value(value).unwrap()
}

fn fields() -> Vec<String> {
    vec!["firstName".to_string(), "embassy".to_string()]
}

#[tokio::test]
async fn an_event_without_a_saved_matrix_uses_the_built_in_one() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let (tenant, event) = event(&tx).await;

    assert_eq!(
        get_latest_approval_matrix(&tx, &tenant, &event)
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        get_current_approval_matrix(&tx, &tenant, &event, fields())
            .await
            .unwrap(),
        MatrixVersion::built_in(fields())
    );
}

#[tokio::test]
async fn saving_adds_versions_after_the_built_in_one() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let (tenant, event) = event(&tx).await;
    let built_in = ApprovalMatrix::built_in(fields());

    let second = insert_approval_matrix(
        &tx,
        &tenant,
        &event,
        &built_in,
        Some(USER_ID),
        Some("admin"),
    )
    .await
    .unwrap();
    let third = insert_approval_matrix(&tx, &tenant, &event, &matrix(association()), None, None)
        .await
        .unwrap();

    assert_eq!(second.version, 2);
    assert_eq!(second.matrix, built_in);
    assert_eq!(second.sha256, matrix_sha256(&built_in).unwrap());
    assert_eq!(second.created_by.as_deref(), Some(USER_ID));
    assert_eq!(second.created_by_username.as_deref(), Some("admin"));
    assert_eq!(third.version, 3);
    assert_ne!(third.sha256, second.sha256);
    assert_eq!(
        get_latest_approval_matrix(&tx, &tenant, &event)
            .await
            .unwrap(),
        Some(third.clone())
    );
    let current = get_current_approval_matrix(&tx, &tenant, &event, fields())
        .await
        .unwrap();
    assert_eq!(current.version, 3);
    assert_eq!(current.source, MatrixSource::SAVED);
    assert_eq!(current.matrix, matrix(association()));
}

#[tokio::test]
async fn versions_belong_to_their_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let (tenant, first_event) = event(&tx).await;
    let (other_tenant, other_event) = event(&tx).await;
    insert_approval_matrix(
        &tx,
        &tenant,
        &first_event,
        &matrix(association()),
        None,
        None,
    )
    .await
    .unwrap();

    assert_eq!(
        get_latest_approval_matrix(&tx, &other_tenant, &other_event)
            .await
            .unwrap(),
        None
    );
    assert!(get_latest_approval_matrix(&tx, &other_tenant, &first_event)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn a_bundles_matrix_is_imported_as_version_1() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let (tenant, event) = event(&tx).await;

    let imported = import_approval_matrix(&tx, &tenant, &event, &association())
        .await
        .unwrap();

    assert_eq!(imported.version, 1);
    assert_eq!(imported.created_by, None);
    assert_eq!(imported.matrix, matrix(association()));
    let next = insert_approval_matrix(&tx, &tenant, &event, &imported.matrix, None, None)
        .await
        .unwrap();
    assert_eq!(next.version, 2);
}

#[tokio::test]
async fn a_bundles_matrix_that_breaks_an_invariant_is_refused() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let (tenant, event) = event(&tx).await;
    let mut accepting = association();
    accepting["otherwise"] = json!({"decision": "ACCEPTED"});
    let mut unknown = association();
    unknown["rules"][0]["when"]["nationality"] = json!("PH");

    let error = import_approval_matrix(&tx, &tenant, &event, &accepting)
        .await
        .unwrap_err();
    assert!(
        format!("{error:#}").contains("OTHERWISE_ACCEPTS"),
        "{error:#}"
    );
    let error = import_approval_matrix(&tx, &tenant, &event, &unknown)
        .await
        .unwrap_err();
    assert!(
        format!("{error:#}").contains("Error reading the bundle's approval matrix"),
        "{error:#}"
    );
    assert_eq!(
        get_latest_approval_matrix(&tx, &tenant, &event)
            .await
            .unwrap(),
        None
    );
}

#[tokio::test]
async fn saved_versions_cannot_be_changed_and_go_with_their_event() {
    let mut client = connect().await;
    let mut tx = client.transaction().await.unwrap();
    let (tenant, event) = event(&tx).await;
    insert_approval_matrix(&tx, &tenant, &event, &matrix(association()), None, None)
        .await
        .unwrap();
    let event_id = Uuid::parse_str(&event).unwrap();

    let savepoint = tx.savepoint("update").await.unwrap();
    let error = savepoint
        .execute(
            "UPDATE sequent_backend.approval_matrix SET rules = '[]' WHERE election_event_id = $1",
            &[&event_id],
        )
        .await
        .unwrap_err();
    assert!(format!("{error:?}").contains("immutable"), "{error:?}");
    savepoint.rollback().await.unwrap();

    tx.execute(
        "DELETE FROM sequent_backend.election_event WHERE id = $1",
        &[&event_id],
    )
    .await
    .unwrap();
    let left: i64 = tx
        .query_one(
            "SELECT count(*) FROM sequent_backend.approval_matrix WHERE election_event_id = $1",
            &[&event_id],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(left, 0);
}
