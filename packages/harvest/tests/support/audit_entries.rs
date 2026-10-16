// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The audit entries routes build, and queueing one on the migrated test
//! database.

use super::*;
use crate::route_services::{rows, Services};
use crate::test_claims::Claims;
use std::collections::HashMap;
use windmill::services::audit_log::AuditOrigin;

const TENANT_ID: &str = "4f1c6b38-9f5e-4a59-8d3b-2a7f0c1e5d61";
const EVENT_ID: &str = "b7e2d915-3c4a-4e8f-9a61-5d0f2c8b7e43";
const ADMIN_ID: &str = "admin-id";
const ADMIN: &str = "ada.admin";

fn admin() -> JwtClaims {
    Claims::new(TENANT_ID, ADMIN_ID).username(ADMIN).build()
}

fn tenant_log() -> AuditLogTarget {
    target(TENANT_ID, None).unwrap()
}

fn event_log() -> AuditLogTarget {
    target(TENANT_ID, Some(EVENT_ID)).unwrap()
}

fn staff(enabled: bool) -> User {
    User {
        id: Some("user-1".into()),
        username: Some("grace".into()),
        email: Some("grace@example.test".into()),
        enabled: Some(enabled),
        first_name: Some("Grace".into()),
        attributes: Some(HashMap::from([(
            "tenant-id".to_string(),
            vec![TENANT_ID.to_string()],
        )])),
        ..Default::default()
    }
}

fn paths(change: &AdminChange) -> Vec<&str> {
    change
        .changes
        .iter()
        .map(|change| change.path.as_str())
        .collect()
}

#[test]
fn the_actor_is_the_admin_of_the_token() {
    assert_eq!(
        actor(&admin()),
        AuditActor {
            user_id: Some(ADMIN_ID.into()),
            username: Some(ADMIN.into()),
            role: Some("user".into()),
        }
    );
}

#[test]
fn an_event_id_selects_the_event_log_and_bad_ids_are_refused() {
    assert_eq!(tenant_log().election_event_id, None);
    assert_eq!(
        event_log().election_event_id,
        Some(Uuid::parse_str(EVENT_ID).unwrap())
    );
    assert_eq!(account_kind(&tenant_log()), STAFF_ACCOUNT);
    assert_eq!(account_kind(&event_log()), VOTER);
    for (tenant, event) in [("tenant", None), (TENANT_ID, Some("event"))] {
        let (status, message) = target(tenant, event).unwrap_err();
        assert_eq!(status, Status::BadRequest);
        assert!(message.starts_with("Invalid "), "{message}");
    }
}

#[test]
fn a_created_account_lists_its_fields_and_roles() {
    let created = account_created(
        &admin(),
        tenant_log(),
        &staff(true),
        &["role-1".to_string()],
    );
    assert_eq!(
        created.object,
        AuditObject::new(STAFF_ACCOUNT, "user-1", Some("grace"))
    );
    assert_eq!(created.action, AdminAction::Create);
    assert_eq!(created.origin, AuditOrigin::Harvest);
    assert_eq!(created.actor.username.as_deref(), Some(ADMIN));
    assert_eq!(
        paths(&created),
        vec![
            "attributes.tenant-id",
            "email",
            "enabled",
            "first_name",
            "username",
            "roles"
        ]
    );
    assert!(created.changes.iter().all(|change| change.old.is_none()));
    assert_eq!(created.changes[5].new, Some(json!(["role-1"])));

    let voter = account_created(&admin(), event_log(), &staff(true), &[]);
    assert_eq!(voter.object.kind, VOTER);
    assert_eq!(voter.target, event_log());
    assert!(!paths(&voter).contains(&"roles"));
}

#[test]
fn an_edit_that_only_enables_or_disables_is_an_action_of_its_own() {
    let disabled = account_edited(
        &admin(),
        tenant_log(),
        &staff(true),
        &staff(false),
        PasswordEdit::Kept,
    );
    assert_eq!(disabled.action, AdminAction::Disable);
    assert_eq!(
        disabled.changes,
        vec![FieldChange::new(
            "enabled",
            Some(&json!(true)),
            Some(&json!(false))
        )]
    );
    let enabled = account_edited(
        &admin(),
        tenant_log(),
        &staff(false),
        &staff(true),
        PasswordEdit::Kept,
    );
    assert_eq!(enabled.action, AdminAction::Enable);

    let mut renamed = staff(false);
    renamed.first_name = Some("Grace B.".into());
    let updated = account_edited(
        &admin(),
        tenant_log(),
        &staff(true),
        &renamed,
        PasswordEdit::Kept,
    );
    assert_eq!(updated.action, AdminAction::Update);
    assert_eq!(paths(&updated), vec!["enabled", "first_name"]);
    assert_eq!(updated.changes[1].old, Some(json!("Grace")));
    assert_eq!(updated.changes[1].new, Some(json!("Grace B.")));
}

#[test]
fn a_password_set_by_an_admin_is_named_and_never_recorded() {
    let reset = account_edited(
        &admin(),
        tenant_log(),
        &staff(true),
        &staff(true),
        PasswordEdit::Set,
    );
    assert_eq!(reset.action, AdminAction::ResetPassword);
    assert_eq!(
        serde_json::to_value(&reset.changes).unwrap(),
        json!([{"path": "password", "secret": true}])
    );

    let with_disable = account_edited(
        &admin(),
        tenant_log(),
        &staff(true),
        &staff(false),
        PasswordEdit::Set,
    );
    assert_eq!(with_disable.action, AdminAction::Update);
    assert_eq!(paths(&with_disable), vec!["enabled", "password"]);
}

#[test]
fn an_edit_that_changes_nothing_is_an_update_without_changes() {
    let same = account_edited(
        &admin(),
        tenant_log(),
        &staff(true),
        &staff(true),
        PasswordEdit::Kept,
    );
    assert_eq!(same.action, AdminAction::Update);
    assert!(same.changes.is_empty());
}

#[test]
fn a_bulk_deletion_is_one_entry_with_its_count_and_ids() {
    let ids = ["user-1".to_string(), "user-2".to_string()];
    let deleted = accounts_deleted(&admin(), tenant_log(), &ids);
    assert_eq!(deleted.object, AuditObject::new(STAFF_ACCOUNT, MANY, None));
    assert_eq!(deleted.action, AdminAction::Delete);
    assert_eq!(
        deleted.extra,
        json!({"count": 2, "ids": ["user-1", "user-2"]})
    );
    assert_eq!(deleted.describe(), "Deleted 2 staff accounts");
    assert_eq!(
        accounts_deleted(&admin(), event_log(), &ids).describe(),
        "Deleted 2 voter accounts"
    );
}

#[test]
fn roles_and_permissions_are_granted_as_new_and_revoked_as_old() {
    let user = account(&tenant_log(), "user-1", Some("grace"));
    let granted = account_role(
        &admin(),
        tenant_log(),
        user.clone(),
        AdminAction::Grant,
        "Auditor",
    );
    assert_eq!(
        serde_json::to_value(&granted.changes).unwrap(),
        json!([{"path": "roles", "new": "Auditor"}])
    );
    let revoked = account_role(
        &admin(),
        tenant_log(),
        user,
        AdminAction::Revoke,
        "Auditor",
    );
    assert_eq!(
        serde_json::to_value(&revoked.changes).unwrap(),
        json!([{"path": "roles", "old": "Auditor"}])
    );

    let auditor = role("group-1", Some("Auditor"));
    let added = role_permission(
        &admin(),
        tenant_log(),
        auditor.clone(),
        AdminAction::Grant,
        "tenant-logs-read",
    );
    assert_eq!(
        added.object,
        AuditObject::new(ROLE, "group-1", Some("Auditor"))
    );
    assert_eq!(
        serde_json::to_value(&added.changes).unwrap(),
        json!([{"path": "permissions", "new": "tenant-logs-read"}])
    );
    let removed = role_permission(
        &admin(),
        tenant_log(),
        auditor,
        AdminAction::Revoke,
        "tenant-logs-read",
    );
    assert_eq!(
        serde_json::to_value(&removed.changes).unwrap(),
        json!([{"path": "permissions", "old": "tenant-logs-read"}])
    );
}

#[test]
fn created_and_deleted_roles_carry_their_permissions() {
    let new_role = Role {
        id: None,
        name: Some("Auditor".into()),
        permissions: Some(vec!["tenant-logs-read".into()]),
        access: None,
        attributes: None,
        client_roles: None,
    };
    let created =
        role_created(&admin(), tenant_log(), &new_role, Some("group-1"));
    assert_eq!(
        created.object,
        AuditObject::new(ROLE, "group-1", Some("Auditor"))
    );
    assert_eq!(created.action, AdminAction::Create);
    assert_eq!(paths(&created), vec!["name", "permissions"]);
    assert_eq!(
        role_created(&admin(), tenant_log(), &new_role, None)
            .object
            .id,
        "Auditor"
    );

    let deleted = role_deleted(
        &admin(),
        tenant_log(),
        "group-1",
        Some("Auditor"),
        &["role-read".to_string(), "area-read".to_string()],
    );
    assert_eq!(deleted.action, AdminAction::Delete);
    assert_eq!(
        serde_json::to_value(&deleted.changes).unwrap(),
        json!([{"path": "permissions", "old": ["area-read", "role-read"]}])
    );
    assert!(role_deleted(&admin(), tenant_log(), "group-1", None, &[])
        .changes
        .is_empty());
}

#[test]
fn a_created_permission_goes_by_its_name() {
    let created = permission_created(
        &admin(),
        tenant_log(),
        &Permission {
            id: None,
            attributes: None,
            container_id: None,
            description: Some("Reads the tenant log".into()),
            name: Some("tenant-logs-read".into()),
        },
    );
    assert_eq!(
        created.object,
        AuditObject::new(
            PERMISSION,
            "tenant-logs-read",
            Some("tenant-logs-read")
        )
    );
    assert_eq!(paths(&created), vec!["description", "name"]);
}

#[rocket::async_test]
async fn a_recorded_change_is_queued_for_its_log_with_its_admin() {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let log = target(&event.tenant_id, None).unwrap();
    let entry = account_edited(
        &admin(),
        log,
        &staff(true),
        &staff(false),
        PasswordEdit::Kept,
    );

    record(&services.harvest_services(), &entry).await.unwrap();

    let queued = rows::query(
        &services.hasura,
        "SELECT election_event_id, statement_kind, event_type, log_type,
                user_id, username, description, details, posted_at IS NULL
         FROM sequent_backend.audit_log_outbox WHERE tenant_id = $1",
        &[&log.tenant_id],
    )
    .await;
    assert_eq!(queued.len(), 1);
    let row = &queued[0];
    assert_eq!(row.get::<_, Option<Uuid>>(0), None);
    assert_eq!(row.get::<_, String>(1), "AdminChange");
    assert_eq!(row.get::<_, String>(2), "USER");
    assert_eq!(row.get::<_, String>(3), "INFO");
    assert_eq!(row.get::<_, Option<String>>(4).as_deref(), Some(ADMIN_ID));
    assert_eq!(row.get::<_, Option<String>>(5).as_deref(), Some(ADMIN));
    assert_eq!(row.get::<_, String>(6), "Disabled staff account \"grace\"");
    assert_eq!(
        row.get::<_, Value>(7),
        json!({
            "object": {"type": "staff_account", "id": "user-1", "name": "grace"},
            "action": "disable",
            "actor": {"user_id": ADMIN_ID, "username": ADMIN, "role": "user"},
            "origin": "harvest",
            "changes": [{"path": "enabled", "old": true, "new": false}],
        })
    );
    assert!(row.get::<_, bool>(8));
}

#[rocket::async_test]
async fn a_change_that_cannot_be_queued_is_an_error() {
    let services = Services::without_database();
    let entry = change(
        &admin(),
        tenant_log(),
        permission("tenant-logs-read"),
        AdminAction::Delete,
    );
    let (status, message) = record(&services.harvest_services(), &entry)
        .await
        .unwrap_err();
    assert_eq!(status, Status::InternalServerError);
    assert!(
        message.starts_with(
            "The change was made, but its audit entry could not be queued"
        ),
        "{message}"
    );
}

fn attributes(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

#[test]
fn realm_attribute_changes_name_what_changed_and_hide_secrets() {
    let current = attributes(&[
        ("operator_note", "old"),
        ("kept", "same"),
        ("removed", "value"),
        ("smart-link-shared-secret", "before"),
        ("untouched", "value"),
    ]);
    let updates = attributes(&[
        ("operator_note", "new"),
        ("kept", "same"),
        ("removed", ""),
        ("added", "value"),
        ("smart-link-shared-secret", "after"),
        ("never-set", ""),
    ]);
    assert_eq!(
        serde_json::to_value(realm_attribute_changes(&current, &updates))
            .unwrap(),
        json!([
            {"path": "realm_attributes.added", "new": "value"},
            {"path": "realm_attributes.operator_note", "old": "old", "new": "new"},
            {"path": "realm_attributes.removed", "old": "value"},
            {"path": "realm_attributes.smart-link-shared-secret", "secret": true},
        ])
    );
    assert!(realm_attribute_changes(&current, &HashMap::new()).is_empty());
}

#[rocket::async_test]
async fn a_route_that_names_its_admin_logs_the_rows_it_changes() {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let admin = Claims::new(&event.tenant_id, ADMIN_ID)
        .username(ADMIN)
        .build();
    let area = event.area(&services.hasura, "North").await;

    let mut client = services.hasura.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    act_for(&transaction, &admin).await.unwrap();
    transaction
        .execute(
            "UPDATE sequent_backend.area SET name = 'North-East'
             WHERE id = $1::text::uuid",
            &[&area],
        )
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    let queued = rows::query(
        &services.hasura,
        "SELECT election_event_id::text, username, description, details
         FROM sequent_backend.audit_log_outbox
         WHERE tenant_id = $1::text::uuid ORDER BY seq",
        &[&event.tenant_id],
    )
    .await;
    assert_eq!(queued.len(), 1);
    let row = &queued[0];
    assert_eq!(
        row.get::<_, Option<String>>(0),
        Some(event.election_event_id.clone())
    );
    assert_eq!(row.get::<_, Option<String>>(1).as_deref(), Some(ADMIN));
    assert_eq!(row.get::<_, String>(2), "Updated area \"North-East\": name");
    let details = row.get::<_, Value>(3);
    assert_eq!(details["origin"], "harvest");
    assert_eq!(
        details["actor"],
        json!({"user_id": ADMIN_ID, "username": ADMIN, "role": "user"})
    );
    assert_eq!(
        details["changes"],
        json!([{"path": "name", "old": "North", "new": "North-East"}])
    );
}

#[test]
fn an_import_is_one_entry_with_its_source_document() {
    let entry = imported(
        &admin(),
        event_log(),
        "candidate",
        "document-1",
        Some("ABCD"),
        "Imported candidates",
    );
    assert_eq!(entry.action, AdminAction::Import);
    assert_eq!(entry.object, AuditObject::new("candidate", MANY, None));
    assert_eq!(entry.describe(), "Imported candidates");
    assert_eq!(
        entry.extra,
        json!({"document_id": "document-1", "declared_sha256": "abcd"})
    );
}

#[test]
fn an_export_names_its_task_and_its_document() {
    let entry = exported(
        &admin(),
        event_log(),
        election_event(EVENT_ID),
        "Exported the election event",
        Some("task-1"),
        Some("document-1"),
    );
    assert_eq!(entry.action, AdminAction::Export);
    assert_eq!(entry.actor, actor(&admin()));
    assert_eq!(entry.describe(), "Exported the election event");
    assert_eq!(
        entry.extra,
        json!({"task_execution_id": "task-1", "document_id": "document-1"})
    );
    let without_document = exported(
        &admin(),
        event_log(),
        election_event(EVENT_ID),
        "Exported the trustees",
        Some("task-2"),
        None,
    );
    assert_eq!(
        without_document.extra,
        json!({"task_execution_id": "task-2"})
    );
}

#[rocket::async_test]
async fn naming_an_admin_needs_a_tenant_id_and_a_working_transaction() {
    let services = Services::on_test_database().await;
    let mut client = services.hasura.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();

    let without_tenant = Claims::new("not-a-tenant", ADMIN_ID).build();
    let (status, message) =
        act_for(&transaction, &without_tenant).await.unwrap_err();
    assert_eq!(status, Status::BadRequest);
    assert!(message.starts_with("Invalid tenant_id"), "{message}");

    // A statement that failed aborts the transaction: nothing more runs.
    assert!(transaction.execute("SELECT 1/0", &[]).await.is_err());
    let (status, message) = act_for(&transaction, &admin()).await.unwrap_err();
    assert_eq!(status, Status::InternalServerError);
    assert!(
        message.starts_with("The admin of the change could not be named"),
        "{message}"
    );
}

#[test]
fn an_audit_error_keeps_its_status_and_message_as_json() {
    let error = json_error((Status::BadRequest, "Invalid tenant_id".into()));
    let body = serde_json::to_value(&error.1 .0).unwrap();
    assert_eq!(error.0, Status::BadRequest);
    assert_eq!(body["message"], "Invalid tenant_id");
    assert_eq!(body["extensions"]["code"], "InternalServerError");
}

#[test]
fn an_entry_that_cannot_be_queued_fails_its_change() {
    let (status, message) = queue_error(anyhow::anyhow!("outbox closed"));
    assert_eq!(status, Status::InternalServerError);
    assert!(
        message.starts_with("Error queueing the audit entry"),
        "{message}"
    );
    assert!(message.contains("outbox closed"), "{message}");

    let error = json_queue_error(anyhow::anyhow!("outbox closed"));
    assert_eq!(error.0, Status::InternalServerError);
    assert!(error
        .1
         .0
        .message
        .starts_with("Error queueing the audit entry"));
}
