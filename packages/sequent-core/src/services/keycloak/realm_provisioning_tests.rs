// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use keycloak::KeycloakAdminToken;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

fn admin(url: &str) -> KeycloakAdmin<KeycloakAdminToken> {
    let token = serde_json::from_value(json!({
        "access_token": "synthetic-token", "expires_in": 300,
        "scope": "openid", "token_type": "Bearer"
    }))
    .unwrap();
    KeycloakAdmin::new(url, token, reqwest::Client::new())
}

fn imported(enabled: Option<bool>) -> RealmRepresentation {
    let mut client = json!({
        "clientId": "api-key-client", "secret": "synthetic-random-credential",
        "serviceAccountsEnabled": true,
        "attributes": {"integration": "imported"}
    });
    if let Some(enabled) = enabled {
        client["enabled"] = json!(enabled);
    }
    serde_json::from_value(json!({
        "clients": [client, {"clientId": "voting-portal", "enabled": true}],
        "users": [{"username": "service-account-api-key-client",
                   "serviceAccountClientId": "api-key-client"},
                  {"username": "admin"}]
    }))
    .unwrap()
}

fn api_client(realm: &RealmRepresentation) -> Option<Value> {
    realm
        .clients
        .as_ref()
        .unwrap()
        .iter()
        .find(|c| c.client_id.as_deref() == Some(API_KEY_CLIENT_ID))
        .map(|c| serde_json::to_value(c).unwrap())
}

// One expected read, bounded even when the implementation never sends it.
fn serve(status: u16, response: Value) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "expected client lookup"
                    );
                    thread::sleep(Duration::from_millis(5));
                }
                Err(e) => panic!("accept: {e}"),
            }
        };
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut request = Vec::new();
        let mut buffer = [0; 1024];
        while !request.windows(4).any(|w| w == b"\r\n\r\n") {
            let n = stream.read(&mut buffer).unwrap();
            assert!(n > 0);
            request.extend_from_slice(&buffer[..n]);
        }
        let body = response.to_string();
        write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        String::from_utf8(request)
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .to_string()
    });
    (url, worker)
}

#[rocket::async_test]
async fn fresh_tenant_and_event_ignore_imported_enablement_and_keep_credentials(
) {
    for name in ["tenant-test", "tenant-test-event-test"] {
        for enabled in [Some(true), Some(false), None] {
            let mut realm = imported(enabled);
            let other = realm.clients.as_ref().unwrap()[1].clone();
            prepare_api_key_client(
                &admin("http://127.0.0.1:1"),
                name,
                &mut realm,
                RealmProvisioning::Create,
            )
            .await
            .unwrap();
            let client = api_client(&realm).unwrap();
            assert_eq!(client["enabled"], false);
            assert_eq!(client["secret"], "synthetic-random-credential");
            assert_eq!(client["serviceAccountsEnabled"], true);
            assert_eq!(realm.clients.as_ref().unwrap()[1], other);
        }
    }
}

#[rocket::async_test]
async fn fresh_client_without_secret_leaves_generation_to_keycloak() {
    let mut realm = imported(Some(true));
    realm.clients.as_mut().unwrap()[0].secret = None;
    prepare_api_key_client(
        &admin("http://127.0.0.1:1"),
        "tenant",
        &mut realm,
        RealmProvisioning::Create,
    )
    .await
    .unwrap();
    let client = api_client(&realm).unwrap();
    assert_eq!(client["enabled"], false);
    assert!(client.get("secret").map_or(true, Value::is_null));
}

#[rocket::async_test]
async fn existing_enabled_or_disabled_client_is_never_sent_for_update() {
    for current in [true, false] {
        for imported_enabled in [Some(true), Some(false), None] {
            let (url, worker) = serve(
                200,
                json!([{"id":"existing-id", "clientId":"api-key-client", "enabled":current}]),
            );
            let mut realm = imported(imported_enabled);
            prepare_api_key_client(
                &admin(&url),
                "tenant",
                &mut realm,
                RealmProvisioning::Update,
            )
            .await
            .unwrap();
            assert_eq!(worker.join().unwrap(), "GET /admin/realms/tenant/clients?clientId=api-key-client&search=false HTTP/1.1");
            assert!(
                api_client(&realm).is_none(),
                "do not send enablement, secrets or integration settings"
            );
            assert_eq!(realm.clients.as_ref().unwrap().len(), 1);
            assert_eq!(realm.users.as_ref().unwrap().len(), 1);
            assert_eq!(
                realm.users.as_ref().unwrap()[0].username.as_deref(),
                Some("admin")
            );
            // Reapplying the prepared import cannot reintroduce the client or call Keycloak.
            let previous = serde_json::to_value(&realm).unwrap();
            prepare_api_key_client(
                &admin("http://127.0.0.1:1"),
                "tenant",
                &mut realm,
                RealmProvisioning::Update,
            )
            .await
            .unwrap();
            assert_eq!(serde_json::to_value(&realm).unwrap(), previous);
        }
    }
}

#[rocket::async_test]
async fn newly_added_client_in_existing_realm_is_disabled() {
    let (url, worker) = serve(200, json!([]));
    let mut realm = imported(Some(true));
    prepare_api_key_client(
        &admin(&url),
        "tenant",
        &mut realm,
        RealmProvisioning::Update,
    )
    .await
    .unwrap();
    worker.join().unwrap();
    assert_eq!(api_client(&realm).unwrap()["enabled"], false);
}

#[rocket::async_test]
async fn lookup_errors_do_not_apply_new_client_defaults() {
    for status in [403, 500] {
        let (url, worker) =
            serve(status, json!({"error":"synthetic lookup failure"}));
        let mut realm = imported(Some(true));
        let previous = serde_json::to_value(&realm).unwrap();
        let error = prepare_api_key_client(
            &admin(&url),
            "tenant",
            &mut realm,
            RealmProvisioning::Update,
        )
        .await
        .unwrap_err();
        worker.join().unwrap();
        assert!(
            matches!(error, KeycloakError::HttpFailure {status: s, ..} if s == status)
        );
        assert_eq!(serde_json::to_value(&realm).unwrap(), previous);
    }
}

#[rocket::async_test]
async fn absent_api_client_does_not_change_other_clients_or_query_keycloak() {
    for provisioning in [RealmProvisioning::Create, RealmProvisioning::Update] {
        let mut realm: RealmRepresentation =
            serde_json::from_value(json!({"clients":[
                {"clientId":"admin-portal", "enabled":true},
                {"clientId":"voting-portal", "enabled":true},
                {"clientId":"ivr-service", "enabled":true},
                {"clientId":"datafix-account", "enabled":true},
                {"clientId":"service-account", "enabled":true}
            ]}))
            .unwrap();
        let previous = serde_json::to_value(&realm).unwrap();
        prepare_api_key_client(
            &admin("http://127.0.0.1:1"),
            "event",
            &mut realm,
            provisioning,
        )
        .await
        .unwrap();
        assert_eq!(serde_json::to_value(&realm).unwrap(), previous);
    }
}

#[test]
fn admin_events_default_on_without_details_and_preserve_login_event_policy() {
    for admin in [Some(true), Some(false), None] {
        for details in [Some(true), Some(false), None] {
            for login in [Some(true), Some(false), None] {
                let mut realm = RealmRepresentation {
                    admin_events_enabled: admin,
                    admin_events_details_enabled: details,
                    events_enabled: login,
                    ..Default::default()
                };
                prepare_admin_event_defaults(
                    &mut realm,
                    RealmProvisioning::Create,
                );
                assert_eq!(realm.admin_events_enabled, Some(true));
                assert_eq!(realm.admin_events_details_enabled, Some(false));
                assert_eq!(realm.events_enabled, login);
            }
        }
    }
}

#[test]
fn existing_admin_event_policy_is_omitted_from_generic_updates() {
    for admin in [Some(true), Some(false), None] {
        for details in [Some(true), Some(false), None] {
            let mut realm = RealmRepresentation {
                admin_events_enabled: admin,
                admin_events_details_enabled: details,
                events_enabled: Some(false),
                ..Default::default()
            };
            prepare_admin_event_defaults(&mut realm, RealmProvisioning::Update);
            let wire = serde_json::to_value(&realm).unwrap();
            assert!(wire.get("adminEventsEnabled").is_none());
            assert!(wire.get("adminEventsDetailsEnabled").is_none());
            assert_eq!(wire["eventsEnabled"], false);
        }
    }
}
