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
    migrate_realm_permissions, migrate_realms, migrated_roles, retry_countdown, RunOnce,
};

/// The English labels Users and Roles shows for the signing permissions,
/// read from the admin portal's translations: the migration gives each
/// role its label as its description.
fn signing_labels_in_the_portal() -> BTreeMap<String, String> {
    include_str!("../../admin-portal/src/translations/en.ts")
        .lines()
        .filter_map(|line| {
            let (key, value) = line.trim().split_once(": ")?;
            let key = key.trim_matches('"');
            // Other screens have keys like `signing-key-usage`; only
            // permissions count.
            let signing = Permissions::from_str(key).is_ok()
                && (key == "election-event-signatures-tab"
                    || key.starts_with("signing-")
                    || key.starts_with("sign-"));
            let value = value.trim_end_matches(',');
            (signing && value.starts_with('"'))
                .then(|| (key.to_string(), value.trim_matches('"').to_string()))
        })
        .collect()
}

#[test]
fn the_migration_adds_the_signing_permissions_with_their_labels() {
    let expected = signing_labels_in_the_portal();
    assert_eq!(expected.len(), 21, "{expected:?}");
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

/// Every tenant realm gets the roles, skipping those it already has, and
/// nothing else: no groups, so no group assignment changes.
#[tokio::test]
async fn every_tenant_realm_gets_the_roles_it_lacks_and_nothing_else() {
    let tenants = ["tenant-one".to_string(), "tenant-two".to_string()];
    let peer = HttpServer::start(vec![
        import("tenant-tenant-one", 200, json!({"added": 21, "skipped": 0})),
        import("tenant-tenant-two", 200, json!({"added": 0, "skipped": 21})),
    ]);
    let migrated = migrate_realms(&peer.public_client(), &tenants)
        .await
        .unwrap();
    assert_eq!(
        migrated,
        vec![
            (
                "tenant-tenant-one".to_string(),
                PartialImportSummary {
                    added: 21,
                    ..Default::default()
                }
            ),
            (
                "tenant-tenant-two".to_string(),
                PartialImportSummary {
                    skipped: 21,
                    ..Default::default()
                }
            ),
        ]
    );
    let requests = peer.finish();
    let expected = json!({
        "ifResourceExists": "SKIP",
        "roles": {"realm": serde_json::to_value(migrated_roles()).unwrap()},
    });
    assert_eq!(requests.len(), 2);
    for request in &requests {
        assert_eq!(request.json(), expected);
    }
}

#[tokio::test]
async fn a_realm_that_fails_does_not_stop_the_others_and_fails_the_run() {
    let tenants = ["missing".to_string(), "present".to_string()];
    let peer = HttpServer::start(vec![
        import("tenant-missing", 404, json!({"error": "Realm not found."})),
        import("tenant-present", 200, json!({"added": 21})),
    ]);
    let error = migrate_realms(&peer.public_client(), &tenants)
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("tenant-missing"), "{error}");
    assert!(!error.contains("tenant-present"), "{error}");
    // Both realms were asked.
    assert_eq!(peer.finish().len(), 2);
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
            .map(|tenant| import(&format!("tenant-{tenant}"), 200, json!({"added": 21})))
            .collect(),
    );
    let migrated = migrate_realms(&peer.public_client(), &tenant_ids)
        .await
        .unwrap();
    assert_eq!(migrated.len(), 2);
    assert_eq!(peer.finish().len(), 2);
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
