// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Credentials and voter data stay out of the logs: the request guards log
//! which credential is missing, never the headers; decoded claims are not
//! logged; and voter searches and edits reach Keycloak while their search
//! text, names, emails, usernames and attribute values stay out of the logs
//! and errors.

#![cfg(all(feature = "keycloak", feature = "default_features"))]

#[path = "support/http.rs"]
mod http;
#[path = "support/logs.rs"]
mod logs;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use http::{token_json, Exchange, HttpServer};
use keycloak::KeycloakAdmin;
use logs::with_captured_logs;
use rocket::http::{Header, Status};
use rocket::local::asynchronous::Client;
use sequent_core::services::connection::AuthHeaders;
use sequent_core::services::jwt::{decode_jwt, JwtClaims};
use sequent_core::services::keycloak::KeycloakAdminClient;
use serde_json::{json, Value};
use std::collections::HashMap;

const ADMIN_SECRET: &str = "AdminSecretValue4f1c";
const COOKIE: &str = "CookieValue9b2e";
const BASIC_CREDENTIALS: &str = "BasicCredentials7d3a";

#[rocket::get("/claims")]
fn claims(_claims: JwtClaims) {}

#[rocket::get("/headers")]
fn headers(_headers: AuthHeaders) {}

#[rocket::async_test]
async fn request_guards_log_no_header_values() {
    let client = Client::untracked(
        rocket::build().mount("/", rocket::routes![claims, headers]),
    )
    .await
    .expect("rocket client");

    let (statuses, logs) = with_captured_logs(async {
        let cookie = Header::new("Cookie", format!("session={COOKIE}"));
        vec![
            // No authorization header, but Hasura's admin secret.
            client
                .get("/claims")
                .header(Header::new("X-Hasura-Admin-Secret", ADMIN_SECRET))
                .header(cookie.clone())
                .dispatch()
                .await
                .status(),
            client
                .get("/claims")
                .header(Header::new(
                    "Authorization",
                    format!("Basic {BASIC_CREDENTIALS}"),
                ))
                .dispatch()
                .await
                .status(),
            client
                .get("/headers")
                .header(cookie)
                .dispatch()
                .await
                .status(),
        ]
    })
    .await;

    assert_eq!(statuses, vec![Status::Unauthorized; 3]);
    assert!(logs.contains("JwtClaims guard"), "{logs}");
    assert!(logs.contains("AuthHeaders guard"), "{logs}");
    for value in [ADMIN_SECRET, COOKIE, BASIC_CREDENTIALS] {
        assert!(!logs.contains(value), "{value} in logs: {logs}");
    }
}

#[rocket::async_test]
async fn decoded_claims_are_not_logged() {
    const NAME: &str = "VoterNameValue2c7d";
    const EMAIL: &str = "voter-email-value-8a1f@example.test";
    let payload = json!({
        "exp": 0, "iat": 0, "jti": "jti", "iss": "issuer", "sub": "subject",
        "typ": "Bearer", "azp": "voting-portal", "acr": "1",
        "allowed-origins": [], "scope": "openid", "email_verified": true,
        "https://hasura.io/jwt/claims": {
            "x-hasura-default-role": "user", "x-hasura-tenant-id": "tenant",
            "x-hasura-user-id": "user", "x-hasura-allowed-roles": ["user"]
        },
        "name": NAME,
        "email": EMAIL
    });
    let token = format!(
        "header.{}.signature",
        URL_SAFE_NO_PAD.encode(payload.to_string())
    );

    let (claims, logs) = with_captured_logs(async { decode_jwt(&token) }).await;

    assert_eq!(claims.expect("claims").name.as_deref(), Some(NAME));
    assert!(logs.contains("decode_jwt"), "{logs}");
    for value in [NAME, EMAIL] {
        assert!(!logs.contains(value), "{value} in logs: {logs}");
    }
}

const REALM: &str = "tenant-north";
const USERS: &str = "/admin/realms/tenant-north/users";
const USER: &str = "/admin/realms/tenant-north/users/voter-1";
const SEARCH: &str = "SearchValue3e9a";
const EMAIL_NAME: &str = "email-value-6b2d";
const FIRST_NAME: &str = "FirstNameValue1f8c";
const LAST_NAME: &str = "LastNameValue7a4e";
const USERNAME: &str = "UsernameValue5d0b";
const ATTRIBUTE_VALUE: &str = "AttributeValue9c3f";

fn email() -> String {
    format!("{EMAIL_NAME}@example.test")
}

/// Searches for the voter with `client`, returning its outcome and the logs.
async fn search(
    client: KeycloakAdminClient,
) -> (
    anyhow::Result<(Vec<sequent_core::types::keycloak::User>, i32)>,
    String,
) {
    with_captured_logs(client.list_users(
        "north",
        "event",
        REALM,
        Some(SEARCH.to_string()),
        Some(email()),
        Some(10),
        Some(0),
    ))
    .await
}

fn assert_no_search_values(text: &str) {
    for value in [SEARCH, EMAIL_NAME] {
        assert!(!text.contains(value), "{value} in: {text}");
    }
}

#[rocket::async_test]
async fn voter_searches_and_edits_record_no_voter_data() {
    let peer = HttpServer::start(vec![
        Exchange::json("GET", USERS, 200, json!([])),
        Exchange::json("GET", &format!("{USERS}/count"), 200, json!(0)),
        Exchange::json("GET", USER, 200, json!({"id": "voter-1"})),
        Exchange::json("PUT", USER, 204, Value::Null),
    ]);

    let (listed, mut logs) = search(peer.client()).await;
    listed.expect("list users");
    let (edited, edit_logs) = with_captured_logs(peer.client().edit_user(
        REALM,
        "voter-1",
        Some(true),
        Some(HashMap::from([(
            "national-id".to_string(),
            vec![ATTRIBUTE_VALUE.to_string()],
        )])),
        Some(email()),
        Some(FIRST_NAME.to_string()),
        Some(LAST_NAME.to_string()),
        Some(USERNAME.to_string()),
        None,
        None,
    ))
    .await;
    edited.expect("edit user");
    logs.push_str(&edit_logs);

    // The values reached Keycloak.
    let requests = peer.finish();
    assert_eq!(requests[0].query()["search"], SEARCH);
    assert_eq!(requests[3].json()["firstName"], json!(FIRST_NAME));
    assert!(logs.contains("list_users"), "{logs}");
    assert!(logs.contains("edit_user_with_credentials"), "{logs}");
    for value in [
        SEARCH,
        EMAIL_NAME,
        FIRST_NAME,
        LAST_NAME,
        USERNAME,
        ATTRIBUTE_VALUE,
    ] {
        assert!(!logs.contains(value), "{value} in logs: {logs}");
    }
}

/// A request that fails before Keycloak answers names its URL, whose query
/// holds the search.
#[rocket::async_test]
async fn a_search_keycloak_cannot_be_reached_for_keeps_the_query_out() {
    // Nothing listens on port 1.
    let unreachable = KeycloakAdminClient {
        client: KeycloakAdmin::new(
            "http://127.0.0.1:1",
            serde_json::from_value(token_json()).expect("token"),
            reqwest::Client::builder()
                .no_proxy()
                .build()
                .expect("HTTP client"),
        ),
    };

    let (listed, logs) = search(unreachable).await;

    let error = listed.expect_err("nothing listens on the port");
    assert!(logs.contains("list_users"), "{logs}");
    assert_no_search_values(&logs);
    assert_no_search_values(&format!("{error:?}"));
}

#[rocket::async_test]
async fn a_search_keycloak_refuses_keeps_the_query_out() {
    let peer = HttpServer::start(vec![
        Exchange::json("GET", USERS, 200, json!([])),
        Exchange::json(
            "GET",
            &format!("{USERS}/count"),
            500,
            json!({"error": "unknown_error"}),
        ),
    ]);

    let (listed, logs) = search(peer.client()).await;

    let error = listed.expect_err("Keycloak refused the count");
    assert!(format!("{error:?}").contains("500"), "{error:?}");
    assert!(logs.contains("list_users"), "{logs}");
    assert_no_search_values(&logs);
    assert_no_search_values(&format!("{error:?}"));
    peer.finish();
}
