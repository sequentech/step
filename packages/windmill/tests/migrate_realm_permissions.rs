// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The realm permission migration against a local Keycloak stand-in: what
//! it sends to each tenant realm, and that one realm's failure neither stops
//! the others nor passes for success.

#[path = "support/schema.rs"]
mod schema;

// Formatted with Sequent Core, whose line width differs.
#[path = "../../sequent-core/tests/support/http.rs"]
#[allow(dead_code)]
#[rustfmt::skip]
mod http;

use celery::beat::Schedule;
use celery::task::Task;
use http::{Exchange, HttpServer};
use sequent_core::services::keycloak::PartialImportSummary;
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::str::FromStr;
use std::time::SystemTime;
use uuid::Uuid;
use windmill::postgres::tenant::get_tenant_ids;
use windmill::tasks::migrate_realm_permissions::{
    migrate_realm_permissions, migrate_realms, migrated_roles, retry_countdown, GroupGrant,
    RealmMigration, RunOnce, TEMPLATE_GROUP_GRANTS,
};

/// The English labels Users and Roles shows for the migrated permissions
/// (signing, messaging and readiness), read from the admin portal's translations: the
/// migration gives each role its label as its description.
fn migrated_labels_in_the_portal() -> BTreeMap<String, String> {
    include_str!("../../admin-portal/src/translations/en.ts")
        .lines()
        .filter_map(|line| {
            let (key, value) = line.trim().split_once(": ")?;
            let key = key.trim_matches('"');
            // Other screens have keys like `signing-key-usage`; only
            // permissions count.
            let migrated = Permissions::from_str(key).is_ok()
                && (key == "election-event-signatures-tab"
                    || key.starts_with("signing-")
                    || key.starts_with("sign-")
                    || key.starts_with("messaging-")
                    || key == "readiness-check");
            let value = value.trim_end_matches(',');
            (migrated && value.starts_with('"'))
                .then(|| (key.to_string(), value.trim_matches('"').to_string()))
        })
        .collect()
}

#[test]
fn the_migration_adds_release_permissions_with_their_labels() {
    let expected = migrated_labels_in_the_portal();
    assert_eq!(expected.len(), 26, "{expected:?}");
    let roles: BTreeMap<String, String> = migrated_roles()
        .into_iter()
        .map(|role| (role.name.unwrap(), role.description.unwrap()))
        .collect();
    assert_eq!(roles, expected);
    for role in migrated_roles() {
        // Plain realm roles, as the realm templates define them.
        assert_eq!(role.composite, Some(false));
        assert_eq!(role.client_role, Some(false));
        assert_eq!(role.id, None, "Keycloak gives each realm its own ids");
    }
}

fn import(realm: &str, status: u16, body: Value) -> Exchange {
    Exchange::json(
        "POST",
        &format!("/admin/realms/{realm}/partialImport"),
        status,
        body,
    )
}

/// The group searches, role holders and role mappings of a realm whose
/// groups already hold every messaging role the migration grants.
fn groups_already_granted(realm: &str) -> Vec<Exchange> {
    let all = [
        "messaging-account-read",
        "messaging-account-write",
        "messaging-config-write",
        "messaging-config-read",
        "readiness-check",
    ];
    vec![
        Exchange::json(
            "GET",
            &format!("/admin/realms/{realm}/groups"),
            200,
            json!([{"id": "admin-id", "name": "admin", "path": "/admin"}]),
        ),
        Exchange::json(
            "GET",
            &format!("/admin/realms/{realm}/groups"),
            200,
            json!([{"id": "light-id", "name": "admin-light", "path": "/admin-light"}]),
        ),
        Exchange::json(
            "GET",
            &format!("/admin/realms/{realm}/roles/messaging-config-write/groups"),
            200,
            json!([{"id": "admin-id", "name": "admin", "path": "/admin"}]),
        ),
        Exchange::json(
            "GET",
            &format!("/admin/realms/{realm}/roles/messaging-account-read/groups"),
            200,
            json!([
                {"id": "admin-id", "name": "admin", "path": "/admin"},
                {"id": "light-id", "name": "admin-light", "path": "/admin-light"},
            ]),
        ),
        Exchange::json(
            "GET",
            &format!("/admin/realms/{realm}/groups/admin-id/role-mappings/realm"),
            200,
            json!(all
                .iter()
                .map(|name| json!({"id": format!("{name}-id"), "name": name}))
                .collect::<Vec<_>>()),
        ),
        Exchange::json(
            "GET",
            &format!("/admin/realms/{realm}/groups/light-id/role-mappings/realm"),
            200,
            json!([
                {"id": "messaging-account-read-id", "name": "messaging-account-read"},
                {"id": "messaging-config-read-id", "name": "messaging-config-read"},
            ]),
        ),
    ]
}

/// Every tenant realm gets the roles, skipping those it already has; groups
/// that already hold every role they're granted are left as they are.
#[tokio::test]
async fn every_tenant_realm_gets_the_roles_it_lacks() {
    let tenants = ["tenant-one".to_string(), "tenant-two".to_string()];
    let mut exchanges = vec![
        import("tenant-tenant-one", 200, json!({"added": 26, "skipped": 0})),
        import("tenant-tenant-two", 200, json!({"added": 0, "skipped": 26})),
    ];
    exchanges.extend(groups_already_granted("tenant-tenant-one"));
    exchanges.extend(groups_already_granted("tenant-tenant-two"));
    let peer = HttpServer::start(exchanges);
    let migrated = migrate_realms(&peer.public_client(), &tenants)
        .await
        .unwrap();
    assert_eq!(
        migrated,
        vec![
            RealmMigration {
                realm: "tenant-tenant-one".to_string(),
                roles: PartialImportSummary {
                    added: 26,
                    ..Default::default()
                },
                granted: vec![],
            },
            RealmMigration {
                realm: "tenant-tenant-two".to_string(),
                roles: PartialImportSummary {
                    skipped: 26,
                    ..Default::default()
                },
                granted: vec![],
            },
        ]
    );
    let requests = peer.finish();
    let expected = json!({
        "ifResourceExists": "SKIP",
        "roles": {"realm": serde_json::to_value(migrated_roles()).unwrap()},
    });
    let imports: Vec<_> = requests
        .iter()
        .filter(|request| request.url.path().ends_with("/partialImport"))
        .collect();
    assert_eq!(imports.len(), 2);
    for request in imports {
        assert_eq!(request.json(), expected);
    }
    // Nothing was granted: no mapping was posted.
    assert!(requests
        .iter()
        .all(|request| request.method != "POST" || request.url.path().ends_with("/partialImport")));
}

/// Regression (R1-04, R4-05): the roles reached existing realms, but no
/// group held them, so every administrator of an existing tenant lost the
/// event's Messaging tab, Logs and Send Notification after the upgrade.
/// The groups get what the realm template gives new realms; any group that
/// manages messaging accounts or configurations can read it too.
#[tokio::test]
async fn existing_groups_get_the_release_roles_the_template_gives_them() {
    let realm = "tenant-upgraded";
    let role = |name: &str| {
        Exchange::json(
            "GET",
            &format!("/admin/realms/{realm}/roles/{name}"),
            200,
            json!({"id": format!("{name}-id"), "name": name, "composite": false}),
        )
    };
    let peer = HttpServer::start(vec![
        import(realm, 200, json!({"added": 4})),
        // `admin` exists, with one messaging role; `admin-light` doesn't.
        Exchange::json(
            "GET",
            &format!("/admin/realms/{realm}/groups"),
            200,
            json!([{"id": "admin-id", "name": "admin", "path": "/admin", "subGroups": []}]),
        ),
        Exchange::json(
            "GET",
            &format!("/admin/realms/{realm}/groups"),
            200,
            // A search returns a parent whose subgroup matches: not the group.
            json!([{"id": "parent-id", "name": "regional", "path": "/regional",
                    "subGroups": [{"id": "nested-id", "name": "admin-light", "path": "/regional/admin-light"}]}]),
        ),
        Exchange::json(
            "GET",
            &format!("/admin/realms/{realm}/roles/messaging-config-write/groups"),
            200,
            json!([]),
        ),
        Exchange::json(
            "GET",
            &format!("/admin/realms/{realm}/roles/messaging-account-read/groups"),
            200,
            json!([
                {"id": "admin-id", "name": "admin", "path": "/admin"},
                {"id": "auditors-id", "name": "auditors", "path": "/auditors"},
            ]),
        ),
        Exchange::json(
            "GET",
            &format!("/admin/realms/{realm}/groups/admin-id/role-mappings/realm"),
            200,
            json!([
                {"id": "admin-role-id", "name": "admin-user"},
                {"id": "messaging-account-read-id", "name": "messaging-account-read"},
            ]),
        ),
        role("messaging-account-write"),
        role("messaging-config-write"),
        role("messaging-config-read"),
        role("readiness-check"),
        Exchange::json(
            "POST",
            &format!("/admin/realms/{realm}/groups/admin-id/role-mappings/realm"),
            204,
            json!({}),
        ),
        Exchange::json(
            "GET",
            &format!("/admin/realms/{realm}/groups/auditors-id/role-mappings/realm"),
            200,
            json!([{"id": "messaging-account-read-id", "name": "messaging-account-read"}]),
        ),
        Exchange::json(
            "POST",
            &format!("/admin/realms/{realm}/groups/auditors-id/role-mappings/realm"),
            204,
            json!({}),
        ),
    ]);
    // A peer that answers the role lookups once each: the second group's
    // missing role is looked up from the first's answer.
    let migrated = migrate_realms(&peer.public_client(), &["upgraded".to_string()])
        .await
        .unwrap();
    assert_eq!(
        migrated[0].granted,
        vec![
            GroupGrant {
                group: "/admin".to_string(),
                roles: vec![
                    "messaging-account-write".to_string(),
                    "messaging-config-write".to_string(),
                    "messaging-config-read".to_string(),
                    "readiness-check".to_string(),
                ],
            },
            GroupGrant {
                group: "/auditors".to_string(),
                roles: vec!["messaging-config-read".to_string()],
            },
        ]
    );
    let requests = peer.finish();
    let posted: Vec<(String, Value)> = requests
        .iter()
        .filter(|request| {
            request.method == "POST" && request.url.path().ends_with("/role-mappings/realm")
        })
        .map(|request| (request.url.path().to_string(), request.json()))
        .collect();
    let mapping = |names: &[&str]| {
        json!(names
            .iter()
            .map(|name| json!({"id": format!("{name}-id"), "name": name}))
            .collect::<Vec<_>>())
    };
    assert_eq!(
        posted,
        vec![
            (
                format!("/admin/realms/{realm}/groups/admin-id/role-mappings/realm"),
                mapping(&[
                    "messaging-account-write",
                    "messaging-config-write",
                    "messaging-config-read",
                    "readiness-check"
                ]),
            ),
            (
                format!("/admin/realms/{realm}/groups/auditors-id/role-mappings/realm"),
                mapping(&["messaging-config-read"]),
            ),
        ]
    );
    // The searches were exact, by name.
    let searches: Vec<_> = requests
        .iter()
        .filter(|request| request.url.path() == format!("/admin/realms/{realm}/groups"))
        .map(|request| {
            (
                request.query().get("search").cloned(),
                request.query().get("exact").cloned(),
            )
        })
        .collect();
    assert_eq!(
        searches,
        vec![
            (Some("admin".to_string()), Some("true".to_string())),
            (Some("admin-light".to_string()), Some("true".to_string())),
        ]
    );
}

/// The groups and roles the migration grants are the realm templates' own.
#[test]
fn the_granted_roles_are_the_realm_templates() {
    for (name, text) in [
        (
            "tenant realm template",
            include_str!(
                "../../../.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json"
            ),
        ),
        (
            "janitor client template",
            include_str!("../external-bin/janitor/templates/COMELEC/keycloakAdmin.hbs"),
        ),
    ] {
        let realm: Value = serde_json::from_str(text).unwrap();
        for (group, roles) in TEMPLATE_GROUP_GRANTS {
            let held: Vec<String> = realm["groups"]
                .as_array()
                .unwrap()
                .iter()
                .find(|g| g["name"] == group)
                .unwrap_or_else(|| panic!("{name}: no group {group}"))["realmRoles"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|role| role.as_str())
                .filter(|role| role.starts_with("messaging-") || *role == "readiness-check")
                .map(str::to_string)
                .collect();
            let mut granted: Vec<String> = roles.iter().map(|r| r.to_string()).collect();
            let mut held = held;
            granted.sort();
            held.sort();
            assert_eq!(granted, held, "{name}: {group}");
        }
    }
}

#[tokio::test]
async fn a_realm_that_fails_does_not_stop_the_others_and_fails_the_run() {
    let tenants = ["missing".to_string(), "present".to_string()];
    let mut exchanges = vec![
        import("tenant-missing", 404, json!({"error": "Realm not found."})),
        import("tenant-present", 200, json!({"added": 26})),
    ];
    exchanges.extend(groups_already_granted("tenant-present"));
    let peer = HttpServer::start(exchanges);
    let error = migrate_realms(&peer.public_client(), &tenants)
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("tenant-missing"), "{error}");
    assert!(!error.contains("tenant-present"), "{error}");
    // Both realms were asked; the missing one only for its roles.
    let requests = peer.finish();
    assert!(requests
        .iter()
        .any(|request| request.url.path() == "/admin/realms/tenant-missing/partialImport"));
    assert!(requests.iter().all(|request| !request
        .url
        .path()
        .starts_with("/admin/realms/tenant-missing/groups")));
}

#[test]
fn beat_sends_the_migration_once_when_it_starts() {
    assert!(RunOnce.next_call_at(None).is_some());
    assert_eq!(RunOnce.next_call_at(Some(SystemTime::now())), None);
}

/// The tenants come from the database, each realm once, in id order.
#[tokio::test]
async fn the_realm_of_every_tenant_in_the_database_is_migrated() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenants = [
        Uuid::parse_str("00000006-0000-4000-8000-000000000002").unwrap(),
        Uuid::parse_str("00000006-0000-4000-8000-000000000001").unwrap(),
    ];
    for tenant in tenants {
        tx.execute(
            "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
            &[&tenant, &format!("slug-{tenant}")],
        )
        .await
        .unwrap();
    }
    let tenant_ids = get_tenant_ids(&tx).await.unwrap();
    assert_eq!(
        tenant_ids,
        [
            "00000006-0000-4000-8000-000000000001",
            "00000006-0000-4000-8000-000000000002",
        ]
    );
    let peer = HttpServer::start(
        tenants
            .iter()
            .flat_map(|tenant| {
                let realm = format!("tenant-{tenant}");
                let mut exchanges = vec![import(&realm, 200, json!({"added": 26}))];
                exchanges.extend(groups_already_granted(&realm));
                exchanges
            })
            .collect(),
    );
    let migrated = migrate_realms(&peer.public_client(), &tenant_ids)
        .await
        .unwrap();
    assert_eq!(migrated.len(), 2);
    peer.finish();
}

/// New realms get the roles from the templates with the same labels the
/// migration gives existing realms.
#[test]
fn the_realm_templates_describe_the_roles_as_the_migration_does() {
    let expected: BTreeMap<String, String> = migrated_roles()
        .into_iter()
        .map(|role| (role.name.unwrap(), role.description.unwrap()))
        .collect();
    for (name, text) in [
        (
            "tenant realm template",
            include_str!(
                "../../../.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json"
            ),
        ),
        (
            "janitor client template",
            include_str!("../external-bin/janitor/templates/COMELEC/keycloakAdmin.hbs"),
        ),
    ] {
        let realm: Value = serde_json::from_str(text).unwrap();
        let described: BTreeMap<String, String> = realm["roles"]["realm"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|role| {
                let name = role["name"].as_str()?;
                expected.contains_key(name).then(|| {
                    (
                        name.to_string(),
                        role["description"].as_str().unwrap_or_default().to_string(),
                    )
                })
            })
            .collect();
        assert_eq!(described, expected, "{name}");
    }
}

/// Keycloak may still be starting when beat sends the migration: it is
/// retried five times, waiting twice as long each time.
#[test]
fn a_failed_migration_is_retried_with_a_growing_wait() {
    assert_eq!(migrate_realm_permissions::DEFAULTS.max_retries, Some(5));
    let waits: Vec<u32> = (0..5).map(retry_countdown).collect();
    assert_eq!(waits, [30, 60, 120, 240, 480]);
    // Far past the retries the wait saturates instead of overflowing.
    assert_eq!(retry_countdown(40), u32::MAX);
}
