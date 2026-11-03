// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Changes to the tenant's accounts are entries of the tenant's log, queued
//! once Keycloak made the change and never for one it refused.

use crate::adapters::memory::identity::LocalIdentityAdmin;
use crate::route_services::http::{Exchange, HttpServer};
use crate::route_services::rows::{self, Event};
use crate::route_services::{json, post, text, Services};
use crate::test_claims::Claims;
use rocket::http::Status;
use sequent_core::services::keycloak::get_tenant_realm;
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};

const ADMIN_ID: &str = "account-admin";
const ADMIN: &str = "ada.admin";
const USER_ID: &str = "user-1";
const ROLE_ID: &str = "group-1";

fn admin(event: &Event) -> Claims {
    Claims::new(&event.tenant_id, ADMIN_ID)
        .username(ADMIN)
        .roles([Permissions::USER_WRITE, Permissions::ROLE_WRITE])
}

fn keycloak(peer: &HttpServer) -> LocalIdentityAdmin {
    LocalIdentityAdmin {
        url: Some(peer.url.clone()),
        ..Default::default()
    }
}

fn user_path(event: &Event, user_id: &str) -> String {
    format!(
        "/admin/realms/{}/users/{user_id}",
        get_tenant_realm(&event.tenant_id)
    )
}

/// The account as Keycloak holds it.
fn account(event: &Event, enabled: bool) -> Exchange {
    Exchange::json(
        "GET",
        &user_path(event, USER_ID),
        200,
        json!({
            "id": USER_ID, "username": "grace", "enabled": enabled,
            "email": "grace@example.test", "firstName": "Grace",
        }),
    )
}

fn answer(method: &'static str, path: &str, status: u16) -> Exchange {
    Exchange::json(method, path, status, Value::Null)
}

/// The tenant log's queued entries: (description, user id, details).
async fn audited(
    services: &Services,
    event: &Event,
) -> Vec<(String, Option<String>, Value)> {
    rows::query(
        &services.hasura,
        "SELECT description, user_id, details FROM sequent_backend.audit_log_outbox
         WHERE tenant_id = $1::text::uuid AND election_event_id IS NULL
             AND statement_kind = 'AdminChange' AND posted_at IS NULL
         ORDER BY seq",
        &[&event.tenant_id],
    )
    .await
    .iter()
    .map(|row| (row.get(0), row.get(1), row.get(2)))
    .collect()
}

fn actor() -> Value {
    json!({"user_id": ADMIN_ID, "username": ADMIN, "role": "user"})
}

#[rocket::async_test]
async fn deleting_a_staff_account_is_an_entry_naming_it() {
    let base = Services::on_test_database().await;
    let event = rows::event(&base.hasura).await;
    let peer = HttpServer::start(vec![
        account(&event, true),
        answer("DELETE", &user_path(&event, USER_ID), 204),
    ]);
    let services = base.with_identity(keycloak(&peer));
    let client = services.client().await;

    let (status, body) = json(
        post(
            &client,
            "/delete-user",
            &admin(&event),
            &json!({"tenant_id": event.tenant_id, "user_id": USER_ID}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    peer.finish();
    assert_eq!(
        audited(&services, &event).await,
        vec![(
            "Deleted staff account \"grace\"".to_string(),
            Some(ADMIN_ID.to_string()),
            json!({
                "object": {"type": "staff_account", "id": USER_ID, "name": "grace"},
                "action": "delete",
                "actor": actor(),
                "origin": "harvest",
                "changes": [],
            })
        )]
    );
}

#[rocket::async_test]
async fn a_deletion_keycloak_refuses_leaves_no_entry() {
    let base = Services::on_test_database().await;
    let event = rows::event(&base.hasura).await;
    let peer = HttpServer::start(vec![
        account(&event, true),
        answer("DELETE", &user_path(&event, USER_ID), 403),
    ]);
    let services = base.with_identity(keycloak(&peer));
    let client = services.client().await;

    let (status, message) = text(
        post(
            &client,
            "/delete-user",
            &admin(&event),
            &json!({"tenant_id": event.tenant_id, "user_id": USER_ID}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::InternalServerError);
    assert!(message.starts_with("Error deleting the user"), "{message}");
    peer.finish();
    assert_eq!(audited(&services, &event).await, vec![]);
}

#[rocket::async_test]
async fn deleting_several_staff_accounts_is_one_entry_with_those_deleted() {
    for (second, deleted, answered) in [
        (204, vec![USER_ID, "user-2"], Status::Ok),
        (500, vec![USER_ID], Status::InternalServerError),
    ] {
        let base = Services::on_test_database().await;
        let event = rows::event(&base.hasura).await;
        let peer = HttpServer::start(vec![
            answer("DELETE", &user_path(&event, USER_ID), 204),
            answer("DELETE", &user_path(&event, "user-2"), second),
        ]);
        let services = base.with_identity(keycloak(&peer));
        let client = services.client().await;

        let response = post(
            &client,
            "/delete-users",
            &admin(&event),
            &json!({"tenant_id": event.tenant_id, "users_id": [USER_ID, "user-2"]}),
        )
        .await;
        assert_eq!(response.status(), answered);
        peer.finish();
        let entries = audited(&services, &event).await;
        assert_eq!(entries.len(), 1, "{second}");
        assert_eq!(
            entries[0].0,
            format!("Deleted {} staff accounts", deleted.len())
        );
        assert_eq!(
            entries[0].2["object"],
            json!({"type": "staff_account", "id": "*", "name": null})
        );
        assert_eq!(
            entries[0].2["extra"],
            json!({"count": deleted.len(), "ids": deleted})
        );
    }
}

#[rocket::async_test]
async fn a_bulk_deletion_that_deletes_nothing_leaves_no_entry() {
    let base = Services::on_test_database().await;
    let event = rows::event(&base.hasura).await;
    let peer = HttpServer::start(vec![answer(
        "DELETE",
        &user_path(&event, USER_ID),
        500,
    )]);
    let services = base.with_identity(keycloak(&peer));
    let client = services.client().await;

    let (status, _) = text(
        post(
            &client,
            "/delete-users",
            &admin(&event),
            &json!({"tenant_id": event.tenant_id, "users_id": [USER_ID, "user-2"]}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::InternalServerError);
    peer.finish();
    assert_eq!(audited(&services, &event).await, vec![]);
}

/// The request's edit, the action it is logged as and its changes.
#[rocket::async_test]
async fn editing_a_staff_account_logs_what_changed_and_never_a_password() {
    let cases = [
        (
            json!({"enabled": false}),
            "disable",
            json!([{"path": "enabled", "old": true, "new": false}]),
        ),
        (
            json!({"first_name": "Grace B.", "email": "g@example.test"}),
            "update",
            json!([
                {"path": "email", "old": "grace@example.test", "new": "g@example.test"},
                {"path": "first_name", "old": "Grace", "new": "Grace B."},
            ]),
        ),
        (
            json!({"password": "correct horse battery", "temporary": true}),
            "reset_password",
            json!([{"path": "password", "secret": true}]),
        ),
    ];
    for (edit, action, changes) in cases {
        let base = Services::on_test_database().await;
        let event = rows::event(&base.hasura).await;
        let peer = HttpServer::start(vec![
            account(&event, true),
            account(&event, true),
            answer("PUT", &user_path(&event, USER_ID), 204),
        ]);
        let services = base.with_identity(keycloak(&peer));
        let client = services.client().await;
        let mut request =
            json!({"tenant_id": event.tenant_id, "user_id": USER_ID});
        request
            .as_object_mut()
            .unwrap()
            .extend(edit.as_object().unwrap().clone());

        let (status, body) =
            json(post(&client, "/edit-user", &admin(&event), &request).await)
                .await;
        assert_eq!(status, Status::Ok, "{action}: {body}");
        peer.finish();
        let entries = audited(&services, &event).await;
        assert_eq!(entries.len(), 1, "{action}");
        assert_eq!(
            entries[0].2,
            json!({
                "object": {"type": "staff_account", "id": USER_ID, "name": "grace"},
                "action": action,
                "actor": actor(),
                "origin": "harvest",
                "changes": changes,
            }),
            "{action}"
        );
        assert!(
            !entries[0].2.to_string().contains("correct horse"),
            "{action}"
        );
    }
}

#[rocket::async_test]
async fn an_edit_keycloak_refuses_leaves_no_entry() {
    let base = Services::on_test_database().await;
    let event = rows::event(&base.hasura).await;
    let peer = HttpServer::start(vec![
        account(&event, true),
        account(&event, true),
        answer("PUT", &user_path(&event, USER_ID), 500),
    ]);
    let services = base.with_identity(keycloak(&peer));
    let client = services.client().await;

    let response = post(
        &client,
        "/edit-user",
        &admin(&event),
        &json!({"tenant_id": event.tenant_id, "user_id": USER_ID, "enabled": false}),
    )
    .await;
    assert_eq!(response.status(), Status::InternalServerError);
    peer.finish();
    assert_eq!(audited(&services, &event).await, vec![]);
}

fn role_path(event: &Event) -> String {
    format!("{}/groups/{ROLE_ID}", user_path(event, USER_ID))
}

fn role(event: &Event) -> Exchange {
    Exchange::json(
        "GET",
        &format!(
            "/admin/realms/{}/groups/{ROLE_ID}",
            get_tenant_realm(&event.tenant_id)
        ),
        200,
        json!({"id": ROLE_ID, "name": "Auditor"}),
    )
}

#[rocket::async_test]
async fn giving_and_taking_a_role_are_entries_naming_the_account_and_the_role()
{
    for (path, method, action, description, value) in [
        (
            "/set-user-role",
            "PUT",
            "grant",
            "Granted staff account \"grace\"",
            json!({"path": "roles", "new": "Auditor"}),
        ),
        (
            "/delete-user-role",
            "DELETE",
            "revoke",
            "Revoked staff account \"grace\"",
            json!({"path": "roles", "old": "Auditor"}),
        ),
    ] {
        let base = Services::on_test_database().await;
        let event = rows::event(&base.hasura).await;
        let peer = HttpServer::start(vec![
            account(&event, true),
            role(&event),
            answer(method, &role_path(&event), 204),
        ]);
        let services = base.with_identity(keycloak(&peer));
        let client = services.client().await;

        let (status, body) = json(
            post(
                &client,
                path,
                &admin(&event),
                &json!({"tenant_id": event.tenant_id, "user_id": USER_ID, "role_id": ROLE_ID}),
            )
            .await,
        )
        .await;
        assert_eq!(status, Status::Ok, "{path}: {body}");
        peer.finish();
        assert_eq!(
            audited(&services, &event).await,
            vec![(
                description.to_string(),
                Some(ADMIN_ID.to_string()),
                json!({
                    "object": {"type": "staff_account", "id": USER_ID, "name": "grace"},
                    "action": action,
                    "actor": actor(),
                    "origin": "harvest",
                    "changes": [value],
                })
            )],
            "{path}"
        );
    }
}

#[rocket::async_test]
async fn a_role_keycloak_refuses_to_give_leaves_no_entry() {
    let base = Services::on_test_database().await;
    let event = rows::event(&base.hasura).await;
    let peer = HttpServer::start(vec![
        account(&event, true),
        role(&event),
        answer("PUT", &role_path(&event), 403),
    ]);
    let services = base.with_identity(keycloak(&peer));
    let client = services.client().await;

    let (status, _) = text(
        post(
            &client,
            "/set-user-role",
            &admin(&event),
            &json!({"tenant_id": event.tenant_id, "user_id": USER_ID, "role_id": ROLE_ID}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::InternalServerError);
    peer.finish();
    assert_eq!(audited(&services, &event).await, vec![]);
}

/// A change Keycloak made stays made when its entry cannot be queued: the
/// route says so.
#[rocket::async_test]
async fn a_change_whose_entry_cannot_be_queued_is_an_error() {
    let event = Event {
        tenant_id: uuid::Uuid::new_v4().to_string(),
        election_event_id: uuid::Uuid::new_v4().to_string(),
    };
    let peer = HttpServer::start(vec![
        account(&event, true),
        answer("DELETE", &user_path(&event, USER_ID), 204),
    ]);
    let services = Services::without_database().with_identity(keycloak(&peer));
    let client = services.client().await;

    let (status, message) = text(
        post(
            &client,
            "/delete-user",
            &admin(&event),
            &json!({"tenant_id": event.tenant_id, "user_id": USER_ID}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::InternalServerError);
    assert!(
        message.starts_with(
            "The change was made, but its audit entry could not be queued"
        ),
        "{message}"
    );
    peer.finish();
}
