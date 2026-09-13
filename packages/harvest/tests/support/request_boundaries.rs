// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Exercise real Rocket routing, request guards and permission denials without
//! starting Harvest's service workers. Synthetic JWT payloads represent claims
//! forwarded by the identity gateway; these tests do not verify JWT signatures.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rocket::http::{ContentType, Header, Status};
use rocket::local::asynchronous::Client;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};

const TENANT_ID: &str = "tenant-a";
const OTHER_TENANT_ID: &str = "tenant-b";
const USER_ID: &str = "test-user";
// Update only with a reviewed change to the checked-in route inventory.
const EXPECTED_GUARDED_POST_ROUTE_COUNT: usize = 115;

fn authorization(permissions: &[Permissions]) -> Header<'static> {
    let payload = json!({
        "exp": 2_000_000_000, "iat": 1_900_000_000,
        "jti": "synthetic", "iss": "https://identity.invalid", "sub": USER_ID,
        "typ": "Bearer", "azp": "admin-portal", "acr": "1", "allowed-origins": [],
        "scope": "openid", "email_verified": false,
        "https://hasura.io/jwt/claims": {
            "x-hasura-default-role": "user", "x-hasura-tenant-id": TENANT_ID,
            "x-hasura-user-id": USER_ID, "x-hasura-allowed-roles": permissions.iter().map(ToString::to_string).collect::<Vec<_>>()
        }
    });
    Header::new(
        "Authorization",
        format!(
            "Bearer fixture.{}.fixture",
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload).unwrap())
        ),
    )
}

// A successful control verifies that the fixture really passes the shared
// guard. No real privileged operation is needed merely to test its parsing.
#[get("/_boundary/claims")]
fn accepted_claims(claims: JwtClaims) -> Json<Value> {
    Json(
        json!({"tenant": claims.hasura_claims.tenant_id, "user": claims.hasura_claims.user_id, "roles": claims.hasura_claims.allowed_roles}),
    )
}

#[get("/_status/<code>")]
fn status_failure(code: u16) -> Status {
    // Rocket's Status responder returns Err(status) for 4xx/5xx codes, which
    // invokes the registered catcher. The tests assert its JSON body as well.
    Status::new(code)
}

async fn client() -> Client {
    Client::tracked(
        crate::build_application()
            .configure(rocket::Config {
                log_level: rocket::config::LogLevel::Off,
                ..Default::default()
            })
            .mount("/", routes![accepted_claims, status_failure])
            .mount("/api/datafix", routes![status_failure]),
    )
    .await
    .expect("the production routes must mount without starting workers")
}

#[rocket::async_test]
async fn valid_claims_reach_the_control_route_but_malformed_headers_do_not() {
    let client = client().await;
    let response = client
        .get("/_boundary/claims")
        .header(authorization(&[Permissions::ROLE_READ]))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(
        response.into_json::<Value>().await.unwrap(),
        json!({"tenant":TENANT_ID, "user":USER_ID, "roles":["role-read"]})
    );

    for malformed in [
        "Basic fixture",
        "bearer fixture",
        "Bearer missing-payload",
        "Bearer fixture.e30.fixture",
    ] {
        let response = client
            .get("/_boundary/claims")
            .header(Header::new("Authorization", malformed))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Unauthorized, "{malformed}");
    }
}

#[rocket::async_test]
async fn sensitive_routes_require_authorization_before_reading_the_body_or_contacting_a_backend(
) {
    let client = client().await;
    let inventory = include_str!("../fixtures/guarded-post-routes.txt");
    let paths: Vec<_> = inventory
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    assert_eq!(
        paths.len(),
        EXPECTED_GUARDED_POST_ROUTE_COUNT,
        "a truncated inventory must not weaken this check"
    );
    assert_eq!(
        paths
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        paths.len(),
        "duplicate paths must not hide a missing authorization case"
    );
    for path in paths {
        for authorization in [None, Some("Bearer fixture.e30.fixture")] {
            let mut request = client
                .post(path)
                .header(ContentType::JSON)
                .body("{malformed body");
            if let Some(value) = authorization {
                request = request.header(Header::new("Authorization", value));
            }
            let response = tokio::time::timeout(std::time::Duration::from_secs(3), request.dispatch())
                .await.unwrap_or_else(|_| panic!("{path} contacted a backend instead of rejecting the request"));
            assert_eq!(response.status(), Status::Unauthorized,
                "{path} must reject missing/malformed claims before parsing the body or calling services");
        }
    }
}

#[rocket::async_test]
async fn document_password_requires_both_download_and_password_permissions() {
    let client = client().await;
    for permissions in [
        vec![],
        vec![Permissions::DOCUMENT_DOWNLOAD],
        vec![Permissions::DOCUMENT_PASSWORD_READ],
        vec![Permissions::ROLE_WRITE],
    ] {
        let response = client
            .post("/get-document-password")
            .header(ContentType::JSON)
            .header(authorization(&permissions))
            .body(json!({"document_id":"test-document"}).to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Forbidden, "{permissions:?}");
        assert_eq!(
            response.into_json::<Value>().await.unwrap(),
            json!({
                "message":"Authorization failed", "extensions":{"code":"Unauthorized"}
            })
        );
    }
}

#[rocket::async_test]
async fn role_assignment_requires_every_permission_and_the_matching_tenant() {
    let client = client().await;
    for path in ["/set-user-role", "/delete-user-role"] {
        for (tenant, permissions) in [
            (TENANT_ID, vec![]),
            (TENANT_ID, vec![Permissions::USER_WRITE]),
            (TENANT_ID, vec![Permissions::ROLE_WRITE]),
            (
                OTHER_TENANT_ID,
                vec![Permissions::USER_WRITE, Permissions::ROLE_WRITE],
            ),
        ] {
            let response = client.post(path).header(ContentType::JSON).header(authorization(&permissions))
                .body(json!({"tenant_id":tenant,"user_id":USER_ID,"role_id":"test-role"}).to_string())
                .dispatch().await;
            assert_eq!(
                response.status(),
                Status::Unauthorized,
                "{path}: {tenant}, {permissions:?}"
            );
        }
    }
}

#[rocket::async_test]
async fn read_only_role_permission_cannot_create_or_delete_a_role() {
    let client = client().await;
    for (path, body) in [
        (
            "/create-role",
            json!({"tenant_id":TENANT_ID,"role":{"name":"new-role"}}),
        ),
        (
            "/delete-role",
            json!({"tenant_id":TENANT_ID,"role_id":"test-role"}),
        ),
    ] {
        let response = client
            .post(path)
            .header(ContentType::JSON)
            .header(authorization(&[Permissions::ROLE_READ]))
            .body(body.to_string())
            .dispatch()
            .await;
        assert_eq!(
            response.status(),
            Status::Unauthorized,
            "{path} must reject before invoking Keycloak"
        );
    }
}

#[rocket::async_test]
async fn document_and_ceremony_reads_require_their_specific_permissions() {
    let client = client().await;
    for (path, body) in [
        ("/fetch-document", json!({"document_id":"test-document"})),
        (
            "/get-private-key",
            json!({"election_event_id":"test-event","keys_ceremony_id":"test-ceremony"}),
        ),
        (
            "/check-private-key",
            json!({"election_event_id":"test-event","keys_ceremony_id":"test-ceremony","private_key_base64":"not-a-key"}),
        ),
        (
            "/create-election",
            json!({"election_event_id":"test-event","external_id":"test-election","presentation":{}}),
        ),
    ] {
        let response = client
            .post(path)
            .header(ContentType::JSON)
            .header(authorization(&[Permissions::ROLE_READ]))
            .body(body.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Unauthorized, "{path}");
    }
}

#[rocket::async_test]
async fn datafix_missing_credentials_use_the_documented_json_error_on_every_operation(
) {
    let client = client().await;
    for path in [
        "add-voter",
        "update-voter",
        "delete-voter",
        "unmark-voted",
        "mark-voted",
        "replace-pin",
    ] {
        let response = client
            .post(format!("/api/datafix/{path}"))
            .header(ContentType::JSON)
            .body("{}")
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::BadRequest, "{path}");
        assert_eq!(
            response.into_json::<Value>().await.unwrap(),
            json!({"code":400,"message":"Bad Request","error_code":"invalid-request"})
        );
    }
}

#[rocket::async_test]
async fn scoped_datafix_catchers_translate_framework_failures_without_changing_other_routes(
) {
    let client = client().await;
    for (original, expected, reason, code) in [
        (400, Status::BadRequest, "Bad Request", "invalid-request"),
        (422, Status::BadRequest, "Bad Request", "invalid-request"),
        (401, Status::Forbidden, "Forbidden", "forbidden"),
    ] {
        let response = client
            .get(format!("/api/datafix/_status/{original}"))
            .dispatch()
            .await;
        assert_eq!(response.status(), expected);
        assert_eq!(
            response.into_json::<Value>().await.unwrap(),
            json!({"code":expected.code,"message":reason,"error_code":code})
        );
    }
    let ordinary = client.get("/_status/401").dispatch().await;
    assert_eq!(ordinary.status(), Status::Unauthorized);
    assert_eq!(
        ordinary.into_json::<Value>().await.unwrap(),
        json!({"message":"Unknown Error"})
    );
}

#[rocket::async_test]
async fn public_catchers_keep_internal_details_out_of_error_bodies() {
    let client = client().await;
    for (path, status, message) in [
        (
            "/missing-with-private-looking-input",
            Status::NotFound,
            "Not found",
        ),
        (
            "/_status/500",
            Status::InternalServerError,
            "Internal error",
        ),
    ] {
        let response = client.get(path).dispatch().await;
        assert_eq!(response.status(), status);
        assert_eq!(
            response.into_json::<Value>().await.unwrap(),
            json!({"message":message})
        );
    }
}
