// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The signing certificate routes on the migrated test database, with the
//! Keycloak tables standing in for the tenant realm: what each answers, the
//! rows and names it writes, and its typed errors.

use crate::route_services::rows::{self, execute, keycloak_realm, Event};
use crate::route_services::{bearer, json, post, Services};
use crate::test_claims::Claims;
use deadpool_postgres::Pool;
use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::Client;
use sequent_core::services::keycloak::get_tenant_realm;
use sequent_core::signing::{SigningAction, SigningRequestStatus};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use uuid::Uuid;

// Formatted by windmill, which owns it (harvest's rustfmt.toml is narrower).
#[rustfmt::skip]
#[path = "../../../windmill/tests/support/signing_pki.rs"]
mod signing_pki;
use signing_pki::{Issued, Pki};

/// A user of the event's tenant realm with a first and last name.
async fn person(
    pool: &Pool,
    event: &Event,
    username: &str,
    first: &str,
    last: &str,
) -> String {
    let realm = get_tenant_realm(&event.tenant_id);
    let realm_id = match rows::query(
        pool,
        "SELECT id FROM realm WHERE name = $1",
        &[&realm],
    )
    .await
    .first()
    {
        Some(row) => row.get::<_, String>(0),
        None => keycloak_realm(pool, &realm).await,
    };
    let id = Uuid::new_v4().to_string();
    execute(
        pool,
        "INSERT INTO user_entity (id, realm_id, username, first_name, last_name, enabled,
             email_verified)
         VALUES ($1, $2, $3, $4, $5, true, true)",
        &[&id, &realm_id, &username, &first, &last],
    )
    .await;
    id
}

fn caller(event: &Event, user_id: &str, permissions: &[Permissions]) -> Claims {
    Claims::new(&event.tenant_id, user_id)
        .username("caller")
        .roles(permissions)
}

async fn send(
    client: &Client,
    method: rocket::http::Method,
    path: String,
    claims: &Claims,
    body: &Value,
) -> (Status, Value) {
    json(
        client
            .req(method, path)
            .header(ContentType::JSON)
            .header(bearer(claims))
            .body(body.to_string())
            .dispatch()
            .await,
    )
    .await
}

fn code(body: &Value) -> &str {
    body["extensions"]["code"].as_str().unwrap_or_default()
}

/// Trusts the test root and individual CA through the route.
async fn trust(client: &Client, event: &Event, officer: &str) {
    let pki = Pki::get();
    let (status, body) = json(
        post(
            client,
            "/signing-issuers",
            &caller(event, officer, &[Permissions::SIGNING_ISSUERS_WRITE]),
            &json!({
                "election_event_id": event.election_event_id,
                "pem": format!("{}{}", pki.root.pem(), pki.individual_ca.pem()),
            }),
        )
        .await,
    )
    .await;
    assert_eq!(
        (status, body),
        (
            Status::Ok,
            json!({"imported": 2, "skipped": 0, "errors": []})
        )
    );
}

async fn put_checks(
    client: &Client,
    event: &Event,
    officer: &str,
    checks: Value,
) -> (Status, Value) {
    let mut body = checks;
    body["election_event_id"] = json!(event.election_event_id);
    send(
        client,
        rocket::http::Method::Put,
        "/signing-checks".to_owned(),
        &caller(event, officer, &[Permissions::SIGNING_CHECKS_WRITE]),
        &body,
    )
    .await
}

fn dont_check(revision: i64) -> Value {
    json!({
        "revocation_check": "dont-check", "crl_unavailable": "refuse",
        "registration": "on-first-use", "post_binding": "one-post",
        "expected_revision": revision,
    })
}

#[rocket::async_test]
async fn issuers_import_from_pem_or_der_and_remove_by_id() {
    let services = Services::on_test_database().await;
    let pool = services.hasura.clone();
    let client = services.client().await;
    let event = rows::event(&pool).await;
    let officer = person(&pool, &event, "olivia", "Olivia", "Officer").await;
    trust(&client, &event, &officer).await;

    // The root again, as a DER .cer: skipped. A leaf: refused.
    use base64::Engine;
    let pki = Pki::get();
    let der = base64::engine::general_purpose::STANDARD.encode(pki.root.der());
    let claims =
        caller(&event, &officer, &[Permissions::SIGNING_ISSUERS_WRITE]);
    let (status, body) = json(
    post(
        &client,
        "/signing-issuers",
        &claims,
        &json!({"election_event_id": event.election_event_id, "der_base64": der}),
    )
    .await,
)
.await;
    assert_eq!(
        (status, body),
        (
            Status::Ok,
            json!({"imported": 0, "skipped": 1, "errors": []})
        )
    );
    let (_, body) = json(
    post(
        &client,
        "/signing-issuers",
        &claims,
        &json!({"election_event_id": event.election_event_id, "pem": pki.maria.pem()}),
    )
    .await,
)
.await;
    assert_eq!(
        body["errors"],
        json!(["Certificate 1: Maria Santos is not a certificate authority"])
    );
    let (status, body) = json(
        post(
            &client,
            "/signing-issuers",
            &claims,
            &json!({"election_event_id": event.election_event_id}),
        )
        .await,
    )
    .await;
    assert_eq!((status, code(&body)), (Status::BadRequest, "invalid"));

    let issuer_id: Uuid = rows::query(
        &pool,
        "SELECT id FROM sequent_backend.certificate_authority
         WHERE election_event_id = $1 AND common_name = 'Test Staff Root CA'",
        &[&Uuid::parse_str(&event.election_event_id).unwrap()],
    )
    .await[0]
        .get(0);
    let client = &client;
    let delete = |claims: Claims| {
        let body = json!({"election_event_id": event.election_event_id});
        let path = format!("/signing-issuers/{issuer_id}");
        async move {
            send(client, rocket::http::Method::Delete, path, &claims, &body)
                .await
        }
    };
    let (status, body) = delete(caller(
        &event,
        &officer,
        &[Permissions::SIGNING_CERTIFICATES_READ],
    ))
    .await;
    assert_eq!((status, code(&body)), (Status::Forbidden, "forbidden"));
    let writer =
        || caller(&event, &officer, &[Permissions::SIGNING_ISSUERS_WRITE]);
    let (status, body) = delete(writer()).await;
    assert_eq!(
        (status, body),
        (Status::Ok, json!({"issuer_id": issuer_id}))
    );
    let (status, body) = delete(writer()).await;
    assert_eq!((status, code(&body)), (Status::NotFound, "not-found"));
}

#[rocket::async_test]
async fn checks_save_on_their_revision_with_the_officers_name() {
    let services = Services::on_test_database().await;
    let pool = services.hasura.clone();
    let client = services.client().await;
    let event = rows::event(&pool).await;
    let officer = person(&pool, &event, "olivia", "Olivia", "Officer").await;
    assert_eq!(
        put_checks(&client, &event, &officer, dont_check(0)).await,
        (Status::Ok, json!({"revision": 1}))
    );
    let (status, body) =
        put_checks(&client, &event, &officer, dont_check(0)).await;
    assert_eq!((status, code(&body)), (Status::Conflict, "conflict"));
    let saved = rows::query(
        &pool,
        "SELECT revocation_check, updated_by, updated_by_name FROM sequent_backend.signing_checks
         WHERE election_event_id = $1",
        &[&Uuid::parse_str(&event.election_event_id).unwrap()],
    )
    .await;
    assert_eq!(saved[0].get::<_, String>(0), "dont-check");
    assert_eq!(saved[0].get::<_, String>(1), officer);
    assert_eq!(
        saved[0].get::<_, Option<String>>(2).as_deref(),
        Some("Olivia Officer")
    );
    let mut bad = dont_check(1);
    bad["registration"] = json!("whoever-asks");
    let response = client
        .put("/signing-checks")
        .header(ContentType::JSON)
        .header(bearer(&caller(
            &event,
            &officer,
            &[Permissions::SIGNING_CHECKS_WRITE],
        )))
        .body(bad.to_string())
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::UnprocessableEntity);
}

async fn register(
    client: &Client,
    event: &Event,
    officer: &str,
    user_id: &str,
    signer: &Issued,
) -> (Status, Value) {
    json(
        post(
            client,
            "/staff-certificates",
            &caller(
                event,
                officer,
                &[Permissions::SIGNING_CERTIFICATES_REGISTER],
            ),
            &json!({
                "election_event_id": event.election_event_id,
                "user_id": user_id,
                "pem": signer.pem(),
            }),
        )
        .await,
    )
    .await
}

#[rocket::async_test]
async fn a_security_officer_registers_and_revokes_certificates() {
    let services = Services::on_test_database().await;
    let pool = services.hasura.clone();
    let client = services.client().await;
    let event = rows::event(&pool).await;
    let officer = person(&pool, &event, "olivia", "Olivia", "Officer").await;
    let maria = person(&pool, &event, "msantos", "Maria", "Santos").await;
    trust(&client, &event, &officer).await;
    let pki = Pki::get();

    let (status, body) =
        register(&client, &event, &officer, &maria, &pki.maria).await;
    assert_eq!(status, Status::Ok, "{body}");
    let certificate_id = body["certificate_id"].as_str().unwrap().to_owned();
    let row = &rows::query(
        &pool,
        "SELECT user_id, username, user_display_name, registered_by, registered_by_name
         FROM sequent_backend.staff_certificate WHERE id = $1",
        &[&Uuid::parse_str(&certificate_id).unwrap()],
    )
    .await[0];
    assert_eq!(row.get::<_, String>(0), maria);
    assert_eq!(row.get::<_, String>(1), "msantos");
    assert_eq!(
        row.get::<_, Option<String>>(2).as_deref(),
        Some("Maria Santos")
    );
    assert_eq!(row.get::<_, String>(3), officer);
    assert_eq!(
        row.get::<_, Option<String>>(4).as_deref(),
        Some("Olivia Officer")
    );

    // Another person with her certificate, a foreign chain, a stranger.
    let jose = person(&pool, &event, "jreyes", "Jose", "Reyes").await;
    let (status, body) =
        register(&client, &event, &officer, &jose, &pki.maria).await;
    assert_eq!(status, Status::UnprocessableEntity);
    assert_eq!(
        body,
        json!({"message": "Maria Santos", "extensions": {
            "code": "signing-refused", "check": "registered-to-other", "reason": "registered-to-other",
            "user_id": maria, "display_name": "Maria Santos"}})
    );
    let (status, body) =
        register(&client, &event, &officer, &jose, &pki.foreign_signer).await;
    assert_eq!(
        (status, code(&body), &body["extensions"]["check"]),
        (
            Status::UnprocessableEntity,
            "signing-refused",
            &json!("trusted-issuer")
        )
    );
    let (status, body) = register(
        &client,
        &event,
        &officer,
        &Uuid::new_v4().to_string(),
        &pki.jose,
    )
    .await;
    assert_eq!((status, code(&body)), (Status::NotFound, "not-found"));
    let (status, body) =
        register(&client, &event, &officer, &maria, &pki.maria).await;
    assert_eq!((status, code(&body)), (Status::Conflict, "conflict"));

    let client = &client;
    let revoke = |reason: &str| {
        let body = json!({"election_event_id": event.election_event_id, "reason": reason});
        let path = format!("/staff-certificates/{certificate_id}/revoke");
        let claims = caller(
            &event,
            &officer,
            &[Permissions::SIGNING_CERTIFICATES_REVOKE],
        );
        async move {
            send(client, rocket::http::Method::Post, path, &claims, &body).await
        }
    };
    let (status, body) = revoke(" ").await;
    assert_eq!((status, code(&body)), (Status::BadRequest, "invalid"));
    assert_eq!(
        revoke("token lost").await,
        (Status::Ok, json!({"certificate_id": certificate_id}))
    );
    let (status, body) = revoke("token lost").await;
    assert_eq!((status, code(&body)), (Status::Conflict, "conflict"));
    // A revoked key isn't registered again.
    let (status, body) =
        register(&client, &event, &officer, &maria, &pki.maria).await;
    assert_eq!(
        (
            status,
            code(&body),
            &body["extensions"]["check"],
            &body["extensions"]["reason"]
        ),
        (
            Status::UnprocessableEntity,
            "signing-refused",
            &json!("registered"),
            &json!("revoked")
        )
    );
    let revoked = &rows::query(
        &pool,
        "SELECT status, revoke_reason, revoked_by_name FROM sequent_backend.staff_certificate
         WHERE id = $1",
        &[&Uuid::parse_str(&certificate_id).unwrap()],
    )
    .await[0];
    assert_eq!(revoked.get::<_, String>(0), "revoked");
    assert_eq!(
        revoked.get::<_, Option<String>>(1).as_deref(),
        Some("token lost")
    );
    assert_eq!(
        revoked.get::<_, Option<String>>(2).as_deref(),
        Some("Olivia Officer")
    );
}

/// A waiting request of `action` at a new Post labelled `label`.
async fn request(
    pool: &Pool,
    event: &Event,
    action: SigningAction,
    label: Option<&str>,
    status: SigningRequestStatus,
) -> Uuid {
    let election = Uuid::parse_str(&event.election(pool).await).unwrap();
    let id = Uuid::new_v4();
    let payload = format!("{{\"request\":\"{id}\"}}");
    let (completed_at, cancel_reason) = match status {
        SigningRequestStatus::Cancelled => (None, Some("by-operator")),
        SigningRequestStatus::Waiting => (None, None),
        _ => (Some(chrono::Utc::now()), None),
    };
    execute(
        pool,
        "INSERT INTO sequent_backend.signing_request
             (id, tenant_id, election_event_id, action, election_id, scope_key, subject,
              canonical_payload, payload_sha256, code, rule_revision, rule_snapshot, required,
              status, requested_by, requested_by_username, permission_label, completed_at,
              cancel_reason)
         VALUES ($1, $2, $3, $4, $5, $6, '{}', $7, $8, '7F3A-91C2', 1, '{}', 2, $9,
                 'requester', 'requester', $10, $11, $12)",
        &[
            &id,
            &Uuid::parse_str(&event.tenant_id).unwrap(),
            &Uuid::parse_str(&event.election_event_id).unwrap(),
            &action.to_string(),
            &election,
            &id.to_string(),
            &payload,
            &"0".repeat(64),
            &status.to_string(),
            &label,
            &completed_at,
            &cancel_reason,
        ],
    )
    .await;
    id
}

#[rocket::async_test]
async fn the_dry_run_checks_the_signers_certificate_for_the_request() {
    let services = Services::on_test_database().await;
    let pool = services.hasura.clone();
    let client = services.client().await;
    let event = rows::event(&pool).await;
    let officer = person(&pool, &event, "olivia", "Olivia", "Officer").await;
    let maria = person(&pool, &event, "msantos", "Maria", "Santos").await;
    let jose = person(&pool, &event, "jreyes", "Jose", "Reyes").await;
    trust(&client, &event, &officer).await;
    assert_eq!(
        put_checks(&client, &event, &officer, dont_check(0)).await.0,
        Status::Ok
    );
    assert_eq!(
        register(&client, &event, &officer, &maria, &Pki::get().maria)
            .await
            .0,
        Status::Ok
    );
    let pki = Pki::get();
    let close = request(
        &pool,
        &event,
        SigningAction::CloseVoting,
        Some("post-a"),
        SigningRequestStatus::Waiting,
    )
    .await;
    let client = &client;
    let dry_run = |user: &str,
                   labels: &[&str],
                   permissions: &[Permissions],
                   request: Uuid,
                   signer: &Issued| {
        let claims =
            caller(&event, user, permissions).permission_labels(labels);
        let body = json!({"chain_pem": [signer.pem()]});
        let path = format!("/signing-requests/{request}/check-certificate");
        async move {
            send(client, rocket::http::Method::Post, path, &claims, &body).await
        }
    };
    let sign = [Permissions::SIGN_CLOSE_VOTING];

    let (status, body) =
        dry_run(&maria, &["post-a"], &sign, close, &pki.maria).await;
    assert_eq!(status, Status::Ok, "{body}");
    let ids: Vec<&str> = body["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|check| check["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        [
            "trusted-issuer",
            "valid-now",
            "signing-key-usage",
            "not-revoked",
            "registered",
            "registered-to-other",
            "already-signed",
            "post-binding"
        ]
    );
    assert!(
        body["checks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|check| check["ok"] == json!(true)),
        "{body}"
    );
    // The chain's root CA, though the path stops at the trusted issuing CA.
    assert_eq!(body["checks"][0]["detail"], "Test Staff Root CA");
    assert_eq!(body["registration"], "registered");
    assert_eq!(body["revocation_status"], "unchecked");
    assert_eq!(body["certificate"]["common_name"], "Maria Santos");
    assert_eq!(body["certificate"]["key_algorithm"], "rsa-pkcs1-sha256");

    // Jose with her certificate: registered to Maria Santos, so it can't
    // be registered to him on first use either.
    let (status, body) =
        dry_run(&jose, &["post-a"], &sign, close, &pki.maria).await;
    assert_eq!(status, Status::Ok);
    assert_eq!(body["checks"][4]["id"], "registered");
    assert_eq!(body["checks"][4]["ok"], false, "{body}");
    assert_eq!(
        body["checks"][5],
        json!({"id": "registered-to-other", "ok": false, "detail": "Maria Santos"})
    );
    assert_eq!(body["registration"], "not-registered");

    // Without the action's sign permission, outside the Post, unknown or closed.
    let (status, body) = dry_run(
        &maria,
        &[],
        &[Permissions::SIGN_OPEN_VOTING],
        close,
        &pki.maria,
    )
    .await;
    assert_eq!((status, code(&body)), (Status::Forbidden, "forbidden"));
    let (status, body) =
        dry_run(&maria, &["post-b"], &sign, close, &pki.maria).await;
    assert_eq!((status, code(&body)), (Status::Forbidden, "forbidden"));
    // Without labels a person sees only unlabelled Posts, as in Hasura.
    let (status, body) = dry_run(&maria, &[], &sign, close, &pki.maria).await;
    assert_eq!((status, code(&body)), (Status::Forbidden, "forbidden"));
    let (status, body) =
        dry_run(&maria, &[], &sign, Uuid::new_v4(), &pki.maria).await;
    assert_eq!((status, code(&body)), (Status::NotFound, "not-found"));
    // Someone who signs nothing can't tell a request from no request.
    let (status, body) = dry_run(
        &maria,
        &[],
        &[Permissions::SIGNING_CERTIFICATES_READ],
        close,
        &pki.maria,
    )
    .await;
    assert_eq!((status, code(&body)), (Status::NotFound, "not-found"));
    // A waiting request past its expiry is closed.
    let expired = request(
        &pool,
        &event,
        SigningAction::CloseVoting,
        None,
        SigningRequestStatus::Waiting,
    )
    .await;
    execute(
        &pool,
        "UPDATE sequent_backend.signing_request SET expires_at = now() - interval '1 minute'
         WHERE id = $1",
        &[&expired],
    )
    .await;
    let (status, body) = dry_run(&maria, &[], &sign, expired, &pki.maria).await;
    assert_eq!(
        (status, body),
        (
            Status::Conflict,
            json!({"message": "The request is expired",
            "extensions": {"code": "request-closed", "status": "expired"}})
        )
    );
    let closed = request(
        &pool,
        &event,
        SigningAction::CloseVoting,
        None,
        SigningRequestStatus::Cancelled,
    )
    .await;
    let (status, body) = dry_run(&maria, &[], &sign, closed, &pki.maria).await;
    assert_eq!(
        (status, body),
        (
            Status::Conflict,
            json!({"message": "The request is cancelled",
            "extensions": {"code": "request-closed", "status": "cancelled"}})
        )
    );
    // Another tenant's request is not found.
    let other = rows::event(&pool).await;
    let foreign = request(
        &pool,
        &other,
        SigningAction::CloseVoting,
        None,
        SigningRequestStatus::Waiting,
    )
    .await;
    let (status, _) = dry_run(&maria, &[], &sign, foreign, &pki.maria).await;
    assert_eq!(status, Status::NotFound);
}

#[rocket::async_test]
async fn the_routes_answer_500_when_the_database_is_unreachable() {
    let client = Services::without_database().client().await;
    let event = Event {
        tenant_id: Uuid::new_v4().to_string(),
        election_event_id: Uuid::new_v4().to_string(),
    };
    let claims =
        caller(&event, "user", &[Permissions::SIGNING_CERTIFICATES_REVOKE]);
    let body =
        json!({"election_event_id": event.election_event_id, "reason": "lost"});
    let (status, body) = send(
        &client,
        rocket::http::Method::Post,
        format!("/staff-certificates/{}/revoke", Uuid::new_v4()),
        &claims,
        &body,
    )
    .await;
    assert_eq!(
        (status, code(&body)),
        (Status::InternalServerError, "internal")
    );
    assert_eq!(body["message"], "Internal error");
}
