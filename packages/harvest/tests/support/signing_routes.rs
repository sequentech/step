// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The signing routes on the migrated test database: a missing permission
//! is 403, a refused signature is 422 with its check, a request that takes
//! no more steps is 409, and steps that are allowed commit.

use crate::route_services::rows::{self, Event};
use crate::route_services::{json, post, Services};
use crate::test_claims::Claims;
use deadpool_postgres::{Pool, Transaction};
use rocket::http::Status;
use sequent_core::signing::{
    CertificateCheckId, CertificateCheckResult, RequesterSigning,
    RevocationStatus, SigningAction, SigningRequirement, SigningRule,
};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use windmill::postgres::signing::{get_signing_request, upsert_signing_rule};
use windmill::services::signing::approve::{NoDocumentSigner, SigningServices};
use windmill::services::signing::certificates::{
    CertificateVerification, CertificateVerificationInput, CertificateVerifier,
    RegistrationState,
};
use windmill::services::signing::executors::SigningExecutorRegistry;
use windmill::services::signing::guard::{
    guard, GuardOutcome, GuardRequest, RequestScope,
};
use windmill::services::signing::SigningCaller;

const ACTION: SigningAction = SigningAction::CloseVoting;
const LABEL: &str = "route-post";

/// Refuses every certificate as untrusted.
struct UntrustedIssuer;

#[rocket::async_trait]
impl CertificateVerifier for UntrustedIssuer {
    async fn verify(
        &self,
        _tx: &Transaction<'_>,
        _input: &CertificateVerificationInput<'_>,
    ) -> anyhow::Result<CertificateVerification> {
        Ok(CertificateVerification {
            checks: vec![CertificateCheckResult {
                id: CertificateCheckId::TrustedIssuer,
                ok: false,
                detail: Some("Test Root".into()),
            }],
            certificate: None,
            revocation_status: RevocationStatus::Unchecked,
            registration: RegistrationState::NotRegistered,
            other_holder: None,
        })
    }
}

fn ids(event: &Event) -> (Uuid, Uuid) {
    (
        Uuid::parse_str(&event.tenant_id).unwrap(),
        Uuid::parse_str(&event.election_event_id).unwrap(),
    )
}

/// An event with a Post, a close-voting rule and one waiting request.
async fn waiting_request(pool: &Pool) -> (Event, Uuid) {
    let event = rows::event(pool).await;
    let (tenant, election_event) = ids(&event);
    let post = Uuid::new_v4();
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id, permission_label)
         VALUES ($1, $2, $3, $4)",
        &[&post, &tenant, &election_event, &LABEL],
    )
    .await
    .unwrap();
    upsert_signing_rule(
        &tx,
        tenant,
        election_event,
        &SigningRule {
            action: ACTION,
            requirement: SigningRequirement::Required,
            signatures: 2,
            requester_signing: RequesterSigning::NotAllowed,
            expires_minutes: None,
            revision: 0,
        },
        0,
        "manager",
        Some("Configuration Manager"),
    )
    .await
    .unwrap()
    .unwrap();
    let requester = SigningCaller::from_claims(&operator(&event).build());
    let outcome = guard(
        &tx,
        &requester,
        &GuardRequest {
            action: ACTION,
            scope: RequestScope {
                tenant_id: tenant,
                election_event_id: election_event,
                election_id: Some(post),
                area_id: None,
                trustee_id: None,
                subject_key: None,
            },
            subject: json!({"channels": ["ONLINE"]}),
            document: None,
            config_revision: None,
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let GuardOutcome::SigningRequired(summary) = outcome else {
        panic!("{outcome:?}");
    };
    (event, summary.id)
}

/// Who starts close voting at the Post: the requester, without
/// signing-requests-cancel or any sign permission.
fn operator(event: &Event) -> Claims {
    Claims::new(&event.tenant_id, "operator")
        .username("operator")
        .roles([Permissions::ELECTION_STATE_WRITE])
}

fn signer(event: &Event, user: &str, labels: &[&str]) -> Claims {
    Claims::new(&event.tenant_id, user)
        .username(user)
        .roles([ACTION.sign_permission()])
        .permission_labels(labels)
}

fn approval(request_id: Uuid) -> Value {
    json!({
        "request_id": request_id,
        "chain_pem": ["-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n"],
        "algorithm": "ecdsa-p256-sha256",
        "payload_signature_b64": "AAEC",
    })
}

async fn steps(pool: &Pool, event: &Event) -> Vec<String> {
    let (_, election_event) = ids(event);
    pool.get()
        .await
        .unwrap()
        .query(
            "SELECT statement_kind FROM sequent_backend.signing_log_outbox
             WHERE election_event_id = $1 AND entry = 0 ORDER BY id",
            &[&election_event],
        )
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get(0))
        .collect()
}

#[rocket::async_test]
async fn a_missing_permission_is_forbidden() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let (event, request_id) = waiting_request(&services.hasura).await;
    let request = json!({"request_id": request_id});
    let stranger = Claims::new(&event.tenant_id, "stranger")
        .roles([Permissions::SIGNING_RULES_READ]);
    let other_post = signer(&event, "jose", &["another-post"]);
    // Starting the action doesn't make someone else's request theirs.
    let other_starter = Claims::new(&event.tenant_id, "starter")
        .username("starter")
        .roles([Permissions::ELECTION_STATE_WRITE]);
    let rule = json!({
        "election_event_id": event.election_event_id, "action": "close-voting",
        "requirement": "required", "signatures": 1, "requester_signing": "allowed",
        "expires_minutes": null, "roles": {"add": ["g"], "remove": []}, "expected_revision": 1,
    });
    let rule_writer = Claims::new(&event.tenant_id, "manager")
        .roles([Permissions::SIGNING_RULES_WRITE]);
    for (path, claims, body) in [
        ("/signing-requests/get", &stranger, request.clone()),
        ("/signing-requests/get", &other_post, request.clone()),
        ("/signing-requests/approve", &stranger, approval(request_id)),
        (
            "/signing-requests/approve",
            &other_post,
            approval(request_id),
        ),
        ("/signing-requests/handover", &stranger, request.clone()),
        ("/signing-requests/cancel", &other_post, request.clone()),
        ("/signing-requests/cancel", &other_starter, request.clone()),
        (
            "/signing-requests/open-failures",
            &stranger,
            json!({"request_id": request_id, "file_name": "a.p12", "reason": "unreadable"}),
        ),
        (
            "/signing-requests/export",
            &stranger,
            json!({"election_event_id": event.election_event_id}),
        ),
        ("/signing-rules/put", &rule_writer, rule.clone()),
        (
            "/signing-rules/capacity",
            &other_post,
            json!({"election_event_id": event.election_event_id, "action": "close-voting"}),
        ),
    ] {
        let (status, body) =
            json(post(&client, path, claims, &body).await).await;
        assert_eq!(status, Status::Forbidden, "{path}: {body}");
        assert_eq!(body["extensions"]["code"], "forbidden", "{path}");
    }
    // The two sign attempts are logged as refusals; nothing else is.
    assert_eq!(
        steps(&services.hasura, &event).await,
        [
            "SigningRequestCreated",
            "SigningSignatureRefused",
            "SigningSignatureRefused"
        ]
    );
}

#[rocket::async_test]
async fn a_refused_signature_is_422_with_its_check_and_is_logged() {
    let services =
        Services::on_test_database()
            .await
            .with_signing(SigningServices {
                verifier: Arc::new(UntrustedIssuer),
                executors: SigningExecutorRegistry::default(),
                documents: Arc::new(NoDocumentSigner),
                exports: Arc::new(crate::route_services::RowOnlyExports),
            });
    let client = services.client().await;
    let (event, request_id) = waiting_request(&services.hasura).await;
    let maria = signer(&event, "maria", &[LABEL]);
    let (status, body) = json(
        post(
            &client,
            "/signing-requests/approve",
            &maria,
            &approval(request_id),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::UnprocessableEntity, "{body}");
    assert_eq!(body["extensions"]["code"], "signing-refused");
    assert_eq!(body["extensions"]["check"], "trusted-issuer");
    assert_eq!(body["message"], "Test Root");
    assert_eq!(
        steps(&services.hasura, &event).await,
        ["SigningRequestCreated", "SigningSignatureRefused"]
    );

    // Not base64: refused before anything is read.
    let mut bad = approval(request_id);
    bad["payload_signature_b64"] = json!("not base64!");
    let (status, _) =
        json(post(&client, "/signing-requests/approve", &maria, &bad).await)
            .await;
    assert_eq!(status, Status::BadRequest);
}

#[rocket::async_test]
async fn allowed_steps_commit_and_a_finished_request_takes_no_more() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let (event, request_id) = waiting_request(&services.hasura).await;
    let request = json!({"request_id": request_id});
    let jose = signer(&event, "jose", &[LABEL]);
    let (status, body) = json(
        post(&client, "/signing-requests/handover", &jose, &request).await,
    )
    .await;
    assert_eq!(
        (status, body),
        (Status::Ok, json!({"request_id": request_id}))
    );
    let (status, _) = json(
        post(
            &client,
            "/signing-requests/open-failures",
            &jose,
            &json!({"request_id": request_id, "file_name": "jose.p12", "reason": "wrong-password"}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok);
    let (status, _) = json(
        post(
            &client,
            "/signing-requests/cancel",
            &operator(&event),
            &json!({"request_id": request_id, "reason": "wrong channel"}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok);
    let (tenant, election_event) = ids(&event);
    let mut db = services.hasura.get().await.unwrap();
    let tx = db.transaction().await.unwrap();
    let row = get_signing_request(&tx, tenant, election_event, request_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.open_failures, 1);
    assert_eq!(row.cancelled_by.as_deref(), Some("operator"));

    // A cancelled request takes no more steps; an unknown one isn't found.
    let (status, body) = json(
        post(&client, "/signing-requests/handover", &jose, &request).await,
    )
    .await;
    assert_eq!(status, Status::Conflict, "{body}");
    assert_eq!(
        body["extensions"],
        json!({"code": "request-closed", "status": "cancelled"})
    );
    let (status, body) = json(
        post(
            &client,
            "/signing-requests/handover",
            &jose,
            &json!({"request_id": Uuid::new_v4()}),
        )
        .await,
    )
    .await;
    assert_eq!(
        (status, &body["extensions"]["code"]),
        (Status::NotFound, &json!("not-found"))
    );

    let auditor = Claims::new(&event.tenant_id, "auditor")
        .roles([Permissions::SIGNING_REQUESTS_EXPORT]);
    let (status, export) = json(
        post(
            &client,
            "/signing-requests/export",
            &auditor,
            &json!({"election_event_id": event.election_event_id}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok);
    assert_eq!(export["rows"], 1);
    let document_id =
        Uuid::parse_str(export["document_id"].as_str().unwrap()).unwrap();
    // The store's download link comes back, so the auditor, who may export
    // but not read documents, downloads the file without the document routes.
    assert_eq!(
        export["url"],
        json!(crate::route_services::export_url(&document_id.to_string()))
    );
    let name: String = services
        .hasura
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT name FROM sequent_backend.document WHERE id = $1",
            &[&document_id],
        )
        .await
        .unwrap()
        .get(0);
    assert!(name.ends_with(".csv"), "{name}");
    assert_eq!(
        steps(&services.hasura, &event).await,
        [
            "SigningRequestCreated",
            "SigningHandover",
            "SigningCertificateOpenFailed",
            "SigningRequestCancelled",
            "SigningRequestsExported"
        ]
    );
}

#[test]
fn each_refusal_has_its_status_and_code() {
    use crate::routes::signing::signing_failure;
    use sequent_core::signing::SigningRequestStatus;
    use windmill::services::signing::{InvalidReason, SigningError};
    let cases = [
        (
            SigningError::Refused {
                check: CertificateCheckId::AlreadySigned,
                message: "again".into(),
                other_holder: None,
            },
            Status::UnprocessableEntity,
            json!({"code": "already-signed", "check": "already-signed"}),
        ),
        (
            SigningError::Refused {
                check: CertificateCheckId::ValidNow,
                message: "expired certificate".into(),
                other_holder: None,
            },
            Status::UnprocessableEntity,
            json!({"code": "signing-refused", "check": "valid-now"}),
        ),
        (
            SigningError::Refused {
                check: CertificateCheckId::RegisteredToOther,
                message: "Maria Santos".into(),
                other_holder: Some(
                    windmill::services::signing::certificates::OtherHolder {
                        user_id: "maria".into(),
                        display_name: "Maria Santos".into(),
                    },
                ),
            },
            Status::UnprocessableEntity,
            json!({"code": "signing-refused", "check": "registered-to-other", "user_id": "maria", "display_name": "Maria Santos"}),
        ),
        (
            SigningError::Closed {
                status: SigningRequestStatus::Expired,
                message: "expired".into(),
            },
            Status::Conflict,
            json!({"code": "request-closed", "status": "expired"}),
        ),
        (
            SigningError::Conflict("stale".into()),
            Status::Conflict,
            json!({"code": "conflict"}),
        ),
        (
            SigningError::invalid(InvalidReason::LockedDown, "locked"),
            Status::Conflict,
            json!({"code": "locked-down"}),
        ),
        (
            SigningError::bad_input("bad"),
            Status::BadRequest,
            json!({"code": "invalid", "reason": "input"}),
        ),
        (
            SigningError::Internal(anyhow::anyhow!("secret detail")),
            Status::InternalServerError,
            json!({"code": "internal"}),
        ),
    ];
    for (error, status, extensions) in cases {
        let failure = signing_failure(error);
        assert_eq!(failure.status, status);
        let body = failure.body();
        assert_eq!(body["extensions"], extensions);
        assert!(!body["message"].as_str().unwrap().contains("secret detail"));
    }
}
