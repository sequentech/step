// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The tenant plugin gateway against the migrated database: turning Projects
//! on provisions its roles in Keycloak before the state commits, operations
//! are scoped to the token's tenant, and every change is an AdminChange
//! entry of the tenant's log as well as the plugin's own history.

use crate::adapters::memory::identity::LocalIdentityAdmin;
use crate::route_services::http::{Exchange, HttpServer};
use crate::route_services::rows::{self, Event};
use crate::route_services::{json, post, Services};
use crate::test_claims::Claims;
use rocket::http::{Header, Status};
use rocket::local::asynchronous::Client;
use sequent_core::services::keycloak::get_tenant_realm;
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};

const ADMIN_ID: &str = "plugin-admin";
const ADMIN: &str = "plugin.admin";
const ADMIN_GROUP_ID: &str = "admin-group";
const PROJECTS: [&str; 3] =
    ["projects-read", "projects-write", "projects-manage"];

fn admin(event: &Event, permissions: &[Permissions]) -> Claims {
    Claims::new(&event.tenant_id, ADMIN_ID)
        .username(ADMIN)
        .roles(permissions.iter().map(ToString::to_string))
}

fn manager(event: &Event) -> Claims {
    admin(
        event,
        &[
            Permissions::ADMIN_USER,
            Permissions::TENANT_WRITE,
            Permissions::PROJECTS_READ,
            Permissions::PROJECTS_WRITE,
            Permissions::PROJECTS_MANAGE,
        ],
    )
}

fn keycloak(peer: &HttpServer) -> LocalIdentityAdmin {
    LocalIdentityAdmin {
        url: Some(peer.url.clone()),
        ..Default::default()
    }
}

/// The exchanges of provisioning Projects in a realm without its roles.
fn provisioning(event: &Event) -> Vec<Exchange> {
    let realm = get_tenant_realm(&event.tenant_id);
    let mut exchanges = vec![];
    for name in PROJECTS {
        exchanges.push(Exchange::json(
            "GET",
            &format!("/admin/realms/{realm}/roles"),
            200,
            json!([]),
        ));
        exchanges.push(Exchange::json(
            "POST",
            &format!("/admin/realms/{realm}/roles"),
            201,
            Value::Null,
        ));
        exchanges.push(Exchange::json(
            "GET",
            &format!("/admin/realms/{realm}/roles/{name}"),
            200,
            json!({"id": format!("role-{name}"), "name": name}),
        ));
    }
    exchanges.push(Exchange::json(
        "GET",
        &format!("/admin/realms/{realm}/groups"),
        200,
        json!([{"id": ADMIN_GROUP_ID, "name": "admin"}]),
    ));
    exchanges.push(Exchange::json(
        "POST",
        &format!(
            "/admin/realms/{realm}/groups/{ADMIN_GROUP_ID}/role-mappings/realm"
        ),
        204,
        Value::Null,
    ));
    exchanges
}

async fn plugins(client: &Client, claims: &Claims) -> (Status, Value) {
    json(
        client
            .get("/plugins")
            .header(Header::new("Authorization", claims.bearer()))
            .dispatch()
            .await,
    )
    .await
}

async fn execute(
    client: &Client,
    claims: &Claims,
    operation: &str,
    data: Value,
) -> (Status, Value) {
    json(
        post(
            client,
            "/plugins/execute",
            claims,
            &json!({"plugin_id": "projects", "operation": operation, "data": data}),
        )
        .await,
    )
    .await
}

/// Turns Projects on directly, for tests about operations rather than roles.
async fn enabled(services: &Services, event: &Event) {
    rows::execute(
        &services.hasura,
        "INSERT INTO sequent_backend.plugin_tenant
            (tenant_id, plugin_id, state, data_version, updated_by)
         VALUES ($1::text::uuid, 'projects', 'enabled', 1, 'setup')",
        &[&event.tenant_id],
    )
    .await;
}

/// The tenant log's queued plugin entries: (description, username, details).
async fn audited(
    services: &Services,
    event: &Event,
) -> Vec<(String, Option<String>, Value)> {
    rows::query(
        &services.hasura,
        "SELECT description, username, details FROM sequent_backend.audit_log_outbox
         WHERE tenant_id = $1::text::uuid AND statement_kind = 'AdminChange'
             AND details -> 'object' ->> 'type' LIKE 'plugin_%'
         ORDER BY seq",
        &[&event.tenant_id],
    )
    .await
    .iter()
    .map(|row| (row.get(0), row.get(1), row.get(2)))
    .collect()
}

fn task(title: &str) -> Value {
    json!({
        "title": title, "type": "task",
        "start": "2028-03-06T08:00:00+08:00", "end": "2028-03-10T17:00:00+08:00",
        "status": "not_started", "notes": "", "order": 0,
    })
}

/// A tenant admin turns Projects on, plans a task, and every change is logged.
#[rocket::async_test]
async fn turning_projects_on_provisions_roles_and_its_changes_are_audited() {
    let base = Services::on_test_database().await;
    let event = rows::event(&base.hasura).await;
    let peer = HttpServer::start(provisioning(&event));
    let services = base.with_identity(keycloak(&peer));
    let client = services.client().await;
    let claims = manager(&event);

    let (status, listed) = plugins(&client, &claims).await;
    assert_eq!(status, Status::Ok, "{listed}");
    assert_eq!(listed["data"]["plugins"][0]["state"], "disabled");
    assert_eq!(listed["data"]["plugins"][0]["version"], 0);

    let (status, answer) = json(
        post(
            &client,
            "/plugins/state",
            &claims,
            &json!({"plugin_id": "projects", "state": "enabled", "expected_version": 0}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{answer}");
    assert_eq!(answer["data"]["plugins"][0]["state"], "enabled");
    let requests = peer.finish();
    let mapping = requests
        .iter()
        .find(|request| request.url.path().ends_with("/role-mappings/realm"))
        .expect("the admin group received the roles");
    let mapped = mapping.json();
    let granted: Vec<&str> = mapped
        .as_array()
        .expect("roles")
        .iter()
        .filter_map(|role| role["name"].as_str())
        .collect();
    assert_eq!(granted, PROJECTS);

    let (status, project) = execute(
        &client,
        &claims,
        "create-project",
        json!({"name": "Readiness", "timezone": "Asia/Manila"}),
    )
    .await;
    assert_eq!(status, Status::Ok, "{project}");
    let project_id = project["data"]["id"].clone();
    let (status, saved) = execute(
        &client,
        &claims,
        "save-task",
        json!({"project_id": project_id, "task": task("Election Readiness Test")}),
    )
    .await;
    assert_eq!(status, Status::Ok, "{saved}");
    assert_eq!(saved["data"]["updated_by"], ADMIN_ID);
    assert_eq!(saved["data"]["updated_by_name"], ADMIN);
    let (status, listed) =
        execute(&client, &claims, "list-projects", json!({})).await;
    assert_eq!(status, Status::Ok);
    assert_eq!(listed["data"]["items"][0]["task_count"], 1);

    let entries = audited(&services, &event).await;
    let objects: Vec<(&str, &str)> = entries
        .iter()
        .map(|(_, _, details)| {
            (
                details["object"]["type"].as_str().unwrap_or_default(),
                details["action"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(
        objects,
        [
            ("plugin_tenant", "create"),
            ("plugin_tenant", "update"),
            ("plugin_record", "create"),
            ("plugin_record", "create"),
        ]
    );
    assert!(entries.iter().all(|(_, username, details)| {
        username.as_deref() == Some(ADMIN)
            && details["actor"]["user_id"] == ADMIN_ID
            && details["origin"] == "harvest"
    }));
}

/// Keycloak refuses: nothing commits, and Projects stays off.
#[rocket::async_test]
async fn a_keycloak_failure_leaves_projects_off_and_unlogged() {
    let base = Services::on_test_database().await;
    let event = rows::event(&base.hasura).await;
    let realm = get_tenant_realm(&event.tenant_id);
    let peer = HttpServer::start(vec![Exchange::json(
        "GET",
        &format!("/admin/realms/{realm}/roles"),
        500,
        json!({"error": "unavailable"}),
    )]);
    let services = base.with_identity(keycloak(&peer));
    let client = services.client().await;
    let claims = manager(&event);

    let (status, answer) = json(
        post(
            &client,
            "/plugins/state",
            &claims,
            &json!({"plugin_id": "projects", "state": "enabled", "expected_version": 0}),
        )
        .await,
    )
    .await;
    peer.finish();
    assert_eq!(status, Status::InternalServerError, "{answer}");
    assert_eq!(answer["extensions"]["code"], "plugin-unavailable");
    let (_, listed) = plugins(&client, &claims).await;
    assert_eq!(listed["data"]["plugins"][0]["state"], "disabled");
    assert!(audited(&services, &event).await.is_empty());
}

/// Operations are refused while Projects is off, stale writes return the
/// current record, and a missing permission is refused before any write.
#[rocket::async_test]
async fn operations_are_scoped_versioned_and_refused_while_off() {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let client = services.client().await;
    let claims = manager(&event);

    let (status, refused) =
        execute(&client, &claims, "list-projects", json!({})).await;
    assert_eq!(status, Status::Forbidden, "{refused}");
    assert_eq!(refused["extensions"]["code"], "plugin-disabled");

    enabled(&services, &event).await;
    let (_, project) = execute(
        &client,
        &claims,
        "create-project",
        json!({"name": "Readiness", "timezone": "Asia/Manila"}),
    )
    .await;
    let project_id = project["data"]["id"].clone();
    let (_, saved) = execute(
        &client,
        &claims,
        "save-task",
        json!({"project_id": project_id, "task": task("Test voting")}),
    )
    .await;
    let update = |version: i64| {
        json!({"project_id": project_id, "id": saved["data"]["id"],
               "expected_version": version, "task": task("Test voting (mock polls)")})
    };
    let (status, _) = execute(&client, &claims, "save-task", update(1)).await;
    assert_eq!(status, Status::Ok);
    let (status, conflict) =
        execute(&client, &claims, "save-task", update(1)).await;
    assert_eq!(status, Status::Conflict, "{conflict}");
    assert_eq!(conflict["extensions"]["code"], "plugin-conflict");
    assert_eq!(conflict["extensions"]["current"]["version"], 2);

    let reader = admin(
        &event,
        &[Permissions::ADMIN_USER, Permissions::PROJECTS_READ],
    );
    let (status, refused) =
        execute(&client, &reader, "save-task", update(2)).await;
    assert_eq!(status, Status::Forbidden, "{refused}");
    assert_eq!(refused["extensions"]["code"], "plugin-forbidden");
    let (status, refused) = json(
        post(
            &client,
            "/plugins/state",
            &reader,
            &json!({"plugin_id": "projects", "state": "disabled", "expected_version": 1}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Forbidden, "{refused}");

    let (status, invalid) = execute(
        &client,
        &claims,
        "create-project",
        json!({"name": "Readiness", "timezone": "Mars/Olympus"}),
    )
    .await;
    assert_eq!(status, Status::BadRequest, "{invalid}");
    assert_eq!(invalid["extensions"]["code"], "plugin-invalid-input");

    let other = rows::event(&services.hasura).await;
    let (status, _) = execute(
        &client,
        &manager(&other),
        "get-project",
        json!({"id": project_id}),
    )
    .await;
    assert_eq!(
        status,
        Status::Forbidden,
        "another tenant has not turned Projects on"
    );
}
