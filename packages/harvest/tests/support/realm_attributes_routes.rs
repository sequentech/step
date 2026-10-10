// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Public realm edits cannot overwrite enrollment authority or race its writers.

use crate::route_services::{http, json, post, rows, Services};
use crate::test_claims::Claims;
use rocket::http::Status;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use windmill::services::enrollment_windows::ENROLLMENT_WINDOWS_ATTRIBUTE;

const RESTORE_ATTRIBUTE: &str = "enrollment_registration_restore";
const CHILD: &str = "HARVEST_REALM_ATTRIBUTES_CHILD";

// Core's token cache is global. Run only this test in a fresh process, retaining
// the isolated test database settings and pointing identity at its loopback peer.
fn isolated(test: &str) -> bool {
    if std::env::var(CHILD).as_deref() == Ok(test) {
        return true;
    }
    let log = tempfile::NamedTempFile::new().unwrap();
    let output = log.reopen().unwrap();
    let mut child =
        std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture"])
            .env(CHILD, test)
            .env("KEYCLOAK_ADMIN_CLIENT_ID", "synthetic-admin")
            .env("KEYCLOAK_ADMIN_CLIENT_SECRET", "synthetic-secret")
            .env("SUPER_ADMIN_TENANT_ID", "fixture-super-admin")
            .stdout(output.try_clone().unwrap())
            .stderr(output)
            .spawn()
            .unwrap();
    let deadline = Instant::now() + Duration::from_secs(45);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("realm attribute route test timed out");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(
        status.success(),
        "{}",
        std::fs::read_to_string(log.path()).unwrap()
    );
    false
}

fn peer(
    event: &rows::Event,
    attributes: Value,
    reads: usize,
    writes: usize,
) -> http::HttpServer {
    let path = format!("/admin/realms/{}", event.realm());
    let mut exchanges = vec![http::Exchange::json(
        "POST",
        "/realms/master/protocol/openid-connect/token",
        200,
        http::token_json(),
    )];
    exchanges.extend((0..reads).map(|_| http::Exchange::json(
        "GET", &path, 200, json!({"realm": event.realm(), "registrationAllowed": false, "attributes": attributes}),
    )));
    exchanges.extend(
        (0..writes).map(|_| http::Exchange::json("PUT", &path, 204, json!({}))),
    );
    let peer = http::HttpServer::start(exchanges);
    // This process runs one exact test; no other client or environment user runs.
    std::env::set_var("KEYCLOAK_URL", &peer.url);
    peer
}

async fn update<'c>(
    client: &'c rocket::local::asynchronous::Client,
    event: &rows::Event,
    attributes: Value,
) -> rocket::local::asynchronous::LocalResponse<'c> {
    post(client, "/update-realm-attributes", &Claims::new(&event.tenant_id, "operator")
        .roles([sequent_core::types::permissions::Permissions::KEYCLOAK_REALM_ATTRIBUTES_WRITE]),
        &json!({"election_event_id": event.election_event_id, "attributes": attributes})).await
}

#[rocket::async_test]
async fn public_edits_preserve_server_owned_enrollment_authority() {
    if !isolated("routes::realm_attributes::route_tests::public_edits_preserve_server_owned_enrollment_authority") { return; }
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let existing = json!({ENROLLMENT_WINDOWS_ATTRIBUTE: "{\"post\":null}", RESTORE_ATTRIBUTE: "enabled", "operator_note": "old"});
    // Four refusals each read authority; two normal edits each read then merge.
    let peer = peer(&event, existing.clone(), 8, 2);
    let client = services.client().await;
    for key in [ENROLLMENT_WINDOWS_ATTRIBUTE, RESTORE_ATTRIBUTE] {
        for value in ["forged", ""] {
            let (status, error) =
                json(update(&client, &event, json!({key: value})).await).await;
            assert_eq!(
                status,
                Status::BadRequest,
                "server-owned {key} cannot be changed or removed"
            );
            assert_eq!(
                error["extensions"]["code"],
                "RealmAttributesValidation"
            );
            assert_eq!(
                error["message"],
                format!(
                    "Realm attribute {key} is managed by enrollment scheduling"
                )
            );
        }
    }
    let mut unchanged = existing.clone();
    unchanged["operator_note"] = json!("new");
    assert_eq!(
        json(update(&client, &event, unchanged).await).await,
        (Status::Ok, json!({"updated": true}))
    );
    // Omitted keys are preserved by the normal merge; they are not deletions.
    assert_eq!(
        json(update(&client, &event, json!({"operator_note": "other"})).await)
            .await,
        (Status::Ok, json!({"updated": true}))
    );
    let requests = peer.finish();
    let writes: Vec<_> =
        requests.iter().filter(|r| r.method == "PUT").collect();
    assert_eq!(writes.len(), 2);
    for request in writes {
        assert_eq!(
            request.json()["attributes"][ENROLLMENT_WINDOWS_ATTRIBUTE],
            existing[ENROLLMENT_WINDOWS_ATTRIBUTE]
        );
        assert_eq!(
            request.json()["attributes"][RESTORE_ATTRIBUTE],
            existing[RESTORE_ATTRIBUTE]
        );
        assert_eq!(request.json()["registrationAllowed"], false);
    }
}

#[rocket::async_test]
async fn realm_snapshot_waits_for_the_event_scheduling_writer() {
    if !isolated("routes::realm_attributes::route_tests::realm_snapshot_waits_for_the_event_scheduling_writer") { return; }
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let peer =
        peer(&event, json!({ENROLLMENT_WINDOWS_ATTRIBUTE: "null"}), 2, 1);
    let client = services.client().await;
    let mut connection = services.hasura.get().await.unwrap();
    let transaction = connection.transaction().await.unwrap();
    windmill::postgres::scheduled_event::lock_scheduling_event(
        &transaction,
        &event.tenant_id,
        &event.election_event_id,
    )
    .await
    .unwrap();
    let update = update(&client, &event, json!({"operator_note": "new"}));
    tokio::pin!(update);
    assert!(tokio::time::timeout(Duration::from_millis(150), &mut update).await.is_err(),
        "realm read/write must wait until the existing scheduling writer finishes");
    transaction.commit().await.unwrap();
    assert_eq!(
        json(
            tokio::time::timeout(Duration::from_secs(5), update)
                .await
                .unwrap()
        )
        .await,
        (Status::Ok, json!({"updated": true}))
    );
    peer.finish();
}

#[rocket::async_test]
async fn password_policy_snapshot_waits_for_the_event_scheduling_writer() {
    if !isolated("routes::realm_attributes::route_tests::password_policy_snapshot_waits_for_the_event_scheduling_writer") { return; }
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let peer =
        peer(&event, json!({ENROLLMENT_WINDOWS_ATTRIBUTE: "null"}), 1, 1);
    let client = services.client().await;
    let mut connection = services.hasura.get().await.unwrap();
    let transaction = connection.transaction().await.unwrap();
    windmill::postgres::scheduled_event::lock_scheduling_event(
        &transaction,
        &event.tenant_id,
        &event.election_event_id,
    )
    .await
    .unwrap();
    let claims = Claims::new(&event.tenant_id, "operator").roles([
        sequent_core::types::permissions::Permissions::ELECTION_EVENT_WRITE,
    ]);
    let body = json!({"election_event_id": event.election_event_id,
        "minimum_length": 8, "maximum_length": 64, "include_uppercase": true,
        "include_lowercase": true, "include_digits": true, "include_special_characters": false});
    let update = post(&client, "/update-realm-password-policy", &claims, &body);
    tokio::pin!(update);
    assert!(
        tokio::time::timeout(Duration::from_millis(150), &mut update)
            .await
            .is_err(),
        "password-policy realm read/write must wait for the scheduling writer"
    );
    transaction.commit().await.unwrap();
    assert_eq!(
        json(
            tokio::time::timeout(Duration::from_secs(5), update)
                .await
                .unwrap()
        )
        .await,
        (Status::Ok, json!({"updated": true}))
    );
    let requests = peer.finish();
    let write = requests.iter().find(|r| r.method == "PUT").unwrap().json();
    assert_eq!(write["attributes"][ENROLLMENT_WINDOWS_ATTRIBUTE], "null");
    assert_eq!(write["registrationAllowed"], false);
}

#[rocket::async_test]
async fn unchanged_empty_and_missing_authority_values_are_noops() {
    if !isolated("routes::realm_attributes::route_tests::unchanged_empty_and_missing_authority_values_are_noops") { return; }
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let peer = peer(&event, json!({ENROLLMENT_WINDOWS_ATTRIBUTE: ""}), 2, 1);
    let client = services.client().await;
    assert_eq!(json(update(&client, &event, json!({ENROLLMENT_WINDOWS_ATTRIBUTE: "", RESTORE_ATTRIBUTE: "", "operator_note": "new"})).await).await,
        (Status::Ok, json!({"updated": true})));
    let requests = peer.finish();
    let write = requests.iter().find(|r| r.method == "PUT").unwrap().json();
    assert_eq!(write["attributes"][ENROLLMENT_WINDOWS_ATTRIBUTE], "");
    assert!(write["attributes"].get(RESTORE_ATTRIBUTE).is_none());
    assert_eq!(write["attributes"]["operator_note"], "new");
}

fn password_policy_body(event_id: &str) -> Value {
    json!({"election_event_id": event_id,
        "minimum_length": 8, "maximum_length": 64, "include_uppercase": true,
        "include_lowercase": true, "include_digits": true, "include_special_characters": false})
}

/// Neither writer may fall through to identity when its database is unavailable.
#[rocket::async_test]
async fn public_realm_writers_refuse_closed_database_pools_before_identity_access(
) {
    if !isolated("routes::realm_attributes::route_tests::public_realm_writers_refuse_closed_database_pools_before_identity_access") { return; }
    let services = Services::without_database();
    services.hasura.close();
    let peer = http::HttpServer::start(vec![]);
    std::env::set_var("KEYCLOAK_URL", &peer.url);
    let client = services.client().await;
    let tenant_id = uuid::Uuid::new_v4().to_string();
    let event_id = uuid::Uuid::new_v4().to_string();
    for (path, permission, body) in [
        ("/update-realm-attributes", sequent_core::types::permissions::Permissions::KEYCLOAK_REALM_ATTRIBUTES_WRITE,
            json!({"election_event_id": event_id, "attributes": {"operator_note": "new"}})),
        ("/update-realm-password-policy", sequent_core::types::permissions::Permissions::ELECTION_EVENT_WRITE,
            password_policy_body(&event_id), "Failed to update realm password policy"),
    ] {
        let claims = Claims::new(&tenant_id, "operator").roles([permission]);
        assert_eq!(json(post(&client, path, &claims, &body).await).await,
            (Status::InternalServerError, json!({"message": message, "extensions": {"code": "InternalServerError"}})));
    }
    let mut invalid_policy = password_policy_body(&event_id);
    invalid_policy["minimum_length"] = json!(64);
    invalid_policy["maximum_length"] = json!(8);
    let claims = Claims::new(&tenant_id, "operator").roles([
        sequent_core::types::permissions::Permissions::ELECTION_EVENT_WRITE,
    ]);
    assert_eq!(
        json(post(&client, "/update-realm-password-policy", &claims, &invalid_policy).await).await,
        (Status::BadRequest, json!({"message": "Minimum password length cannot exceed maximum password length",
            "extensions": {"code": "InvalidPasswordPolicy"}})),
        "invalid policy must be refused before even the closed pool is accessed"
    );
    assert!(
        peer.finish().is_empty(),
        "a failed database must not contact identity"
    );
    assert!(services.tasks.sent().is_empty());
    assert!(services.ledger.tasks().is_empty());
}

/// Malformed target IDs cannot bypass the scheduling writer lock.
#[rocket::async_test]
async fn public_realm_writers_refuse_invalid_lock_targets_before_identity_access(
) {
    if !isolated("routes::realm_attributes::route_tests::public_realm_writers_refuse_invalid_lock_targets_before_identity_access") { return; }
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let peer = http::HttpServer::start(vec![]);
    std::env::set_var("KEYCLOAK_URL", &peer.url);
    let event_id = uuid::Uuid::parse_str(&event.election_event_id).unwrap();
    let connection = services.hasura.get().await.unwrap();
    let before: Value = connection.query_one(
        "SELECT to_jsonb(e) FROM sequent_backend.election_event e WHERE id = $1", &[&event_id],
    ).await.unwrap().get(0);
    let client = services.client().await;
    for (path, permission, body) in [
        ("/update-realm-attributes", sequent_core::types::permissions::Permissions::KEYCLOAK_REALM_ATTRIBUTES_WRITE,
            json!({"election_event_id": "not-a-uuid", "attributes": {"operator_note": "new"}})),
        ("/update-realm-password-policy", sequent_core::types::permissions::Permissions::ELECTION_EVENT_WRITE,
            password_policy_body("not-a-uuid")),
    ] {
        let claims = Claims::new(&event.tenant_id, "operator").roles([permission]);
        assert_eq!(json(post(&client, path, &claims, &body).await).await,
            (Status::BadRequest, json!({"message": "election_event_id must be a UUID", "extensions": {"code": "UuidParseFailed"}})));
    }
    assert!(
        peer.finish().is_empty(),
        "a failed event lock must not contact identity"
    );
    assert!(services.tasks.sent().is_empty());
    assert!(services.ledger.tasks().is_empty());
    let after: Value = connection.query_one(
        "SELECT to_jsonb(e) FROM sequent_backend.election_event e WHERE id = $1", &[&event_id],
    ).await.unwrap().get(0);
    assert_eq!(after, before, "a refused lock must not change the event");
}

/// A refused identity read or write must never report a successful realm edit.
#[rocket::async_test]
async fn public_realm_writers_report_identity_read_and_write_failures() {
    if !isolated("routes::realm_attributes::route_tests::public_realm_writers_report_identity_read_and_write_failures") { return; }
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let path = format!("/admin/realms/{}", event.realm());
    let existing = json!({"realm": event.realm(), "registrationAllowed": false,
        "attributes": {ENROLLMENT_WINDOWS_ATTRIBUTE: "null", RESTORE_ATTRIBUTE: "enabled"}});
    let peer = http::HttpServer::start(vec![
        http::Exchange::json(
            "POST",
            "/realms/master/protocol/openid-connect/token",
            200,
            http::token_json(),
        ),
        http::Exchange::json(
            "GET",
            &path,
            503,
            json!({"error": "read unavailable"}),
        ),
        http::Exchange::json("GET", &path, 200, existing.clone()),
        http::Exchange::json("GET", &path, 200, existing.clone()),
        http::Exchange::json(
            "PUT",
            &path,
            503,
            json!({"error": "write unavailable"}),
        ),
        http::Exchange::json(
            "GET",
            &path,
            503,
            json!({"error": "read unavailable"}),
        ),
        http::Exchange::json("GET", &path, 200, existing),
        http::Exchange::json(
            "PUT",
            &path,
            503,
            json!({"error": "write unavailable"}),
        ),
    ]);
    std::env::set_var("KEYCLOAK_URL", &peer.url);
    let client = services.client().await;
    for (route, permission, body, message) in [
        ("/update-realm-attributes", sequent_core::types::permissions::Permissions::KEYCLOAK_REALM_ATTRIBUTES_WRITE,
            json!({"election_event_id": event.election_event_id, "attributes": {"operator_note": "new"}})),
        ("/update-realm-password-policy", sequent_core::types::permissions::Permissions::ELECTION_EVENT_WRITE,
            password_policy_body(&event.election_event_id), "Failed to update realm password policy"),
    ] {
        let claims = Claims::new(&event.tenant_id, "operator").roles([permission]);
        for _ in 0..2 {
            assert_eq!(json(post(&client, route, &claims, &body).await).await,
                (Status::InternalServerError, json!({"message": message, "extensions": {"code": "InternalServerError"}})));
        }
    }
    let requests = peer.finish();
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.method == "GET")
            .count(),
        5
    );
    let writes: Vec<_> = requests
        .iter()
        .filter(|request| request.method == "PUT")
        .collect();
    assert_eq!(writes.len(), 2, "failed reads must not attempt a write");
    for write in writes {
        assert_eq!(write.json()["registrationAllowed"], false);
        assert_eq!(
            write.json()["attributes"][ENROLLMENT_WINDOWS_ATTRIBUTE],
            "null"
        );
        assert_eq!(write.json()["attributes"][RESTORE_ATTRIBUTE], "enabled");
    }
}
