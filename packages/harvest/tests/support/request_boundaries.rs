// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Exercise real Rocket routing, request guards and permission denials without
//! starting Harvest's service workers. Synthetic JWTs are signed with a public
//! test key and verified by the real guards against a local key server.

use crate::test_claims::Claims;
use rocket::http::{ContentType, Header, Status};
use rocket::local::asynchronous::Client;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::signing::SigningAction;
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

const TENANT_ID: &str = "7d6f1c2a-5b0e-4e0a-9a43-2f6c1b8d9e10";
const OTHER_TENANT_ID: &str = "c1a9d4f7-3e58-4b62-8f0d-6a2e7b9c4d31";
const UUID_TENANT_ID: &str = "00000000-0000-0000-0000-00000000000a";
const SUPER_ADMIN_TENANT_ID: &str = "fixture-super-admin";
const USER_ID: &str = "test-user";
// Update only with a reviewed change to the checked-in route inventory.
const EXPECTED_GUARDED_POST_ROUTE_COUNT: usize = 216;

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

pub(crate) fn run_isolated_with_postgres(
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
    // The electoral-log proof routes need a server to mount. None listens
    // here, so without the integration fixture nothing reaches a database.
    command
        .env("ELECTORAL_LOG_PG_HOST", "127.0.0.1")
        .env("ELECTORAL_LOG_PG_PORT", "1")
        .env("ELECTORAL_LOG_PG_USER", "synthetic")
        .env("ELECTORAL_LOG_PG_PASSWORD", "synthetic")
        .env("ELECTORAL_LOG_PG_DATABASE", "synthetic")
        .env("ELECTORAL_LOG_PG_SSLMODE", "disable");
    if postgres {
        // Only this explicit integration fixture retains its owned test server.
        for (name, value) in std::env::vars().filter(|(name, _)| {
            name.starts_with("HASURA_DB__")
                || name.starts_with("KEYCLOAK_DB__")
                || name.starts_with("ELECTORAL_LOG_PG_")
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
                &format!("/admin/realms/tenant-{TENANT_ID}/groups"),
                201,
                json!({}),
            ),
            http::Exchange::json(
                "GET",
                &format!("/admin/realms/tenant-{TENANT_ID}/groups"),
                200,
                json!([{"id":"new-role", "name":"Election observer"}]),
            ),
        ]);
        run_isolated_with_postgres(
            "request_boundaries::role_creation_requires_create_permission_and_preserves_the_role",
            &peer.url,
            true,
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
    // Creating a role queues its audit entry, through the pool this child
    // initializes on its own migrated database.
    let services = crate::route_services::Services::on_test_database().await;
    let database: String = crate::route_services::rows::query(
        &services.hasura,
        "SELECT current_database() AS name",
        &[],
    )
    .await[0]
        .get("name");
    std::env::set_var("HASURA_DB__DBNAME", database);
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
// guard. No real privileged operation is needed merely to test the guard.
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
    // Both guards verify the signed user fixtures before the service guard
    // rejects them for not belonging to the service client.
    let service_keys = std::sync::Arc::new(
        crate::adapters::memory::service_keys::StaticServiceKeys::default(),
    );
    service_keys.publish(
        &crate::route_services::service_tokens::realm(
            route_permissions::UUID_TENANT_ID,
        ),
        crate::services::service_auth::parse_key_set(
            &crate::test_claims::signing::key_set(),
        ),
    );
    // The production services as a deployment that leaves
    // MESSAGING_FAKE_PROVIDER_ROUTES unset has them, whatever the developer's
    // environment says: the fake provider routes have their own case.
    let services = crate::services::dependencies::HarvestServices {
        service_keys: std::sync::Arc::new(
            crate::services::service_auth::ServiceKeys::new(service_keys),
        ),
        messaging_fake_provider_routes:
            crate::routes::fake_provider::FakeProviderRoutes::DISABLED,
        ..crate::services::dependencies::HarvestServices::production()
    };
    Client::tracked(
        crate::build_application_with(services)
            .manage(crate::test_claims::signing::issuers())
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
    // No provider webhook route: providers reach harvest only through the
    // webhook component, which relays to /messaging/callbacks (inventoried).
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

/// `/messaging/callbacks` takes only envelopes the webhook component signed
/// with the relay secret, recently. Everything else is refused before the
/// envelope is read or a backend is contacted.
#[rocket::async_test]
async fn relayed_callbacks_need_a_fresh_relay_signature() {
    use crate::route_services::{Services, RELAY_SECRET};
    use messaging::relay::{
        self, CallbackEnvelope, CallbackMethod, CallbackProvider,
        RelayResponse, RELAY_SIGNATURE_HEADER,
    };
    let client = Services::without_database().client().await;
    assert!(
        client.rocket().routes().any(|route| {
            route.method == rocket::http::Method::Post
                && route.uri.path() == relay::CALLBACKS_PATH
        }),
        "the component posts to {}",
        relay::CALLBACKS_PATH
    );
    let key =
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let envelope = |key: &str| {
        serde_json::to_vec(&CallbackEnvelope::new(
            CallbackProvider::META,
            key,
            CallbackMethod::POST,
            "",
            [("x-hub-signature-256", "sha256=00")],
            b"{}",
            chrono::Utc::now(),
        ))
        .unwrap()
    };
    let payload = envelope(key);
    let now = chrono::Utc::now().timestamp();
    let sign = |secret: &[u8], at: i64, body: &[u8]| {
        Some(relay::sign(secret, at, body))
    };
    let mut tampered = payload.clone();
    tampered.extend_from_slice(b" ");
    let mut newer: serde_json::Value =
        serde_json::from_slice(&payload).unwrap();
    newer["version"] = json!(2);
    let newer = serde_json::to_vec(&newer).unwrap();

    for (signature, body, status) in [
        (None, payload.clone(), Status::Unauthorized),
        (
            Some("t=1,v1=zz".to_string()),
            payload.clone(),
            Status::Unauthorized,
        ),
        (
            Some(format!("v1={}", "00".repeat(32))),
            payload.clone(),
            Status::Unauthorized,
        ),
        (
            sign(RELAY_SECRET, now - 301, &payload),
            payload.clone(),
            Status::Unauthorized,
        ),
        (
            sign(RELAY_SECRET, now + 301, &payload),
            payload.clone(),
            Status::Unauthorized,
        ),
        (
            sign(b"another secret", now, &payload),
            payload.clone(),
            Status::Unauthorized,
        ),
        (
            sign(RELAY_SECRET, now, &payload),
            tampered,
            Status::Unauthorized,
        ),
        (
            sign(RELAY_SECRET, now, b"not json"),
            b"not json".to_vec(),
            Status::BadRequest,
        ),
        (sign(RELAY_SECRET, now, &newer), newer, Status::BadRequest),
        // Authenticated and well formed: the account lookup needs the
        // database this client has none of.
        (
            sign(RELAY_SECRET, now, &payload),
            payload.clone(),
            Status::ServiceUnavailable,
        ),
    ] {
        let mut request = client
            .post(relay::CALLBACKS_PATH)
            .header(ContentType::JSON)
            .body(body);
        if let Some(signature) = &signature {
            request = request
                .header(Header::new(RELAY_SIGNATURE_HEADER, signature.clone()));
        }
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            request.dispatch(),
        )
        .await
        .expect("answered without waiting for a backend");
        assert_eq!(response.status(), status, "{signature:?}");
    }

    // A key of the wrong shape is never looked up.
    let unknown = envelope("../../etc/passwd");
    let response = client
        .post(relay::CALLBACKS_PATH)
        .header(ContentType::JSON)
        .header(Header::new(
            RELAY_SIGNATURE_HEADER,
            relay::sign(RELAY_SECRET, now, &unknown),
        ))
        .body(unknown)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(
        response.into_json::<RelayResponse>().await,
        Some(RelayResponse::text(404, ""))
    );

    // An envelope over the limit is refused unread.
    let limit = relay::envelope_limit(relay::DEFAULT_MAX_BODY_BYTES);
    let response = client
        .post(relay::CALLBACKS_PATH)
        .header(ContentType::JSON)
        .header(Header::new(
            RELAY_SIGNATURE_HEADER,
            relay::sign(RELAY_SECRET, now, b""),
        ))
        .body(vec![b' '; usize::try_from(limit).unwrap() + 1])
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::PayloadTooLarge);

    // Regression (R1-09): a captured relayed request could be sent again,
    // unchanged, while its signature was fresh. It is taken once.
    let post_signed = |signature: String, body: Vec<u8>| {
        let client = &client;
        async move {
            client
                .post(relay::CALLBACKS_PATH)
                .header(ContentType::JSON)
                .header(Header::new(RELAY_SIGNATURE_HEADER, signature))
                .body(body)
                .dispatch()
                .await
                .status()
        }
    };
    let once = envelope(key);
    let signature = relay::sign(RELAY_SECRET, now, &once);
    assert_eq!(
        post_signed(signature.clone(), once.clone()).await,
        Status::ServiceUnavailable
    );
    assert_eq!(post_signed(signature, once).await, Status::Unauthorized);
    // The envelope's time is the signature's: an old capture can't be
    // signed again as new.
    let mut stale: serde_json::Value =
        serde_json::from_slice(&envelope(key)).unwrap();
    stale["received_at"] =
        json!(chrono::DateTime::from_timestamp(now - 301, 0)
            .unwrap()
            .to_rfc3339());
    let stale = serde_json::to_vec(&stale).unwrap();
    assert_eq!(
        post_signed(relay::sign(RELAY_SECRET, now, &stale), stale).await,
        Status::Unauthorized
    );
}

/// Without a usable relay secret, harvest refuses every relayed callback
/// with 503 (the component has the provider retry), signed or not.
#[rocket::async_test]
async fn without_a_relay_secret_relayed_callbacks_are_refused_with_503() {
    use crate::route_services::{Services, RELAY_SECRET};
    use messaging::relay::{self, RELAY_SIGNATURE_HEADER};
    let client = Services::without_database()
        .with_messaging_relay(crate::routes::messaging::MessagingRelay::new(
            None,
            relay::DEFAULT_MAX_BODY_BYTES,
        ))
        .client()
        .await;
    let now = chrono::Utc::now().timestamp();
    let response = client
        .post(relay::CALLBACKS_PATH)
        .header(ContentType::JSON)
        .header(Header::new(
            RELAY_SIGNATURE_HEADER,
            relay::sign(RELAY_SECRET, now, b"{}"),
        ))
        .body("{}")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::ServiceUnavailable);
}

/// `MESSAGING_FAKE_PROVIDER_ROUTES=ENABLED` adds exactly the fake
/// provider's routes under `/fake-provider`, and nothing else; without it
/// they don't exist. The viewer's two routes (the Hasura actions) refuse a
/// request without valid claims before reading the body or reaching a
/// backend. The provider routes take what a provider would.
#[rocket::async_test]
async fn fake_provider_routes_are_mounted_only_by_their_switch() {
    use crate::route_services::Services;
    use std::collections::BTreeSet;
    fn routes(client: &Client) -> BTreeSet<(String, String)> {
        client
            .rocket()
            .routes()
            .map(|route| {
                (route.method.to_string(), route.uri.path().to_string())
            })
            .collect()
    }
    let off = Services::without_database().client().await;
    let on = Services::without_database()
        .with_fake_provider_routes()
        .client()
        .await;
    let (off_routes, on_routes) = (routes(&off), routes(&on));
    assert!(
        off_routes
            .iter()
            .all(|(_, path)| !path.starts_with("/fake-provider")),
        "{off_routes:?}"
    );
    let added: BTreeSet<(String, String)> =
        on_routes.difference(&off_routes).cloned().collect();
    let expected: BTreeSet<(String, String)> = [
        (
            "POST",
            "/fake-provider/meta/<_version>/<sender_id>/messages",
        ),
        ("GET", "/fake-provider/meta/<path..>"),
        ("POST", "/fake-provider/meta/messenger/get-started"),
        ("POST", "/fake-provider/infobip/viber/2/messages"),
        ("GET", "/fake-provider/infobip/account/1/balance"),
        ("GET", "/fake-provider/infobip/viber/2/logs"),
        ("POST", "/fake-provider/aws/<path..>"),
        ("GET", "/fake-provider/aws/<path..>"),
        ("POST", "/fake-provider/messages"),
        ("POST", "/fake-provider/message"),
    ]
    .into_iter()
    .map(|(method, path)| (method.to_string(), path.to_string()))
    .collect();
    assert_eq!(added, expected);
    assert!(off_routes.is_subset(&on_routes), "nothing else changes");

    let admin = authorization(&[Permissions::MESSAGING_ACCOUNT_WRITE]);
    for path in [
        "/fake-provider/messages",
        "/fake-provider/message",
        "/fake-provider/meta/messenger/get-started",
    ] {
        for authorization in [None, Some("Bearer fixture.e30.fixture")] {
            let mut request = on
                .post(path)
                .header(ContentType::JSON)
                .body("{malformed body");
            if let Some(value) = authorization {
                request = request.header(Header::new("Authorization", value));
            }
            let response = tokio::time::timeout(
                std::time::Duration::from_secs(3),
                request.dispatch(),
            )
            .await
            .unwrap_or_else(|_| panic!("{path} contacted a backend"));
            assert_eq!(response.status(), Status::Unauthorized, "{path}");
        }
        let response = off
            .post(path)
            .header(ContentType::JSON)
            .header(admin.clone())
            .body("{}")
            .dispatch()
            .await;
        assert_eq!(
            response.status(),
            Status::NotFound,
            "{path} without the switch"
        );
    }

    // A send without an account's credential is refused as the provider
    // refuses it, before any backend: nothing is kept or reported.
    for (path, content_type, body, refused) in [
        (
            "/fake-provider/meta/v23.0/123456/messages",
            ContentType::JSON,
            json!({"messaging_product": "whatsapp", "to": "+34600000001",
                   "type": "text", "text": {"body": "Your code is 111111"}})
            .to_string(),
            Status::Unauthorized,
        ),
        (
            "/fake-provider/infobip/viber/2/messages",
            ContentType::JSON,
            json!({"messages": [{"sender": "Elections",
                   "destinations": [{"to": "+34600000001", "messageId": "m-1"}],
                   "content": {"text": "Your code is 111111"}}]})
            .to_string(),
            Status::Unauthorized,
        ),
        (
            "/fake-provider/aws/",
            ContentType::Form,
            "Action=Publish&PhoneNumber=%2B34600000001&Message=x".to_string(),
            Status::Forbidden,
        ),
    ] {
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            on.post(path).header(content_type).body(body).dispatch(),
        )
        .await
        .unwrap_or_else(|_| panic!("{path} contacted a backend"));
        assert_eq!(response.status(), refused, "{path}");
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
    let (tenant_id, other_tenant_id) =
        if std::env::var("SUPER_ADMIN_TENANT_ID").as_deref() == Ok(TENANT_ID) {
            (OTHER_TENANT_ID, TENANT_ID)
        } else {
            (TENANT_ID, OTHER_TENANT_ID)
        };
    let client = client().await;
    for path in ["/set-user-role", "/delete-user-role"] {
        for (tenant, permissions) in [
            (tenant_id, vec![]),
            (tenant_id, vec![Permissions::USER_WRITE]),
            (tenant_id, vec![Permissions::ROLE_WRITE]),
            (
                other_tenant_id,
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
async fn role_permission_changes_reject_role_ids_that_are_not_one_path_segment()
{
    let sign_permission =
        SigningAction::OpenVoting.sign_permission().to_string();
    let client = client().await;
    for path in ["/set-role-permission", "/delete-role-permission"] {
        for permission_name in ["test-permission", sign_permission.as_str()] {
            let response = client
                .post(path)
                .header(ContentType::JSON)
                .header(authorization_for(
                    UUID_TENANT_ID,
                    &[
                        Permissions::USER_PERMISSION_WRITE,
                        Permissions::ROLE_WRITE,
                    ],
                ))
                .body(
                    json!({
                        "tenant_id": UUID_TENANT_ID,
                        "role_id": "../../other-realm",
                        "permission_name": permission_name,
                    })
                    .to_string(),
                )
                .dispatch()
                .await;
            assert_eq!(
                response.status(),
                Status::BadRequest,
                "{path}: {permission_name} must reject before any backend call"
            );
        }
    }
}

#[rocket::async_test]
async fn realm_routes_reject_event_ids_that_are_not_uuids() {
    let password_policy = json!({
        "minimum_length": 8,
        "maximum_length": 64,
        "include_uppercase": true,
        "include_lowercase": true,
        "include_digits": true,
        "include_special_characters": false,
    });
    let client = client().await;
    for (path, permission, mut body) in [
        (
            "/get-realm-attributes",
            Permissions::KEYCLOAK_REALM_ATTRIBUTES_READ,
            json!({}),
        ),
        (
            "/update-realm-attributes",
            Permissions::KEYCLOAK_REALM_ATTRIBUTES_WRITE,
            json!({"attributes": {}}),
        ),
        (
            "/get-realm-password-policy",
            Permissions::ELECTION_EVENT_READ,
            json!({}),
        ),
        (
            "/update-realm-password-policy",
            Permissions::ELECTION_EVENT_WRITE,
            password_policy,
        ),
    ] {
        body["election_event_id"] = json!("x/../tenant-other-event-other");
        let response = client
            .post(path)
            .header(ContentType::JSON)
            .header(authorization_for(UUID_TENANT_ID, &[permission]))
            .body(body.to_string())
            .dispatch()
            .await;
        assert_eq!(
            response.status(),
            Status::BadRequest,
            "{path} must reject before any backend call"
        );
    }
}

#[rocket::async_test]
async fn role_deletion_rejects_role_ids_that_are_not_one_path_segment() {
    let client = client().await;
    let response = client
        .post("/delete-role")
        .header(ContentType::JSON)
        .header(authorization_for(
            UUID_TENANT_ID,
            &[Permissions::ROLE_WRITE],
        ))
        .body(
            json!({
                "tenant_id": UUID_TENANT_ID,
                "role_id": "../../other-realm",
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(
        response.status(),
        Status::BadRequest,
        "/delete-role must reject before any backend call"
    );
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
            "/get-custody-request",
            json!({"election_event_id":"test-event","keys_ceremony_id":"test-ceremony"}),
        ),
        (
            "/confirm-key-custody",
            json!({"election_event_id":"test-event","keys_ceremony_id":"test-ceremony","custody_proof_json":"not-a-key"}),
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

#[rocket::async_test]
async fn deleting_an_event_with_ballot_box_seals_is_refused_before_its_task() {
    const TEST: &str = "request_boundaries::deleting_an_event_with_ballot_box_seals_is_refused_before_its_task";
    if !is_isolated_child() {
        run_isolated_with_postgres(TEST, "http://127.0.0.1:9", true);
        return;
    }
    use crate::route_services::{
        json as response_json, post, rows, text, Services,
    };
    use windmill::postgres::ballot_box_seal::{
        insert_pending, ClosedBy, NewPendingSeal,
    };
    use windmill::postgres::election_event::SEALED_EVENT_DELETE_REFUSAL;
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let database: String =
        rows::query(&services.hasura, "SELECT current_database() AS name", &[])
            .await[0]
            .get("name");
    // The delete route records its task and reads the seals through
    // Windmill's global pool. This child initializes it on its own database.
    std::env::set_var("HASURA_DB__DBNAME", database);
    let election = event.election(&services.hasura).await;
    let area = event.area(&services.hasura, "Sealed box").await;
    let tenant = uuid::Uuid::parse_str(&event.tenant_id).unwrap();
    let event_id = uuid::Uuid::parse_str(&event.election_event_id).unwrap();
    {
        let mut client = services.hasura.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let closed_at = chrono::Utc::now();
        let seal = NewPendingSeal {
            tenant_id: tenant,
            election_event_id: event_id,
            election_id: uuid::Uuid::parse_str(&election).unwrap(),
            area_id: uuid::Uuid::parse_str(&area).unwrap(),
            closed_at,
            grace_deadline: closed_at,
            close_request_id: None,
            closed_by: ClosedBy::Scheduled,
        };
        assert_eq!(insert_pending(&tx, &[seal]).await.unwrap(), 1);
        tx.commit().await.unwrap();
    }
    let client = services.client().await;
    let deleter = Claims::new(&event.tenant_id, USER_ID)
        .roles([Permissions::ELECTION_EVENT_DELETE]);

    // Seals are permanent: the route answers the refusal and records it as
    // the task's failure, before any delete task is queued.
    let (status, output) = response_json(
        post(
            &client,
            "/delete-election-event",
            &deleter,
            &json!({"election_event_id": event.election_event_id}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{output}");
    assert_eq!(output["id"], json!(event.election_event_id));
    assert_eq!(output["error_msg"], json!(SEALED_EVENT_DELETE_REFUSAL));
    let task =
        uuid::Uuid::parse_str(output["task_execution"]["id"].as_str().unwrap())
            .unwrap();
    let stored = rows::query(
        &services.hasura,
        "SELECT execution_status FROM sequent_backend.tasks_execution WHERE id = $1",
        &[&task],
    )
    .await;
    assert_eq!(stored[0].get::<_, String>("execution_status"), "FAILED");
    let events = rows::query(
        &services.hasura,
        "SELECT count(*) AS n FROM sequent_backend.election_event WHERE id = $1",
        &[&event_id],
    )
    .await;
    assert_eq!(events[0].get::<_, i64>("n"), 1);

    // A seal check that cannot run is a server error, not a delete.
    let (status, error) = text(
        post(
            &client,
            "/delete-election-event",
            &deleter,
            &json!({"election_event_id": "not-an-event-uuid"}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::InternalServerError);
    assert!(
        error.starts_with("Failed to check the ballot box seals"),
        "{error}"
    );
}

#[rocket::async_test]
async fn deleting_an_event_that_must_be_kept_is_refused_before_its_task() {
    const TEST: &str = "request_boundaries::deleting_an_event_that_must_be_kept_is_refused_before_its_task";
    if !is_isolated_child() {
        run_isolated_with_postgres(TEST, "http://127.0.0.1:9", true);
        return;
    }
    use crate::route_services::{
        json as response_json, post, rows, text, Services,
    };
    let services = Services::on_test_database().await;
    let database: String =
        rows::query(&services.hasura, "SELECT current_database() AS name", &[])
            .await[0]
            .get("name");
    std::env::set_var("HASURA_DB__DBNAME", database);
    let client = services.client().await;

    let open = rows::event(&services.hasura).await;
    let open_id = uuid::Uuid::parse_str(&open.election_event_id).unwrap();
    rows::execute(
        &services.hasura,
        "UPDATE sequent_backend.election_event SET status = $1
         WHERE id = $2
           AND set_config('sequent.trusted_write', 'on', true) = 'on'",
        &[&json!({"voting_status": "OPEN"}), &open_id],
    )
    .await;
    let deleter = Claims::new(&open.tenant_id, USER_ID)
        .roles([Permissions::ELECTION_EVENT_DELETE]);
    let (status, output) = response_json(
        post(
            &client,
            "/delete-election-event",
            &deleter,
            &json!({"election_event_id": open.election_event_id}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{output}");
    assert_eq!(output["id"], json!(open.election_event_id));
    let message = output["error_msg"].as_str().unwrap();
    assert!(message.contains("Voting is open or paused"), "{message}");
    let task =
        uuid::Uuid::parse_str(output["task_execution"]["id"].as_str().unwrap())
            .unwrap();
    let stored = rows::query(
        &services.hasura,
        "SELECT execution_status FROM sequent_backend.tasks_execution WHERE id = $1",
        &[&task],
    )
    .await;
    assert_eq!(stored[0].get::<_, String>("execution_status"), "FAILED");
    let events = rows::query(
        &services.hasura,
        "SELECT count(*) AS n FROM sequent_backend.election_event WHERE id = $1",
        &[&open_id],
    )
    .await;
    assert_eq!(events[0].get::<_, i64>("n"), 1);

    let locked = rows::event(&services.hasura).await;
    let locked_id = uuid::Uuid::parse_str(&locked.election_event_id).unwrap();
    rows::execute(
        &services.hasura,
        "UPDATE sequent_backend.election_event SET presentation = $1
         WHERE id = $2
           AND set_config('sequent.trusted_write', 'on', true) = 'on'",
        &[&json!({"locked_down": "locked-down"}), &locked_id],
    )
    .await;
    let deleter = Claims::new(&locked.tenant_id, USER_ID)
        .roles([Permissions::ELECTION_EVENT_DELETE]);
    let (status, output) = response_json(
        post(
            &client,
            "/delete-election-event",
            &deleter,
            &json!({"election_event_id": locked.election_event_id}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{output}");
    assert_eq!(
        output["error_msg"],
        json!("The election event is locked down")
    );

    // A status that cannot be read means the checks cannot run: a server
    // error, not a delete.
    let unreadable = rows::event(&services.hasura).await;
    let unreadable_id =
        uuid::Uuid::parse_str(&unreadable.election_event_id).unwrap();
    rows::execute(
        &services.hasura,
        "UPDATE sequent_backend.election_event SET status = $1
         WHERE id = $2
           AND set_config('sequent.trusted_write', 'on', true) = 'on'",
        &[&json!({"voting_status": "NOT-A-STATUS"}), &unreadable_id],
    )
    .await;
    let deleter = Claims::new(&unreadable.tenant_id, USER_ID)
        .roles([Permissions::ELECTION_EVENT_DELETE]);
    let (status, error) = text(
        post(
            &client,
            "/delete-election-event",
            &deleter,
            &json!({"election_event_id": unreadable.election_event_id}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::InternalServerError);
    assert!(
        error.starts_with(
            "Failed to check whether the election event can be deleted"
        ),
        "{error}"
    );
}

#[rocket::async_test]
async fn setting_the_google_service_account_key_stores_it_in_the_vault() {
    const TEST: &str = "request_boundaries::setting_the_google_service_account_key_stores_it_in_the_vault";
    if !is_isolated_child() {
        run_isolated_with_postgres(TEST, "http://127.0.0.1:9", true);
        return;
    }
    use crate::route_services::{
        json as response_json, post, rows, text, Services,
    };
    use windmill::services::google_meet::service_account_key_secret_name;
    std::env::set_var("MASTER_SECRET", "5a".repeat(32));
    let services = Services::on_test_database().await;
    let database: String =
        rows::query(&services.hasura, "SELECT current_database() AS name", &[])
            .await[0]
            .get("name");
    std::env::set_var("HASURA_DB__DBNAME", database);
    let client = services.client().await;
    let service_account_key = |token_uri: &str| {
        json!({
            "type": "service_account",
            "private_key": "-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----\n",
            "client_email": "meet@project.iam.gserviceaccount.com",
            "token_uri": token_uri,
        })
    };

    let event = rows::event(&services.hasura).await;
    let tenant_id = uuid::Uuid::parse_str(&event.tenant_id).unwrap();
    rows::execute(
        &services.hasura,
        "UPDATE sequent_backend.tenant SET settings = $1 WHERE id = $2",
        &[
            &json!({
                "gapi_key": service_account_key("https://oauth2.googleapis.com/token"),
                "gapi_email": "organizer@example.org",
            }),
            &tenant_id,
        ],
    )
    .await;
    let writer = Claims::new(&event.tenant_id, USER_ID)
        .roles([Permissions::TENANT_WRITE]);

    let (status, output) = response_json(
        post(
            &client,
            "/set-google-service-account-key",
            &writer,
            &json!({"service_account_key": service_account_key("https://oauth2.googleapis.com/token")}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{output}");
    assert_eq!(
        output,
        json!({"client_email": "meet@project.iam.gserviceaccount.com"})
    );
    let secrets = rows::query(
        &services.hasura,
        "SELECT count(*) AS n FROM sequent_backend.secret WHERE tenant_id = $1 AND key = $2",
        &[&tenant_id, &service_account_key_secret_name(&event.tenant_id)],
    )
    .await;
    assert_eq!(secrets[0].get::<_, i64>("n"), 1);
    let settings = rows::query(
        &services.hasura,
        "SELECT settings FROM sequent_backend.tenant WHERE id = $1",
        &[&tenant_id],
    )
    .await;
    assert_eq!(
        settings[0].get::<_, Value>("settings"),
        json!({"gapi_email": "organizer@example.org"})
    );
    // The change is one entry of the tenant's log, without the key.
    let entries = rows::query(
        &services.hasura,
        "SELECT election_event_id, user_id, details FROM sequent_backend.audit_log_outbox WHERE tenant_id = $1",
        &[&tenant_id],
    )
    .await;
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].get::<_, Option<uuid::Uuid>>("election_event_id"),
        None
    );
    let details = entries[0].get::<_, Value>("details");
    assert_eq!(
        details["changes"],
        json!([{"path": "settings.gapi_key", "secret": true}])
    );
    assert_eq!(
        details["extra"]["client_email"],
        "meet@project.iam.gserviceaccount.com"
    );
    assert!(!details.to_string().contains("PRIVATE KEY"));

    // A key that cannot be used is the caller's mistake.
    for (body, message) in [
        (
            json!({"service_account_key": {"type": "service_account"}}),
            "Failed to parse service account key",
        ),
        (
            json!({"service_account_key": service_account_key("https://example.org/token")}),
            "Token URI not allowed",
        ),
    ] {
        let (status, error) = text(
            post(&client, "/set-google-service-account-key", &writer, &body)
                .await,
        )
        .await;
        assert_eq!(status, Status::BadRequest, "{error}");
        assert!(error.contains(message), "{error}");
    }

    // A tenant that does not exist cannot own a secret.
    let stranger = Claims::new(&uuid::Uuid::new_v4().to_string(), USER_ID)
        .roles([Permissions::TENANT_WRITE]);
    let (status, error) = text(
        post(
            &client,
            "/set-google-service-account-key",
            &stranger,
            &json!({"service_account_key": service_account_key("https://oauth2.googleapis.com/token")}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::InternalServerError, "{error}");
}
