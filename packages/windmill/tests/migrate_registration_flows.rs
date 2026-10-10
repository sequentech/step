// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Real Keycloak wire failures and recoverable signup state during gate repair.
#[path = "../../sequent-core/tests/support/http.rs"]
#[allow(dead_code)]
#[rustfmt::skip]
mod http;

use http::{Exchange, HttpServer};
use serde_json::json;
use windmill::tasks::migrate_registration_flows::{migrate_realm, FlowOutcome};

#[tokio::test]
async fn failed_gate_repair_records_enabled_signup_for_safe_retry() {
    let peer = HttpServer::start(vec![
        Exchange::json(
            "GET",
            "/admin/realms/event",
            200,
            json!({"realm":"event","registrationFlow":"registration","registrationAllowed":true,"attributes":{"other":"kept"}}),
        ),
        Exchange::json(
            "GET",
            "/admin/realms/event/authentication/flows/registration/executions",
            200,
            json!([
                {"providerId":"registration-page-form","authenticationFlow":true,"displayName":"form","flowId":"form-id","level":0},
                {"providerId":"registration-user-creation","id":"user","level":1,"requirement":"REQUIRED"}
            ]),
        ),
        Exchange::json("PUT", "/admin/realms/event", 204, json!({})),
        Exchange::json(
            "GET",
            "/admin/realms/event/authentication/flows/form-id",
            200,
            json!({"id":"form-id","alias":"form","builtIn":false}),
        ),
        Exchange::json(
            "POST",
            "/admin/realms/event/authentication/flows/form/executions/execution",
            500,
            json!({"error":"synthetic unavailable"}),
        ),
    ]);
    assert!(
        migrate_realm(&peer.client(), &peer.public_client(), "event")
            .await
            .is_err()
    );
    let requests = peer.finish();
    let paused = requests
        .iter()
        .find(|request| request.method == "PUT" && request.url.path() == "/admin/realms/event")
        .unwrap()
        .json();
    assert_eq!(paused["registrationAllowed"], false);
    assert_eq!(
        paused["attributes"]["enrollment_registration_restore"],
        "enabled"
    );
    assert_eq!(paused["attributes"]["other"], "kept");
}

#[tokio::test]
async fn verified_retry_restores_recorded_signup_and_clears_only_its_marker() {
    let paused = json!({"realm":"event","registrationFlow":"registration","registrationAllowed":false,"attributes":{"enrollment_registration_restore":"enabled","other":"kept"}});
    let peer = HttpServer::start(vec![
        Exchange::json("GET", "/admin/realms/event", 200, paused.clone()),
        Exchange::json(
            "GET",
            "/admin/realms/event/authentication/flows/registration/executions",
            200,
            json!([
                {"providerId":"registration-page-form","authenticationFlow":true,"displayName":"form","flowId":"form-id","level":0},
                {"providerId":"enrollment-window-check","id":"gate","level":1,"requirement":"REQUIRED"},
                {"providerId":"registration-user-creation","id":"user","level":1,"requirement":"REQUIRED"}
            ]),
        ),
        Exchange::json("GET", "/admin/realms/event", 200, paused),
        Exchange::json("PUT", "/admin/realms/event", 204, json!({})),
    ]);
    assert_eq!(
        migrate_realm(&peer.client(), &peer.public_client(), "event")
            .await
            .unwrap(),
        FlowOutcome::AlreadyPresent
    );
    let requests = peer.finish();
    let restored = requests
        .iter()
        .find(|request| request.method == "PUT" && request.url.path() == "/admin/realms/event")
        .unwrap()
        .json();
    assert_eq!(restored["registrationAllowed"], true);
    assert!(restored["attributes"]
        .get("enrollment_registration_restore")
        .is_none());
    assert_eq!(restored["attributes"]["other"], "kept");
}

#[tokio::test]
async fn import_guard_verification_preserves_pending_intent_until_setup_completes() {
    let paused = json!({"realm":"event","registrationFlow":"registration","registrationAllowed":false,"attributes":{"enrollment_registration_restore":"enabled","other":"kept"}});
    let peer = HttpServer::start(vec![
        Exchange::json("GET", "/admin/realms/event", 200, paused.clone()),
        Exchange::json(
            "GET",
            "/admin/realms/event/authentication/flows/registration/executions",
            200,
            json!([
                {"providerId":"registration-page-form","authenticationFlow":true,"displayName":"form","flowId":"form-id","level":0},
                {"providerId":"enrollment-window-check","id":"gate","level":1,"requirement":"REQUIRED"},
                {"providerId":"registration-user-creation","id":"user","level":1,"requirement":"REQUIRED"}
            ]),
        ),
        Exchange::json("GET", "/admin/realms/event", 200, paused),
        Exchange::json("PUT", "/admin/realms/event", 204, json!({})),
    ]);
    windmill::tasks::migrate_registration_flows::migrate_realm_for_import(
        &peer.client(),
        &peer.public_client(),
        "event",
    )
    .await
    .unwrap();
    let requests = peer.finish();
    let updated = requests
        .iter()
        .find(|request| request.method == "PUT" && request.url.path() == "/admin/realms/event")
        .unwrap()
        .json();
    assert_eq!(
        updated["registrationAllowed"], false,
        "guard verification alone must not reopen an incomplete import"
    );
    assert_eq!(
        updated["attributes"]["enrollment_registration_restore"],
        "enabled"
    );
}

#[tokio::test]
async fn startup_guard_repair_does_not_finish_an_incomplete_import() {
    let mut realm:keycloak::types::RealmRepresentation=serde_json::from_value(json!({
        "realm":"event","registrationFlow":"registration","registrationAllowed":true,"attributes":{"other":"kept"}
    })).unwrap();
    windmill::tasks::migrate_registration_flows::mark_import_registration_pending(&mut realm, true);
    let paused = serde_json::to_value(realm).unwrap();
    let peer = HttpServer::start(vec![
        Exchange::json("GET", "/admin/realms/event", 200, paused.clone()),
        Exchange::json(
            "GET",
            "/admin/realms/event/authentication/flows/registration/executions",
            200,
            json!([
                {"providerId":"registration-page-form","authenticationFlow":true,"displayName":"form","flowId":"form-id","level":0},
                {"providerId":"enrollment-window-check","id":"gate","level":1,"requirement":"REQUIRED"},
                {"providerId":"registration-user-creation","id":"user","level":1,"requirement":"REQUIRED"}
            ]),
        ),
        Exchange::json("GET", "/admin/realms/event", 200, paused),
        Exchange::json("PUT", "/admin/realms/event", 204, json!({})),
    ]);
    migrate_realm(&peer.client(), &peer.public_client(), "event")
        .await
        .unwrap();
    let requests = peer.finish();
    let updated = requests
        .iter()
        .find(|request| request.method == "PUT" && request.url.path() == "/admin/realms/event")
        .unwrap()
        .json();
    assert_eq!(
        updated["registrationAllowed"], false,
        "startup cannot complete a failed import's external setup"
    );
    assert_eq!(
        updated["attributes"]["enrollment_registration_restore"],
        "import-enabled"
    );
}
