// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Exercise real Rocket routing, request guards and permission denials without
//! starting Harvest's service workers. Synthetic JWT payloads represent claims
//! forwarded by the identity gateway; these tests do not verify JWT signatures.

use crate::test_claims::Claims;
use rocket::http::{ContentType, Header, Status};
use rocket::local::asynchronous::Client;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use windmill::services::external::datafix_types::{
    MarkVotedBody, VoterInformationBody,
};

// Reuse Core's bounded HTTP protocol fixture, not its client implementation.
#[path = "../../../sequent-core/tests/support/http.rs"]
#[allow(dead_code)]
mod http;

#[path = "route_permissions.rs"]
mod route_permissions;

const TENANT_ID: &str = "tenant-a";
const OTHER_TENANT_ID: &str = "tenant-b";
const SUPER_ADMIN_TENANT_ID: &str = "fixture-super-admin";
const USER_ID: &str = "test-user";
// Update only with a reviewed change to the checked-in route inventory.
const EXPECTED_GUARDED_POST_ROUTE_COUNT: usize = 153;

const CHILD: &str = "HARVEST_ISOLATED_TEST_CHILD";

// A leftover environment flag must not bypass the clean child environment.
// Only the parent's private, short-lived nonce can select the child branch.
pub(crate) fn is_isolated_child() -> bool {
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
pub(crate) fn run_isolated(test: &str, keycloak_url: &str) -> String {
    run_isolated_with_postgres(test, keycloak_url, false)
}

fn run_isolated_with_postgres(
    test: &str,
    keycloak_url: &str,
    postgres: bool,
) -> String {
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
        .env("SUPER_ADMIN_TENANT_ID", SUPER_ADMIN_TENANT_ID)
        .stdin(Stdio::null())
        .stdout(output.try_clone().unwrap())
        .stderr(output);
    // Preserve instrumentation and native library lookup, never credentials.
    for name in ["LLVM_PROFILE_FILE", "LD_LIBRARY_PATH"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    if postgres {
        // Only this explicit integration fixture retains its owned test server.
        for (name, value) in std::env::vars().filter(|(name, _)| {
            name.starts_with("HASURA_DB__")
                || name.starts_with("KEYCLOAK_DB__")
                || [
                    "LOW_SQL_LIMIT",
                    "DEFAULT_SQL_LIMIT",
                    "DEFAULT_SQL_BATCH_SIZE",
                ]
                .contains(&name.as_str())
        }) {
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
    let output = std::fs::read_to_string(log.path()).unwrap();
    assert!(status.success(), "{output}");
    output
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

fn authorization(permissions: &[Permissions]) -> Header<'static> {
    authorization_for(TENANT_ID, permissions)
}

fn authorization_for(
    tenant: &str,
    permissions: &[Permissions],
) -> Header<'static> {
    bearer(&Claims::new(tenant, USER_ID).roles(permissions))
}

fn bearer(claims: &Claims) -> Header<'static> {
    Header::new("Authorization", claims.bearer())
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
                .post(route_permissions::concrete(path))
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

#[rocket::async_test]
async fn manual_verification_is_mounted_and_requires_its_permission() {
    let client = client().await;
    let body = json!({
        "tenant_id": TENANT_ID,
        "election_event_id": "manual-verification-event",
        "voter_id": "manual-verification-voter",
    });
    for permissions in [None, Some(vec![Permissions::VOTER_READ])] {
        let mut request = client
            .post("/get-manual-verification-pdf")
            .header(ContentType::JSON)
            .body(body.to_string());
        if let Some(permissions) = permissions {
            request = request.header(authorization(&permissions));
        }
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Unauthorized);
    }
}

#[rocket::async_test]
async fn cast_log_range_sort_keys_fail_before_database_access_after_voter_authorization(
) {
    const TEST: &str = "request_boundaries::cast_log_range_sort_keys_fail_before_database_access_after_voter_authorization";
    if !is_isolated_child() {
        run_isolated_with_postgres(TEST, "http://127.0.0.1:9", true);
        return;
    }
    let database_settings: Vec<_> = std::env::vars()
        .filter(|(name, _)| {
            name.starts_with("HASURA_DB__") || name.starts_with("KEYCLOAK_DB__")
        })
        .collect();
    let services = crate::route_services::Services::on_test_database().await;
    let event = crate::route_services::rows::event(&services.hasura).await;
    let election = event.election(&services.hasura).await;
    let database: String = crate::route_services::rows::query(
        &services.hasura,
        "SELECT current_database() AS name",
        &[],
    )
    .await[0]
        .get("name");
    // Before the pool is initialized, an observed local peer would catch any
    // accidental database access for unauthorized or malformed sort input.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    std::env::set_var("HASURA_DB__HOST", "127.0.0.1");
    std::env::set_var("HASURA_DB__PORT", port.to_string());
    let connections =
        std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let finished =
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let observed = connections.clone();
    let stop = finished.clone();
    let peer = std::thread::spawn(move || {
        while !stop.load(std::sync::atomic::Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    drop(stream);
                }
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock =>
                {
                    std::thread::sleep(std::time::Duration::from_millis(1))
                }
                Err(error) => panic!("cast log database peer: {error}"),
            }
        }
    });
    let client = client().await;
    let voter = || {
        Claims::new(&event.tenant_id, USER_ID)
            .username("synthetic-voter")
            .azp("voting-portal")
            .area("test-area")
            .election_event(&event.election_event_id)
            .authorized_elections(&[&election])
    };
    let body = |order: Value| {
        json!({
            "tenant_id": event.tenant_id,
            "election_event_id": event.election_event_id,
            "election_id": election,
            "ballot_id": "test-ballot",
            "order_by": order,
        })
    };
    let authorized = voter()
        .roles([sequent_core::types::permissions::VoterPermissions::CAST_VOTE]);
    // A malformed sort never reveals validation details to an unauthorized voter.
    for claims in [
        voter(),
        voter()
            .roles([
                sequent_core::types::permissions::VoterPermissions::CAST_VOTE,
            ])
            .authorized_elections(&["other-election"]),
    ] {
        let response = client
            .post("/list-cast-vote-messages")
            .header(ContentType::JSON)
            .header(bearer(&claims))
            .body(body(json!({"created_from": "asc"})).to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Unauthorized);
        assert_eq!(
            response.into_json::<Value>().await.unwrap()["extensions"]["code"],
            "Unauthorized"
        );
    }
    for field in [
        "created_from",
        "created_to",
        "statement_timestamp_from",
        "statement_timestamp_to",
    ] {
        let response = client
            .post("/list-cast-vote-messages")
            .header(ContentType::JSON)
            .header(bearer(&authorized))
            .body(body(json!({field: "asc"})).to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::BadRequest, "{field}");
        let error = response.into_json::<Value>().await.unwrap();
        assert_eq!(error["extensions"]["code"], "InvalidOrderBy");
        assert_eq!(error["message"], format!("Cannot sort by {field}"));
    }
    assert_eq!(connections.load(std::sync::atomic::Ordering::SeqCst), 0);
    finished.store(true, std::sync::atomic::Ordering::SeqCst);
    peer.join().unwrap();
    for (name, value) in database_settings {
        std::env::set_var(name, value);
    }
    std::env::set_var("HASURA_DB__DBNAME", database);
    // The real migrated event has the default hidden-log policy. Valid columns
    // reach that actual policy and are denied before the electoral-log backend.
    let mut absent = body(Value::Null);
    absent.as_object_mut().unwrap().remove("order_by");
    let controls = [
        absent,
        body(Value::Null),
        body(json!({})),
        body(json!({"created": "desc"})),
        body(json!({"statement_timestamp": "asc", "id": "desc"})),
    ];
    for input in controls {
        let response = client
            .post("/list-cast-vote-messages")
            .header(ContentType::JSON)
            .header(bearer(&authorized))
            .body(input.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Forbidden);
        let error = response.into_json::<Value>().await.unwrap();
        assert_eq!(
            error["extensions"]["code"],
            "ConfirmPolicyShowCastVoteLogsFailed"
        );
        assert!(error["message"].as_str().unwrap().contains("hide-logs-tab"));
    }
    assert_eq!(connections.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[rocket::async_test]
async fn schedule_recompute_noops_and_scope_refusals_preserve_stored_dates() {
    const TEST: &str = "request_boundaries::schedule_recompute_noops_and_scope_refusals_preserve_stored_dates";
    if !is_isolated_child() {
        run_isolated_with_postgres(TEST, "http://127.0.0.1:9", true);
        return;
    }
    use crate::route_services::{json as response_json, post, rows, Services};
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let database: String =
        rows::query(&services.hasura, "SELECT current_database() AS name", &[])
            .await[0]
            .get("name");
    // Recompute uses Windmill's global pool, unlike routes using managed state.
    // This child owns one migrated database and initializes that pool only here.
    std::env::set_var("HASURA_DB__DBNAME", database);
    let client = services.client().await;
    let body = json!({"election_event_id": event.election_event_id});
    let writer = || {
        Claims::new(&event.tenant_id, USER_ID)
            .roles([Permissions::SCHEDULED_EVENT_WRITE])
    };

    for claims in [
        Claims::new(&event.tenant_id, USER_ID),
        Claims::new(&event.tenant_id, USER_ID)
            .roles([Permissions::ELECTION_EVENT_READ]),
    ] {
        let (status, error) = response_json(
            post(&client, "/apply-schedule-recompute", &claims, &body).await,
        )
        .await;
        assert_eq!(status, Status::Unauthorized);
        assert_eq!(error["extensions"]["code"], "Unauthorized");
    }
    // No queued change means an idempotent success, even without a display name.
    // The actor still falls back to the authenticated user ID.
    assert_eq!(
        response_json(
            post(&client, "/apply-schedule-recompute", &writer(), &body).await,
        )
        .await,
        (Status::Ok, json!({"updated": 0}))
    );
    let (status, error) = response_json(
        post(
            &client,
            "/apply-schedule-recompute",
            &writer(),
            &json!({"election_event_id": "not-an-event-uuid"}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::InternalServerError);
    assert_eq!(error["extensions"]["code"], "InternalServerError");
    assert!(error["message"]
        .as_str()
        .unwrap()
        .starts_with("apply schedule recompute failed:"));
    let reader = Claims::new(&event.tenant_id, USER_ID)
        .roles([Permissions::ELECTION_EVENT_READ]);
    let (status, error) = response_json(
        post(
            &client,
            "/get-scheduled-outcomes",
            &reader,
            &json!({"election_event_id": "not-an-event-uuid"}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::BadRequest);
    assert_eq!(error["extensions"]["code"], "UuidParseFailed");
    assert_eq!(error["message"], "not-an-event-uuid is not a UUID");
    // A real schedule with a pending timezone correction must remain untouched
    // when another tenant's administrator presents the event's ID.
    let id = uuid::Uuid::new_v4();
    let tenant = uuid::Uuid::parse_str(&event.tenant_id).unwrap();
    let event_id = uuid::Uuid::parse_str(&event.election_event_id).unwrap();
    let cron = json!({"scheduled_date": "2099-01-01T10:00:00Z",
        "local": "2099-01-01T12:00", "timezone": "UTC"});
    let annotations = json!({"schedule_recompute": {
        "scheduled_date": "2099-01-01T12:00:00Z",
        "previous": "2099-01-01T10:00:00Z", "local": "2099-01-01T12:00",
        "timezone": "UTC", "checked_at": "2026-10-01T00:00:00Z"}});
    rows::execute(&services.hasura,
        "INSERT INTO sequent_backend.scheduled_event
         (id, tenant_id, election_event_id, event_processor, cron_config, annotations)
         VALUES ($1,$2,$3,'END_VOTING_PERIOD',$4,$5)",
        &[&id, &tenant, &event_id, &cron, &annotations]).await;
    let other = Claims::new(&uuid::Uuid::new_v4().to_string(), USER_ID)
        .roles([Permissions::SCHEDULED_EVENT_WRITE]);
    assert_eq!(
        response_json(
            post(&client, "/apply-schedule-recompute", &other, &body).await,
        )
        .await,
        (Status::Ok, json!({"updated": 0}))
    );
    let saved = rows::query(&services.hasura,
        "SELECT cron_config, annotations FROM sequent_backend.scheduled_event WHERE id = $1", &[&id]).await;
    assert_eq!(saved[0].get::<_, Value>("cron_config"), cron);
    assert_eq!(saved[0].get::<_, Value>("annotations"), annotations);
}
