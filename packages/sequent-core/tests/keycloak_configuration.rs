// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Verify the configuration sent to Keycloak, including realm isolation and
//! preservation of settings the operator did not change.

#![cfg(feature = "keycloak")]

#[path = "support/http.rs"]
mod http;

use http::{Exchange, HttpServer};
use keycloak::types::{GroupRepresentation, RealmRepresentation};
use sequent_core::services::keycloak::*;
use serde_json::{json, Value};
use std::ffi::OsString;
use std::sync::{Mutex, MutexGuard};

// Environment configuration belongs to the process. Every test in this binary
// that changes it holds this guard; other test binaries have separate processes.
static ENVIRONMENT: Mutex<()> = Mutex::new(());
struct Environment {
    previous: Vec<(&'static str, Option<OsString>)>,
    _guard: MutexGuard<'static, ()>,
}
impl Environment {
    fn set(settings: &[(&'static str, Option<&str>)]) -> Self {
        let guard = ENVIRONMENT
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let previous = settings
            .iter()
            .map(|(key, value)| {
                let previous = std::env::var_os(key);
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
                (*key, previous)
            })
            .collect();
        Self {
            previous,
            _guard: guard,
        }
    }
}
impl Drop for Environment {
    fn drop(&mut self) {
        for (key, value) in &self.previous {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

const REALM_PATH: &str = "/admin/realms/tenant-north-event-mayor";

#[test]
fn realm_names_and_copy_replacements_keep_cross_references_consistent() {
    assert_eq!(get_tenant_realm("north"), "tenant-north");
    assert_eq!(
        get_event_realm("north", "mayor"),
        "tenant-north-event-mayor"
    );
    assert_eq!(parse_realm("tenant-north"), Some(("north".into(), None)));
    assert_eq!(
        parse_realm("tenant-north-event-mayor"),
        Some(("north".into(), Some("mayor".into())))
    );
    for malformed in [
        "master",
        "event-mayor",
        "tenant-event-mayor",
        "tenant-north-event",
    ] {
        assert_eq!(parse_realm(malformed), None);
    }
    let realm: RealmRepresentation =
        serde_json::from_value(json!({"realm": "tenant-north-event-mayor"}))
            .unwrap();
    assert_eq!(
        extract_realm_replacements(&realm, "south", &Some("council".into())),
        (
            Some(("north".into(), "south".into())),
            Some(("mayor".into(), "council".into()))
        )
    );
    assert_eq!(
        extract_realm_replacements(
            &RealmRepresentation::default(),
            "south",
            &None
        ),
        (None, None)
    );
    let same =
        extract_realm_replacements(&realm, "north", &Some("mayor".into()));
    assert_eq!(same.1, None);

    const ID: &str = "10000000-0000-4000-8000-000000000001";
    const KEPT: &str = "20000000-0000-4000-8000-000000000002";
    let source = json!({"realm": "tenant-north-event-mayor", "id": ID,
        "clients": [{"id": ID}], "authenticatorConfig": [{"config": {"reference": KEPT}}, {}]});
    let (copied, replacements) = replace_realm_ids(
        &source.to_string(),
        vec![],
        Some(("north".into(), "south".into())),
        Some(("mayor".into(), "council".into())),
    )
    .unwrap();
    let copied: Value = serde_json::from_str(&copied).unwrap();
    assert_eq!(copied["realm"], "tenant-south-event-council");
    assert_eq!(copied["id"], copied["clients"][0]["id"]);
    assert_ne!(copied["id"], ID);
    assert_eq!(
        copied["authenticatorConfig"][0]["config"]["reference"],
        KEPT
    );
    assert_eq!(replacements.len(), 1);
    assert!(replace_realm_ids("{", vec![], None, None).is_err());
    let secret = generate_client_secret();
    assert_eq!(secret.len(), 32);
    assert!(secret.bytes().all(|byte| byte.is_ascii_alphanumeric()));
}

#[rocket::async_test]
async fn realm_upsert_configures_client_redirects_and_replaces_copied_tenant_ids(
) {
    for exists in [true, false] {
        let peer = HttpServer::start(vec![
            Exchange::json(
                "GET",
                REALM_PATH,
                if exists { 200 } else { 404 },
                json!({}),
            ),
            Exchange::json(
                if exists { "PUT" } else { "POST" },
                if exists { REALM_PATH } else { "/admin/realms" },
                204,
                Value::Null,
            ),
        ]);
        let _environment = Environment::set(&[
            ("VOTING_PORTAL_URL", Some("https://voting.example.invalid")),
            (
                "KIOSK_VOTING_PORTAL_URL",
                Some("https://kiosk.example.invalid"),
            ),
            (
                "BALLOT_VERIFIER_URL",
                Some("https://verifier.example.invalid/"),
            ),
            (
                "RESULTS_PORTAL_URL",
                Some("https://results.example.invalid/"),
            ),
        ]);
        let mut clients = vec![
            json!({"clientId": "voting-portal", "id": "template-id", "protocolMappers": [{"id": "mapper-id"}]}),
            json!({"clientId": "voting-portal-kiosk"}),
            json!({"clientId": "onsite-voting-portal"}),
            json!({"clientId": "account"}),
            json!({"clientId": "unrelated", "rootUrl": "https://keep.example.invalid"}),
        ];
        if exists {
            clients.push(json!({"clientId": "results-portal"}));
        }
        let source = json!({"realm": "tenant-old-event-previous", "clients": clients,
            "users": [{"username": "voter", "attributes": {"locale": ["en"]}}],
            "authenticatorConfig": [{"config": {"conditional-client": "voting-portal"}}, {}]});
        peer.client()
            .upsert_realm(
                "tenant-north-event-mayor",
                &source.to_string(),
                "north",
                true,
                Some("Mayor election".into()),
                Some("mayor".into()),
            )
            .await
            .unwrap();
        let body = peer.finish()[1].json();
        assert_eq!(body["realm"], "tenant-north-event-mayor");
        assert_eq!(body["displayName"], "Mayor election");
        assert_eq!(
            body["users"][0]["attributes"]["tenant-id"],
            json!(["north"])
        );
        assert_eq!(body["users"][0]["attributes"]["locale"], json!(["en"]));
        let clients = body["clients"].as_array().unwrap();
        let find = |name| {
            clients
                .iter()
                .find(|client| client["clientId"] == name)
                .unwrap()
        };
        for name in ["voting-portal", "onsite-voting-portal"] {
            assert_eq!(find(name)["rootUrl"], "https://voting.example.invalid");
            assert_eq!(
                find(name)["baseUrl"],
                "https://voting.example.invalid/tenant/north/event/mayor/login"
            );
            assert_eq!(
                find(name)["redirectUris"],
                json!(["/*", "https://verifier.example.invalid/*"])
            );
        }
        assert_eq!(find("voting-portal-kiosk")["baseUrl"], "https://kiosk.example.invalid/tenant/north/event/mayor/login?kiosk");
        assert_eq!(
            find("results-portal")["redirectUris"],
            json!(["https://results.example.invalid/*"])
        );
        assert_eq!(
            find("account")["baseUrl"],
            find("voting-portal")["baseUrl"]
        );
        assert_eq!(
            find("unrelated")["rootUrl"],
            "https://keep.example.invalid"
        );
        assert_eq!(
            body["authenticatorConfig"][0]["config"]["conditional-client"],
            "voting-portal,results-portal"
        );
        if !exists {
            // Cloned client/mappers get fresh server IDs, not the template IDs.
            assert!(find("results-portal")["id"].is_null());
            assert!(
                find("results-portal")["protocolMappers"][0]["id"].is_null()
            );
        }
    }
}

#[rocket::async_test]
async fn rejected_realm_lookup_does_not_attempt_to_create_a_replacement() {
    let peer = HttpServer::start(vec![Exchange::json(
        "GET",
        REALM_PATH,
        403,
        json!({"error": "forbidden"}),
    )]);
    let _environment = Environment::set(&[
        ("VOTING_PORTAL_URL", Some("https://voting.example.invalid")),
        (
            "BALLOT_VERIFIER_URL",
            Some("https://verifier.example.invalid"),
        ),
        ("RESULTS_PORTAL_URL", None),
    ]);
    let result = peer
        .client()
        .upsert_realm(
            "tenant-north-event-mayor",
            "{}",
            "north",
            false,
            None,
            None,
        )
        .await;
    let requests = peer.finish();
    assert!(result.is_err());
    assert_eq!(
        requests.len(),
        1,
        "only a 404 authorizes the create path; a forbidden lookup must stop"
    );
}

#[rocket::async_test]
async fn realm_attribute_updates_preserve_unmentioned_values_and_secret_placeholders(
) {
    let saved = json!({"realm": "tenant-north-event-mayor", "attributes": {
        "label": "old", "keep": "preserved", "remove": "obsolete", "custom-secret": "synthetic-secret"}});
    let peer = HttpServer::start(vec![
        Exchange::json("GET", REALM_PATH, 200, saved.clone()),
        Exchange::json("GET", REALM_PATH, 200, saved),
        Exchange::json("PUT", REALM_PATH, 204, Value::Null),
    ]);
    let attributes = peer
        .client()
        .get_realm_attributes("tenant-north-event-mayor")
        .await
        .unwrap();
    assert_eq!(attributes["keep"], "preserved");
    let updates = [
        ("label".into(), "new".into()),
        ("remove".into(), "".into()),
        ("custom-secret".into(), REDACTED_ATTRIBUTE_VALUE.into()),
    ]
    .into();
    peer.client()
        .update_realm_attributes("tenant-north-event-mayor", updates)
        .await
        .unwrap();
    let body = peer.finish()[2].json();
    assert_eq!(body["attributes"]["keep"], "preserved");
    assert_eq!(body["attributes"]["label"], "new");
    assert_eq!(body["attributes"]["custom-secret"], "synthetic-secret");
    assert!(body["attributes"].get("remove").is_none());
}

#[rocket::async_test]
async fn password_policy_updates_preserve_unmanaged_rules_and_validate_before_http(
) {
    let saved = json!({"passwordPolicy": "hashIterations(27500) and length(12) and digits(2)"});
    let peer = HttpServer::start(vec![
        Exchange::json("GET", REALM_PATH, 200, saved.clone()),
        Exchange::json("GET", REALM_PATH, 200, saved),
        Exchange::json("PUT", REALM_PATH, 204, Value::Null),
    ]);
    let parsed = peer
        .client()
        .get_realm_password_policy("tenant-north-event-mayor")
        .await
        .unwrap();
    assert_eq!(parsed.required_digits, Some(2));
    let policy = RealmPasswordPolicy {
        configured: true,
        minimum_length: 16,
        maximum_length: 64,
        include_uppercase: true,
        include_lowercase: true,
        include_digits: false,
        include_special_characters: false,
    };
    let mut invalid = policy.clone();
    invalid.maximum_length = 1;
    assert!(peer
        .client()
        .update_realm_password_policy("tenant-north-event-mayor", invalid)
        .await
        .is_err());
    peer.client()
        .update_realm_password_policy("tenant-north-event-mayor", policy)
        .await
        .unwrap();
    let body = peer.finish()[2].json();
    let policies: Vec<_> = body["passwordPolicy"]
        .as_str()
        .unwrap()
        .split(" and ")
        .collect();
    assert!(policies.contains(&"hashIterations(27500)"));
    assert!(policies.contains(&"length(16)"));
    assert!(policies.contains(&"maxLength(64)"));
    assert!(!policies.contains(&"digits(2)"));
}

#[rocket::async_test]
async fn admin_credentials_cache_retries_failed_login_renews_expiring_tokens_and_reuses_valid_tokens(
) {
    let token_endpoint = "/realms/master/protocol/openid-connect/token";
    let mut expiring_token = http::token_json();
    expiring_token["expires_in"] = json!(5);
    let peer = HttpServer::start(vec![
        Exchange::json(
            "POST",
            token_endpoint,
            401,
            json!({"error": "invalid_grant"}),
        ),
        Exchange::json("POST", token_endpoint, 200, expiring_token),
        Exchange::json("POST", token_endpoint, 200, http::token_json()),
        Exchange::json("POST", token_endpoint, 200, http::token_json()),
        Exchange::json("POST", token_endpoint, 200, http::token_json()),
        Exchange::json(
            "GET",
            REALM_PATH,
            200,
            json!({"attributes": {"label": "current"}}),
        ),
        Exchange::json("GET", REALM_PATH, 200, json!({})),
        Exchange::json("PUT", REALM_PATH, 204, Value::Null),
        Exchange::json(
            "GET",
            REALM_PATH,
            200,
            json!({"passwordPolicy": "length(12) and digits(1)"}),
        ),
        Exchange::json("GET", REALM_PATH, 200, json!({})),
        Exchange::json("PUT", REALM_PATH, 204, Value::Null),
        Exchange::json("GET", REALM_PATH, 403, json!({})),
        Exchange::json("GET", REALM_PATH, 403, json!({})),
        Exchange::json("GET", REALM_PATH, 403, json!({})),
        Exchange::json("GET", REALM_PATH, 403, json!({})),
    ]);
    let _environment = Environment::set(&[
        ("KEYCLOAK_URL", Some(&peer.url)),
        ("KEYCLOAK_ADMIN_CLIENT_ID", Some("admin-client")),
        ("KEYCLOAK_ADMIN_CLIENT_SECRET", Some("synthetic-secret")),
        ("SUPER_ADMIN_TENANT_ID", Some("north")),
    ]);
    assert!(KeycloakAdminClient::new().await.is_err());
    KeycloakAdminClient::new().await.unwrap();
    // The five-second renewal margin makes this token immediately due for
    // renewal. No wall-clock sleeps or artificial cache mutation are needed.
    KeycloakAdminClient::new().await.unwrap();
    KeycloakAdminClient::new().await.unwrap();
    KeycloakAdminClient::new_requested().await.unwrap();
    KeycloakAdminClient::pub_new().await.unwrap();
    // The tenant/event convenience APIs must preserve scope and propagate
    // errors while sharing the same cached administrative credential.
    assert_eq!(
        get_realm_attributes("north", "mayor").await.unwrap()["label"],
        "current"
    );
    update_realm_attributes(
        "north",
        "mayor",
        [("label".into(), "new".into())].into(),
    )
    .await
    .unwrap();
    assert_eq!(
        get_realm_password_policy("north", "mayor")
            .await
            .unwrap()
            .minimum_length,
        Some(12)
    );
    update_realm_password_policy(
        "north",
        "mayor",
        RealmPasswordPolicy::default(),
    )
    .await
    .unwrap();
    assert!(get_realm_attributes("north", "mayor").await.is_err());
    assert!(
        update_realm_attributes("north", "mayor", Default::default())
            .await
            .is_err()
    );
    assert!(get_realm_password_policy("north", "mayor").await.is_err());
    assert!(update_realm_password_policy(
        "north",
        "mayor",
        RealmPasswordPolicy::default()
    )
    .await
    .is_err());
    let requests = peer.finish();
    let token_requests: Vec<_> = requests
        .iter()
        .filter(|request| request.url.path() == token_endpoint)
        .collect();
    assert_eq!(
        token_requests.len(),
        5,
        "cached operations make no extra token requests"
    );
    for request in token_requests {
        let form: std::collections::HashMap<String, String> =
            serde_urlencoded::from_str(&request.body).unwrap();
        // Admin construction uses master/admin-cli username/password login.
        assert_eq!(form["client_id"], "admin-cli");
        assert_eq!(form["username"], "admin-client");
        assert_eq!(form["password"], "synthetic-secret");
        assert_eq!(form["grant_type"], "password");
    }
}

#[rocket::async_test]
async fn fresh_admin_clients_propagate_denied_authentication() {
    let endpoint = "/realms/master/protocol/openid-connect/token";
    let peer = HttpServer::start(vec![
        Exchange::json(
            "POST",
            endpoint,
            401,
            json!({"error": "invalid_grant"}),
        ),
        Exchange::json(
            "POST",
            endpoint,
            401,
            json!({"error": "invalid_grant"}),
        ),
        Exchange::json("POST", endpoint, 200, http::token_json()),
        Exchange::json("POST", endpoint, 200, http::token_json()),
    ]);
    let _environment = Environment::set(&[
        ("KEYCLOAK_URL", Some(&peer.url)),
        ("KEYCLOAK_ADMIN_CLIENT_ID", Some("admin-client")),
        ("KEYCLOAK_ADMIN_CLIENT_SECRET", Some("synthetic-secret")),
        ("SUPER_ADMIN_TENANT_ID", Some("north")),
    ]);
    assert!(KeycloakAdminClient::new_requested().await.is_err());
    assert!(KeycloakAdminClient::pub_new().await.is_err());
    KeycloakAdminClient::new_requested().await.unwrap();
    let public = KeycloakAdminClient::pub_new().await.unwrap();
    assert_eq!(public.url, peer.url);
    assert_eq!(peer.finish().len(), 4);
}

#[rocket::async_test]
async fn malformed_successful_token_responses_fail_without_returning_secret_material(
) {
    let endpoint = "/realms/tenant-north/protocol/openid-connect/token";
    let malformed = json!({"access_token": "synthetic-private-token", "expires_in": "invalid"});
    let peer = HttpServer::start(vec![
        Exchange::json("POST", endpoint, 200, malformed.clone()),
        Exchange::json("POST", endpoint, 200, malformed),
        Exchange::json("POST", endpoint, 200, http::token_json()),
    ]);
    let _environment = Environment::set(&[
        ("KEYCLOAK_URL", Some(&peer.url)),
        ("KEYCLOAK_CLIENT_ID", Some("party")),
        ("KEYCLOAK_CLIENT_SECRET", Some("synthetic-secret")),
        ("SUPER_ADMIN_TENANT_ID", Some("north")),
    ]);
    let headers_error = get_client_credentials().await.err().unwrap();
    let party_error = get_third_party_client_access_token(
        "party".into(),
        "synthetic-secret".into(),
        "north".into(),
    )
    .await
    .unwrap_err();
    for error in [headers_error, party_error] {
        assert_eq!(error.to_string(), "Invalid Keycloak token response");
        assert!(!format!("{error:?}").contains("synthetic-private-token"));
    }
    assert_eq!(
        get_client_credentials().await.unwrap().value,
        "Bearer synthetic-access-token"
    );
    assert_eq!(peer.finish().len(), 3);
}

#[rocket::async_test]
async fn group_update_and_creation_use_the_server_assigned_user_id() {
    let token_endpoint = "/realms/master/protocol/openid-connect/token";
    let peer = HttpServer::start(vec![
        Exchange::json("POST", token_endpoint, 200, http::token_json()),
        Exchange::json("PUT", "/admin/realms/tenant-north/groups/group-1", 204, Value::Null),
        Exchange::json("POST", token_endpoint, 200, http::token_json()),
        Exchange::json("POST", "/admin/realms/tenant-north/users", 201, Value::Null)
            .header("Location", "https://identity.invalid/admin/realms/tenant-north/users/server-id"),
        Exchange::json("GET", "/admin/realms/tenant-north/users/server-id", 200, json!({"id": "server-id", "username": "new-voter"})),
    ]);
    let _environment = Environment::set(&[
        ("KEYCLOAK_URL", Some(&peer.url)),
        ("KEYCLOAK_ADMIN_CLIENT_ID", Some("admin-client")),
        ("KEYCLOAK_ADMIN_CLIENT_SECRET", Some("synthetic-secret")),
        ("SUPER_ADMIN_TENANT_ID", Some("north")),
    ]);
    let group: GroupRepresentation =
        serde_json::from_value(json!({"id": "group-1", "name": "Clerks"}))
            .unwrap();
    peer.client().update_group("north", &group).await.unwrap();
    let user = serde_json::from_value(
        json!({"id": "discard-input-id", "username": "new-voter"}),
    )
    .unwrap();
    let created = peer
        .client()
        .create_user(
            "tenant-north",
            &user,
            Some([("area-id".into(), vec!["north".into()])].into()),
            Some(vec!["Clerks".into()]),
        )
        .await
        .unwrap();
    assert_eq!(created.id.as_deref(), Some("server-id"));
    let requests = peer.finish();
    assert_eq!(requests[1].json()["name"], "Clerks");
    assert!(requests[3].json()["id"].is_null());
    assert_eq!(requests[3].json()["groups"], json!(["Clerks"]));
}

#[rocket::async_test]
async fn rejected_user_creation_preserves_validation_errors_and_stops_before_lookup(
) {
    let token_endpoint = "/realms/master/protocol/openid-connect/token";
    let users = "/admin/realms/tenant-north/users";
    let peer = HttpServer::start(vec![
        Exchange::json("POST", token_endpoint, 200, http::token_json()),
        Exchange::json(
            "POST",
            users,
            400,
            json!({
                "field": "email", "errorMessage": "invalid-email", "params": ["email"]
            }),
        ),
        Exchange::json("POST", token_endpoint, 200, http::token_json()),
        Exchange::json("POST", users, 503, Value::Null)
            .body("identity provider unavailable"),
    ]);
    let _environment = Environment::set(&[
        ("KEYCLOAK_URL", Some(&peer.url)),
        ("KEYCLOAK_ADMIN_CLIENT_ID", Some("admin-client")),
        ("KEYCLOAK_ADMIN_CLIENT_SECRET", Some("synthetic-secret")),
        ("SUPER_ADMIN_TENANT_ID", Some("north")),
    ]);
    let user = serde_json::from_value(
        json!({"username": "new-voter", "email": "bad-email"}),
    )
    .unwrap();
    let error = peer
        .client()
        .create_user("tenant-north", &user, None, None)
        .await
        .unwrap_err();
    assert!(is_keycloak_bad_request(&error));
    let validation = get_user_profile_validation_errors(&error);
    assert_eq!(validation.len(), 1);
    assert_eq!(validation[0].field.as_deref(), Some("email"));
    assert_eq!(
        validation[0].error_message.as_deref(),
        Some("invalid-email")
    );
    assert_eq!(validation[0].params, Some(vec![json!("email")]));
    let error = peer
        .client()
        .create_user("tenant-north", &user, None, None)
        .await
        .unwrap_err();
    assert!(!is_keycloak_bad_request(&error));
    assert!(get_user_profile_validation_errors(&error).is_empty());
    assert!(matches!(error.downcast_ref::<keycloak::KeycloakError>(),
        Some(keycloak::KeycloakError::HttpFailure { status: 503, text, .. }) if text == "identity provider unavailable"));
    let requests = peer.finish();
    assert_eq!(requests.len(), 4);
    assert!(requests.iter().all(|request| request.method == "POST"));
}

#[rocket::async_test]
async fn user_creation_requires_a_valid_location_before_reading_back_the_user()
{
    let token_endpoint = "/realms/master/protocol/openid-connect/token";
    let users = "/admin/realms/tenant-north/users";
    let locations = [
        None,
        Some("not-a-url"),
        Some("https://identity.invalid/admin/realms/tenant-north/users/"),
        Some("https://identity.invalid/admin/realms/tenant-north/groups/group-1"),
        Some("https://identity.invalid/admin/realms/tenant-north/users/voter-1?lookup=other"),
        Some("https://identity.invalid/admin/realms/tenant-north/users/voter-1#other"),
    ];
    let mut exchanges = vec![];
    for location in locations {
        exchanges.push(Exchange::json(
            "POST",
            token_endpoint,
            200,
            http::token_json(),
        ));
        let mut response = Exchange::json("POST", users, 201, Value::Null);
        if let Some(location) = location {
            response = response.header("Location", location);
        }
        exchanges.push(response);
    }
    let peer = HttpServer::start(exchanges);
    let _environment = Environment::set(&[
        ("KEYCLOAK_URL", Some(&peer.url)),
        ("KEYCLOAK_ADMIN_CLIENT_ID", Some("admin-client")),
        ("KEYCLOAK_ADMIN_CLIENT_SECRET", Some("synthetic-secret")),
        ("SUPER_ADMIN_TENANT_ID", Some("north")),
    ]);
    let user =
        serde_json::from_value(json!({"username": "new-voter"})).unwrap();
    for location in locations {
        assert!(
            peer.client()
                .create_user("tenant-north", &user, None, None)
                .await
                .is_err(),
            "accepted unusable user location {location:?}"
        );
    }
    let requests = peer.finish();
    assert_eq!(requests.len(), locations.len() * 2);
    assert!(requests.iter().all(|request| request.method == "POST"),
        "without a usable id, creation must not search for or return a different user");
}

#[rocket::async_test]
async fn group_updates_report_rejection_and_require_an_identifier() {
    let token_endpoint = "/realms/master/protocol/openid-connect/token";
    let group_path = "/admin/realms/tenant-north/groups/group-1";
    let peer = HttpServer::start(vec![
        Exchange::json("POST", token_endpoint, 200, http::token_json()),
        Exchange::json("PUT", group_path, 403, json!({"error": "forbidden"})),
        Exchange::json("POST", token_endpoint, 200, http::token_json()),
        Exchange::json("PUT", group_path, 204, Value::Null),
        // The current implementation authenticates before validating the id.
        Exchange::json("POST", token_endpoint, 200, http::token_json()),
    ]);
    let _environment = Environment::set(&[
        ("KEYCLOAK_URL", Some(&peer.url)),
        ("KEYCLOAK_ADMIN_CLIENT_ID", Some("admin-client")),
        ("KEYCLOAK_ADMIN_CLIENT_SECRET", Some("synthetic-secret")),
        ("SUPER_ADMIN_TENANT_ID", Some("north")),
    ]);
    let mut group: GroupRepresentation =
        serde_json::from_value(json!({"id": "group-1", "name": "Clerks"}))
            .unwrap();
    let error = peer
        .client()
        .update_group("north", &group)
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "Failed to update group");
    peer.client().update_group("north", &group).await.unwrap();
    group.id = None;
    assert!(peer.client().update_group("north", &group).await.is_err());
    let requests = peer.finish();
    assert_eq!(requests.len(), 5);
    assert_eq!(requests[1].json(), requests[3].json());
}

#[rocket::async_test]
async fn realm_upsert_propagates_rejected_creation_and_updates() {
    for exists in [true, false] {
        let peer = HttpServer::start(vec![
            Exchange::json(
                "GET",
                REALM_PATH,
                if exists { 200 } else { 404 },
                json!({}),
            ),
            Exchange::json(
                if exists { "PUT" } else { "POST" },
                if exists { REALM_PATH } else { "/admin/realms" },
                403,
                json!({"error": "forbidden"}),
            ),
        ]);
        let _environment = Environment::set(&[
            ("VOTING_PORTAL_URL", Some("https://voting.example.invalid")),
            (
                "BALLOT_VERIFIER_URL",
                Some("https://verifier.example.invalid"),
            ),
            ("RESULTS_PORTAL_URL", None),
        ]);
        let error = peer
            .client()
            .upsert_realm(
                "tenant-north-event-mayor",
                "{}",
                "north",
                false,
                None,
                None,
            )
            .await
            .unwrap_err();
        assert!(error.to_string().contains("403"));
        let requests = peer.finish();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[1].json()["realm"], "tenant-north-event-mayor");
    }
}

#[rocket::async_test]
async fn realm_import_without_a_client_template_cannot_create_a_results_client()
{
    let peer = HttpServer::start(vec![Exchange::json(
        "GET",
        REALM_PATH,
        404,
        json!({}),
    )]);
    let _environment = Environment::set(&[
        ("VOTING_PORTAL_URL", Some("https://voting.example.invalid")),
        (
            "BALLOT_VERIFIER_URL",
            Some("https://verifier.example.invalid"),
        ),
        (
            "RESULTS_PORTAL_URL",
            Some("https://results.example.invalid"),
        ),
    ]);
    let error = peer
        .client()
        .upsert_realm(
            "tenant-north-event-mayor",
            r#"{"clients":[{"clientId":"unrelated"}]}"#,
            "north",
            false,
            None,
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Event realm does not contain a voting portal client template"
    );
    let requests = peer.finish();
    assert_eq!(
        requests.len(),
        1,
        "invalid provisioning input must not be written"
    );
    assert_eq!(requests[0].method, "GET");
}

/// Capture logs in memory so confidentiality assertions inspect the same spans
/// and events an operator would receive. Synthetic secrets never leave the VM.
#[derive(Clone)]
struct LogWriter(std::sync::Arc<Mutex<Vec<u8>>>);
impl std::io::Write for LogWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[rocket::async_test]
async fn tenant_credentials_are_encoded_correctly_and_never_disclosed_in_logs_or_errors(
) {
    use tracing::instrument::WithSubscriber;
    let endpoint = "/realms/tenant-north/protocol/openid-connect/token";
    let peer = HttpServer::start(vec![
        Exchange::json("POST", endpoint, 200, http::token_json()),
        Exchange::json("POST", endpoint, 200, http::token_json()),
        Exchange::json("POST", endpoint, 200, http::token_json()),
        Exchange::json(
            "POST",
            endpoint,
            200,
            json!({"access_token": "synthetic-private-token"}),
        ),
    ]);
    let _environment = Environment::set(&[
        ("KEYCLOAK_URL", Some(&peer.url)),
        ("KEYCLOAK_CLIENT_ID", Some("service & client")),
        ("KEYCLOAK_CLIENT_SECRET", Some("synthetic & secret+value")),
        ("SUPER_ADMIN_TENANT_ID", Some("north")),
    ]);
    let log = LogWriter(Default::default());
    let writer = log.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::NEW)
        .with_writer(move || writer.clone())
        .finish();
    async {
        let headers = get_client_credentials().await.unwrap();
        assert_eq!(headers.key, "authorization");
        assert_eq!(headers.value, "Bearer synthetic-access-token");
        let public: PubKeycloakAdminToken =
            get_auth_credentials().await.unwrap().try_into().unwrap();
        assert_eq!(public.access_token, "synthetic-access-token");
        let token = get_third_party_client_access_token(
            "party".into(),
            "synthetic & secret+value".into(),
            "north".into(),
        )
        .await
        .unwrap();
        let public: PubKeycloakAdminToken = token.try_into().unwrap();
        assert_eq!(public.token_type, "Bearer");
        let error = get_auth_credentials().await.unwrap_err();
        assert!(
            !format!("{error:?}").contains("synthetic-private-token"),
            "malformed token bodies must not enter caller errors"
        );
    }
    .with_subscriber(subscriber)
    .await;
    let log = String::from_utf8(log.0.lock().unwrap().clone()).unwrap();
    assert!(
        !log.is_empty(),
        "the test must capture real application events"
    );
    for secret in [
        "synthetic & secret+value",
        "synthetic+%26+secret%2Bvalue",
        "synthetic-access-token",
        "synthetic-private-token",
    ] {
        assert!(
            !log.contains(secret),
            "credential material appeared in application logs"
        );
    }
    let requests = peer.finish();
    for request in requests {
        let form: std::collections::HashMap<String, String> =
            serde_urlencoded::from_str(&request.body).unwrap();
        assert_eq!(form["scope"], "openid");
        assert_eq!(form["grant_type"], "client_credentials");
        assert_eq!(form["client_secret"], "synthetic & secret+value");
    }
}

#[rocket::async_test]
async fn a_token_shaped_error_response_is_still_an_authentication_failure() {
    let peer = HttpServer::start(vec![Exchange::json(
        "POST",
        "/realms/tenant-north/protocol/openid-connect/token",
        401,
        http::token_json(),
    )]);
    let _environment = Environment::set(&[("KEYCLOAK_URL", Some(&peer.url))]);
    let result = get_third_party_client_access_token(
        "party".into(),
        "synthetic-secret".into(),
        "north".into(),
    )
    .await;
    peer.finish();
    assert!(
        result.is_err(),
        "the HTTP status must be checked before accepting a token body"
    );
}

/// Return the identities that the guard would pass to an application handler.
/// The local issuer supplies a synthetic JWT; this does not test JWT signatures.
#[rocket::get("/datafix")]
fn guarded_datafix(
    claims: sequent_core::services::connection::DatafixClaims,
) -> String {
    format!(
        "{}|{}|{}",
        claims.tenant_id, claims.datafix_event_id, claims.jwt_claims.sub
    )
}

fn datafix_token(subject: &str, expires_in: u64) -> Value {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    let claims = json!({"exp": 2000000000, "iat": 0, "jti": "test", "iss": "local-test-issuer",
        "sub": subject, "typ": "Bearer", "azp": "datafix", "acr": "1", "allowed-origins": [],
        "scope": "openid", "email_verified": false, "https://hasura.io/jwt/claims": {
            "x-hasura-default-role": "user", "x-hasura-tenant-id": "north",
            "x-hasura-user-id": subject, "x-hasura-allowed-roles": ["user"]}});
    json!({"access_token": format!("fixture.{}.unsigned", URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap())),
        "expires_in": expires_in, "token_type": "Bearer", "scope": "openid"})
}

#[rocket::async_test]
async fn datafix_cache_is_bound_to_credentials_and_tenant_and_renews_before_expiry(
) {
    use rocket::{
        http::{Header, Status},
        local::asynchronous::Client,
    };
    use sequent_core::services::connection::LastDatafixAccessToken;
    let north = "/realms/tenant-north/protocol/openid-connect/token";
    let south = "/realms/tenant-south/protocol/openid-connect/token";
    let peer = HttpServer::start(vec![
        Exchange::json("POST", north, 200, datafix_token("first", 300)),
        Exchange::json("POST", north, 200, datafix_token("rotated", 300)),
        Exchange::json("POST", north, 200, datafix_token("new-client", 300)),
        Exchange::json("POST", south, 200, datafix_token("new-tenant", 5)),
        Exchange::json("POST", south, 200, datafix_token("renewed", 300)),
        Exchange::json("POST", north, 401, json!({"error": "invalid_client"})),
        Exchange::json("POST", north, 200, http::token_json()),
    ]);
    let _environment = Environment::set(&[("KEYCLOAK_URL", Some(&peer.url))]);
    let client = Client::tracked(
        rocket::build()
            .manage(LastDatafixAccessToken::init())
            .mount("/", rocket::routes![guarded_datafix]),
    )
    .await
    .unwrap();

    // A repeat can reuse a token. Rotating a secret, changing client or changing
    // tenant must contact the issuer again. Five seconds is the renewal margin,
    // so expiry can be tested deterministically without sleeping.
    for (tenant, credentials, expected_subject) in [
        ("north", "party:synthetic-one", "first"),
        ("north", "party:synthetic-one", "first"),
        ("north", "party:synthetic-two", "rotated"),
        ("north", "other:synthetic-two", "new-client"),
        ("south", "other:synthetic-two", "new-tenant"),
        ("south", "other:synthetic-two", "renewed"),
    ] {
        let response = client
            .get("/datafix")
            .header(Header::new("tenant-id", tenant))
            .header(Header::new("event-id", "mayor"))
            .header(Header::new("authorization", credentials))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(
            response.into_string().await.unwrap(),
            format!("{tenant}|mayor|{expected_subject}")
        );
    }

    // Neither a rejected credential nor a token with malformed claims reaches
    // the handler, even after a previous request has populated the cache.
    for credentials in ["rejected:synthetic-three", "malformed:synthetic-four"]
    {
        assert_eq!(
            client
                .get("/datafix")
                .header(Header::new("tenant-id", "north"))
                .header(Header::new("event-id", "mayor"))
                .header(Header::new("authorization", credentials))
                .dispatch()
                .await
                .status(),
            Status::Unauthorized
        );
    }
    let requests = peer.finish();
    assert_eq!(
        requests.len(),
        7,
        "exactly one request should use the cached token"
    );
    for (request, expected_secret) in requests.iter().zip([
        "synthetic-one",
        "synthetic-two",
        "synthetic-two",
        "synthetic-two",
        "synthetic-two",
        "synthetic-three",
        "synthetic-four",
    ]) {
        let form: std::collections::HashMap<String, String> =
            serde_urlencoded::from_str(&request.body).unwrap();
        assert_eq!(form["client_secret"], expected_secret);
    }
}

#[rocket::async_test]
async fn request_guard_diagnostics_never_log_authorization_headers_or_client_secrets(
) {
    use rocket::{
        http::{Header, Status},
        local::asynchronous::Client,
    };
    use sequent_core::services::connection::LastDatafixAccessToken;
    use tracing::instrument::WithSubscriber;
    let peer = HttpServer::start(vec![Exchange::json(
        "POST",
        "/realms/tenant-north/protocol/openid-connect/token",
        200,
        datafix_token("voter", 300),
    )]);
    let _environment = Environment::set(&[("KEYCLOAK_URL", Some(&peer.url))]);
    let log = LogWriter(Default::default());
    let writer = log.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::NEW)
        .with_writer(move || writer.clone())
        .finish();
    async {
        let client = Client::tracked(
            rocket::build()
                .manage(LastDatafixAccessToken::init())
                .mount("/", rocket::routes![guarded_datafix]),
        )
        .await
        .unwrap();
        // Both the cache miss and hit create instrumented spans. An incomplete
        // request also exercises the diagnostic path before authentication.
        for _ in 0..2 {
            assert_eq!(
                client
                    .get("/datafix")
                    .header(Header::new("tenant-id", "north"))
                    .header(Header::new("event-id", "mayor"))
                    .header(Header::new(
                        "authorization",
                        "party:synthetic-private-secret"
                    ))
                    .dispatch()
                    .await
                    .status(),
                Status::Ok
            );
        }
        assert_eq!(
            client
                .get("/datafix")
                .header(Header::new(
                    "authorization",
                    "party:synthetic-incomplete-secret"
                ))
                .dispatch()
                .await
                .status(),
            Status::BadRequest
        );
    }
    .with_subscriber(subscriber)
    .await;
    peer.finish();
    let log = String::from_utf8(log.0.lock().unwrap().clone()).unwrap();
    assert!(
        log.contains("DatafixClaims"),
        "capture application diagnostics, not an empty log"
    );
    for secret in ["synthetic-private-secret", "synthetic-incomplete-secret"] {
        assert!(
            !log.contains(secret),
            "request diagnostics disclosed credentials"
        );
    }
}
