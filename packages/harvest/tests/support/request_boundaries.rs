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
use windmill::services::datafix::types::{MarkVotedBody, VoterInformationBody};

// Reuse Core's bounded HTTP protocol fixture, not its client implementation.
#[path = "../../../sequent-core/tests/support/http.rs"]
#[allow(dead_code)]
mod http;

const TENANT_ID: &str = "tenant-a";
const OTHER_TENANT_ID: &str = "tenant-b";
const USER_ID: &str = "test-user";
// Update only with a reviewed change to the checked-in route inventory.
const EXPECTED_GUARDED_POST_ROUTE_COUNT: usize = 117;

const CHILD: &str = "HARVEST_ISOLATED_TEST_CHILD";

// A leftover environment flag must not bypass the clean child environment.
// Only the parent's private, short-lived nonce can select the child branch.
fn is_isolated_child() -> bool {
    std::env::var(CHILD)
        .ok()
        .and_then(|value| {
            serde_json::from_str::<(std::path::PathBuf, String)>(&value).ok()
        })
        .is_some_and(|(path, nonce)| {
            std::fs::read_to_string(&path).ok().as_deref()
                == Some(nonce.as_str())
        })
}

// Keycloak's token cache and environment are process-global. A fresh child
// isolates them from all other tests and from developer settings; its only
// identity provider is the local peer and it has no database settings.
fn run_isolated(test: &str, keycloak_url: &str) {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    let marker = tempfile::NamedTempFile::new().unwrap();
    let nonce = uuid::Uuid::new_v4().to_string();
    std::fs::write(marker.path(), &nonce).unwrap();
    let log = tempfile::NamedTempFile::new().unwrap();
    let output = log.reopen().unwrap();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", test, "--nocapture"])
        .env_clear()
        .env(CHILD, json!([marker.path(), nonce]).to_string())
        .env("KEYCLOAK_URL", keycloak_url)
        .env("KEYCLOAK_ADMIN_CLIENT_ID", "synthetic-admin")
        .env("KEYCLOAK_ADMIN_CLIENT_SECRET", "synthetic-secret")
        .env("SUPER_ADMIN_TENANT_ID", "fixture-super-admin")
        .stdin(Stdio::null())
        .stdout(output.try_clone().unwrap())
        .stderr(output);
    // Preserve instrumentation and native library lookup, never credentials.
    for name in ["LLVM_PROFILE_FILE", "LD_LIBRARY_PATH"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    let mut child = command.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!(
                "{test} timed out: {}",
                std::fs::read_to_string(log.path()).unwrap()
            );
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    assert!(
        status.success(),
        "{}",
        std::fs::read_to_string(log.path()).unwrap()
    );
}

#[rocket::async_test]
async fn role_creation_requires_create_permission_and_preserves_the_role() {
    if !is_isolated_child() {
        // KeycloakAdminClient uses KeycloakAdminToken::acquire: the pinned
        // client's admin-password flow authenticates in master. The separate
        // get_credentials_inner tenant-client flow is not used by this route.
        let peer = http::HttpServer::start(vec![
            http::Exchange::json(
                "POST",
                "/realms/master/protocol/openid-connect/token",
                200,
                http::token_json(),
            ),
            http::Exchange::json(
                "POST",
                "/admin/realms/tenant-tenant-a/groups",
                201,
                json!({}),
            ),
            http::Exchange::json(
                "GET",
                "/admin/realms/tenant-tenant-a/groups",
                200,
                json!([{"id":"new-role", "name":"Election observer"}]),
            ),
        ]);
        run_isolated(
            "request_boundaries::role_creation_requires_create_permission_and_preserves_the_role",
            &peer.url,
        );
        let requests = peer.finish();
        let created = requests
            .iter()
            .find(|r| r.method == "POST" && r.url.path().ends_with("/groups"))
            .unwrap();
        assert_eq!(created.json()["name"], "Election observer");
        assert_eq!(
            created.headers["authorization"],
            "Bearer synthetic-access-token"
        );
        return;
    }
    let client = client().await;
    let body =
        json!({"tenant_id": TENANT_ID, "role": {"name": "Election observer"}});
    let response = client
        .post("/create-role")
        .header(ContentType::JSON)
        .header(authorization(&[Permissions::ROLE_CREATE]))
        .body(body.to_string())
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(
        response.into_json::<Value>().await.unwrap()["name"],
        "Election observer"
    );
    for permissions in [
        vec![],
        vec![Permissions::ROLE_READ],
        vec![Permissions::ROLE_WRITE],
    ] {
        let response = client
            .post("/create-role")
            .header(ContentType::JSON)
            .header(authorization(&permissions))
            .body(body.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Unauthorized, "{permissions:?}");
    }
}

#[rocket::async_test]
async fn complete_permission_sets_pass_each_authorization_check() {
    if !is_isolated_child() {
        // Nothing is scripted, so every Keycloak call gets HTTP 500.
        let peer = http::HttpServer::start(vec![]);
        run_isolated(
            "request_boundaries::complete_permission_sets_pass_each_authorization_check",
            &peer.url,
        );
        peer.finish();
        return;
    }
    // The denial tests below remove one of these permissions or change the
    // tenant. Here the complete set must get past authorization and stop at
    // the unavailable backend, so a route that required a different
    // permission would answer 401 or 403 instead.
    let client = client().await;
    let user_role =
        json!({"tenant_id":TENANT_ID,"user_id":USER_ID,"role_id":"test-role"});
    let document = json!({"document_id":"test-document"});
    let ceremony = json!({"election_event_id":"test-event","keys_ceremony_id":"test-ceremony"});
    let mut key_check = ceremony.clone();
    key_check["private_key_base64"] = json!("not-a-key");
    for (path, body, permissions) in [
        (
            "/get-document-password",
            &document,
            vec![
                Permissions::DOCUMENT_DOWNLOAD,
                Permissions::DOCUMENT_PASSWORD_READ,
            ],
        ),
        (
            "/set-user-role",
            &user_role,
            vec![Permissions::USER_WRITE, Permissions::ROLE_WRITE],
        ),
        (
            "/delete-user-role",
            &user_role,
            vec![Permissions::USER_WRITE, Permissions::ROLE_WRITE],
        ),
        (
            "/delete-role",
            &json!({"tenant_id":TENANT_ID,"role_id":"test-role"}),
            vec![Permissions::ROLE_WRITE],
        ),
        (
            "/fetch-document",
            &document,
            vec![Permissions::DOCUMENT_DOWNLOAD],
        ),
        (
            "/get-private-key",
            &ceremony,
            vec![Permissions::TRUSTEE_CEREMONY],
        ),
        (
            "/check-private-key",
            &key_check,
            vec![Permissions::TRUSTEE_CEREMONY],
        ),
        (
            "/create-election",
            &json!({"election_event_id":"test-event","external_id":"test-election","presentation":{}}),
            vec![Permissions::ELECTION_EVENT_WRITE],
        ),
    ] {
        let response = client
            .post(path)
            .header(ContentType::JSON)
            .header(authorization(&permissions))
            .body(body.to_string())
            .dispatch()
            .await;
        assert_eq!(
            response.status(),
            Status::InternalServerError,
            "{path}: {permissions:?}"
        );
    }
}

fn authorization(permissions: &[Permissions]) -> Header<'static> {
    authorization_for(TENANT_ID, permissions)
}

fn authorization_for(
    tenant: &str,
    permissions: &[Permissions],
) -> Header<'static> {
    let payload = json!({
        "exp": 2_000_000_000, "iat": 1_900_000_000,
        "jti": "synthetic", "iss": "https://identity.invalid", "sub": USER_ID,
        "typ": "Bearer", "azp": "admin-portal", "acr": "1", "allowed-origins": [],
        "scope": "openid", "email_verified": false,
        "https://hasura.io/jwt/claims": {
            "x-hasura-default-role": "user", "x-hasura-tenant-id": tenant,
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
    let registered: std::collections::BTreeSet<_> = client
        .rocket()
        .routes()
        .filter(|route| route.method == rocket::http::Method::Post)
        .map(|route| route.uri.path().to_string())
        .collect();
    let mut expected: std::collections::BTreeSet<_> =
        paths.iter().map(|path| path.to_string()).collect();
    // Datafix uses its own authentication and catcher contract, rather than
    // the Hasura JWT guard exercised by this inventory.
    expected.extend(
        [
            "/api/datafix/add-voter",
            "/api/datafix/delete-voter",
            "/api/datafix/mark-voted",
            "/api/datafix/replace-pin",
            "/api/datafix/unmark-voted",
            "/api/datafix/update-voter",
        ]
        .into_iter()
        .map(str::to_owned),
    );
    assert_eq!(
        registered, expected,
        "update authorization cases when production POST routes change"
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
    // Choose an ordinary tenant even when the caller's environment names our
    // usual fixture tenant as super-admin. Never mutate process-global settings.
    let tenant_id =
        if std::env::var("SUPER_ADMIN_TENANT_ID").as_deref() == Ok(TENANT_ID) {
            OTHER_TENANT_ID
        } else {
            TENANT_ID
        };
    let client = client().await;
    for path in ["/set-user-role", "/delete-user-role"] {
        for (tenant, permissions) in [
            (tenant_id, vec![]),
            (tenant_id, vec![Permissions::USER_WRITE]),
            (tenant_id, vec![Permissions::ROLE_WRITE]),
            (
                "fixture-other-request-tenant",
                vec![Permissions::USER_WRITE, Permissions::ROLE_WRITE],
            ),
        ] {
            let response = client.post(path).header(ContentType::JSON).header(authorization_for(tenant_id, &permissions))
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
    const VOTER_ID: &str = "synthetic-voter";
    let voter = json!({"voter_id": VOTER_ID, "ward": "synthetic-ward"});
    let voted = json!({"voter_id": VOTER_ID, "channel": "online"});
    let voter_id = json!({ "voter_id": VOTER_ID });
    // Each body is valid, so a malformed-body 422 (mapped to the same 400)
    // cannot stand in for the missing-credentials response.
    serde_json::from_value::<VoterInformationBody>(voter.clone()).unwrap();
    serde_json::from_value::<MarkVotedBody>(voted.clone()).unwrap();
    serde_json::from_value::<crate::routes::api_datafix::VoterIdBody>(
        voter_id.clone(),
    )
    .unwrap();
    for (path, body) in [
        ("add-voter", &voter),
        ("update-voter", &voter),
        ("delete-voter", &voter_id),
        ("unmark-voted", &voter_id),
        ("mark-voted", &voted),
        ("replace-pin", &voter_id),
    ] {
        let response = client
            .post(format!("/api/datafix/{path}"))
            .header(ContentType::JSON)
            .body(body.to_string())
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
