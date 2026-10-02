// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The guard (one waiting request per scope, replaced when what it signs
//! changes), the signing panel, cancelling, handing over, certificate files
//! that didn't open and the export of an event's requests.

#[path = "support/schema.rs"]
mod schema;
#[path = "support/signing.rs"]
mod signing;

use deadpool_postgres::Transaction;
use sequent_core::signing::{
    CancelReason, CertificateOpenFailure, RequesterSigning, SigningAction, SigningRequestStatus,
};
use sequent_core::types::permissions::Permissions;
use serde_json::json;
use signing::*;
use std::sync::Arc;
use uuid::Uuid;
use windmill::services::signing::guard::{GuardOutcome, GuardRequest, SigningDocument};
use windmill::services::signing::requests::{
    cancel, export_requests, get_panel, handover, report_open_failure, ExportFilter, SignerStatus,
};
use windmill::services::signing::{InvalidReason, SigningCaller, SigningError};

const ACTION: SigningAction = SigningAction::GenerateElectionReturns;

fn summary(outcome: GuardOutcome) -> windmill::services::signing::guard::SigningRequestSummary {
    match outcome {
        GuardOutcome::SigningRequired(summary) => summary,
        GuardOutcome::Proceed => panic!("expected a signing request"),
    }
}

fn returns(w: &World, document: &str, config: Option<&str>) -> GuardRequest {
    let mut scope = w.scope(ACTION);
    scope.area_id = Some(w.area);
    GuardRequest {
        action: ACTION,
        scope,
        subject: json!({"report_type": "election-returns", "document_sha256": sha(document), "template_id": null}),
        document: Some(SigningDocument {
            document_id: None,
            sha256: sha(document),
        }),
        config_revision: config.map(str::to_owned),
    }
}

#[tokio::test]
async fn the_guard_keeps_one_waiting_request_per_scope() {
    let w = world("madrid-pe").await;
    let maria = caller("maria", &[], &[]);
    let jose = caller("jose", &[], &[]);
    // Without a rule, the action runs as it always did.
    assert_eq!(
        w.guard_request(&maria, &returns(&w, "er-v1", None), at(0))
            .await
            .unwrap(),
        GuardOutcome::Proceed
    );
    w.rule(ACTION, 3, RequesterSigning::Allowed, Some(120))
        .await;
    let first = summary(
        w.guard_request(&maria, &returns(&w, "er-v1", None), at(0))
            .await
            .unwrap(),
    );
    assert_eq!(
        (first.required, first.expires_at),
        (3, Some(at(0) + chrono::Duration::minutes(120)))
    );
    let row = w.request(first.id).await;
    assert_eq!(row.code, first.code);
    assert_eq!(row.permission_label.as_deref(), Some("madrid-pe"));
    assert_eq!(row.payload_sha256, sha(&row.canonical_payload));
    assert!(row
        .canonical_payload
        .starts_with("{\"action\":\"generate-election-returns\""));
    let payload: serde_json::Value = serde_json::from_str(&row.canonical_payload).unwrap();
    assert_eq!(payload["domain"], "step-signing/v1");
    assert_eq!(payload["request_id"], json!(row.id));
    assert_eq!(payload["code"], json!(first.code));
    assert_eq!(payload["action"], "generate-election-returns");
    assert_eq!(payload["tenant_id"], json!(w.tenant));
    assert_eq!(payload["election_event_id"], json!(w.event));
    assert_eq!(payload["election_id"], json!(w.post));
    assert_eq!(payload["area_id"], json!(w.area));
    assert_eq!(
        payload["subject"]["document_sha256"],
        json!(row.document_sha256)
    );
    assert_eq!(payload["requested_by"], "maria");

    // The same document, even from someone else: the same request.
    let again = summary(
        w.guard_request(&jose, &returns(&w, "er-v1", None), at(5))
            .await
            .unwrap(),
    );
    assert_eq!(again, first);

    // A recount: a new document supersedes it.
    let recount = summary(
        w.guard_request(&jose, &returns(&w, "er-v2", None), at(6))
            .await
            .unwrap(),
    );
    assert_ne!(recount.id, first.id);
    let old = w.request(first.id).await;
    assert_eq!(
        (old.status, old.cancel_reason),
        (
            SigningRequestStatus::Cancelled,
            Some(CancelReason::PayloadChanged)
        )
    );
    // The same document under a new configuration version.
    let versioned = summary(
        w.guard_request(&jose, &returns(&w, "er-v2", Some("2")), at(7))
            .await
            .unwrap(),
    );
    assert_ne!(versioned.id, recount.id);
    assert_eq!(
        w.request(recount.id).await.cancel_reason,
        Some(CancelReason::Superseded)
    );

    // Past its time, the waiting one expires and a new one starts.
    w.make_overdue(versioned.id).await;
    let later = summary(
        w.guard_request(&jose, &returns(&w, "er-v2", Some("2")), at(8))
            .await
            .unwrap(),
    );
    assert_ne!(later.id, versioned.id);
    assert_eq!(
        w.request(versioned.id).await.status,
        SigningRequestStatus::Expired
    );
    assert_eq!(
        w.steps().await,
        [
            "SigningRequestCreated",
            "SigningRequestCancelled",
            "SigningRequestCreated",
            "SigningRequestCancelled",
            "SigningRequestCreated",
            "SigningRequestExpired",
            "SigningRequestCreated"
        ]
    );
    w.assert_two_entries_per_step().await;
}

#[tokio::test]
async fn the_guard_refuses_a_scope_that_does_not_fit_the_action() {
    let w = world("madrid-pe").await;
    w.rule(ACTION, 2, RequesterSigning::Allowed, None).await;
    w.rule(
        SigningAction::ApproveConfiguration,
        2,
        RequesterSigning::Allowed,
        None,
    )
    .await;
    let maria = caller("maria", &[], &[]);
    let mut no_post = returns(&w, "er", None);
    no_post.scope.election_id = None;
    let mut unknown_post = returns(&w, "er", None);
    unknown_post.scope.election_id = Some(Uuid::new_v4());
    let mut event_with_post = returns(&w, "er", None);
    event_with_post.action = SigningAction::ApproveConfiguration;
    let mut float_subject = returns(&w, "er", None);
    float_subject.subject = json!({"ratio": 0.5});
    // The subject names another document than the request keeps, or none.
    let mut other_document = returns(&w, "er", None);
    other_document.document = Some(SigningDocument {
        document_id: None,
        sha256: sha("er-2"),
    });
    let mut no_document = returns(&w, "er", None);
    no_document.document = None;
    for (request, expected) in [
        (no_post, "bad request"),
        (unknown_post, "not found"),
        (event_with_post, "bad request"),
        (float_subject, "bad request"),
        (other_document, "bad request"),
        (no_document, "bad request"),
    ] {
        let got = match w.guard_request(&maria, &request, at(0)).await {
            Err(SigningError::Invalid {
                reason: InvalidReason::Input,
                ..
            }) => "bad request",
            Err(SigningError::NotFound(_)) => "not found",
            other => panic!("{other:?}"),
        };
        assert_eq!(got, expected);
    }
    assert!(w.steps().await.is_empty());
}

async fn keycloak_directory(tx: &Transaction<'_>, realm: &str, label: &str) {
    keycloak_tables(tx).await;
    let sign = ACTION.sign_permission().to_string();
    tx.batch_execute(&format!(
        "INSERT INTO realm VALUES ('r', '{realm}');
         INSERT INTO keycloak_role VALUES ('role', '{sign}', 'r');
         INSERT INTO keycloak_group (id, name, realm_id) VALUES ('g', 'sbei', 'r');
         INSERT INTO group_role_mapping VALUES ('role', 'g');
         INSERT INTO user_entity VALUES ('maria', 'maria', 'Maria', 'Santos', true, 'r', NULL),
             ('jose', 'jose', 'Jose', 'Rizal', true, 'r', NULL),
             ('ana', 'ana', 'Ana', 'Cruz', true, 'r', NULL),
             ('elsewhere', 'elsewhere', 'Else', 'Where', true, 'r', NULL),
             ('unlabelled', 'unlabelled', 'Una', 'Belled', true, 'r', NULL);
         INSERT INTO user_group_membership VALUES ('g', 'maria'), ('g', 'jose'), ('g', 'ana'),
             ('g', 'elsewhere'), ('g', 'unlabelled');
         INSERT INTO user_attribute VALUES ('title', 'Chairperson', 'maria'),
             ('permission_labels', '{label}', 'maria'), ('permission_labels', '{label}', 'jose'),
             ('permission_labels', '{label}', 'ana'), ('permission_labels', 'another-post', 'elsewhere');"
    ))
    .await
    .unwrap();
}

#[tokio::test]
async fn the_panel_shows_who_signed_with_which_certificate_and_who_is_next() {
    for label in ["madrid-pe", "faculty-of-science"] {
        let w = world(label).await;
        w.rule(ACTION, 3, RequesterSigning::Allowed, None).await;
        let maria_certificate = cert_with_pem("maria", "maria", "Maria", &real_pem("Maria Santos"));
        w.register("maria", &maria_certificate, None).await;
        let services = services_taking_documents(
            FakeVerifier::knowing(&[&maria_certificate]),
            vec![Arc::new(FakeExecutor::new(
                ACTION,
                FakeBehaviour::Succeed(json!({})),
            ))],
        );
        let maria = w.signer("maria", ACTION);
        let request = summary(
            w.guard_request(&maria, &returns(&w, "er", None), at(0))
                .await
                .unwrap(),
        );
        w.sign(&services, &maria, request.id, &maria_certificate, at(1))
            .await
            .unwrap();

        let mut hasura = w.pool.get().await.unwrap();
        let htx = hasura.transaction().await.unwrap();
        let mut keycloak = w.pool.get().await.unwrap();
        let ktx = keycloak.transaction().await.unwrap();
        keycloak_directory(&ktx, &format!("tenant-{}", w.tenant), label).await;

        let jose = w.signer("jose", ACTION);
        let panel = get_panel(&htx, &ktx, &jose, w.tenant, request.id)
            .await
            .unwrap();
        assert_eq!((panel.count, panel.required), (1, 3));
        assert_eq!(panel.rule.signatures, 3);
        assert_eq!(
            panel.election_name.as_deref(),
            Some(format!("Post {label}").as_str())
        );
        assert_eq!(panel.area_name.as_deref(), Some("Country"));
        assert_eq!(panel.request.code, request.code);
        // Those of another Post, and those without labels (who see no
        // labelled Post), are not its signers.
        let signers: Vec<(&str, Option<&str>, bool, SignerStatus, Option<&str>)> = panel
            .signers
            .iter()
            .map(|s| {
                (
                    s.username.as_str(),
                    s.title.as_deref(),
                    s.is_you,
                    s.status,
                    s.certificate_cn.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            signers,
            [
                ("ana", Some("sbei"), false, SignerStatus::NotSigned, None),
                ("jose", Some("sbei"), true, SignerStatus::NotSigned, None),
                (
                    "maria",
                    Some("Chairperson"),
                    false,
                    SignerStatus::Signed,
                    Some("Maria Santos")
                ),
            ]
        );
        assert_eq!(panel.signers[2].signed_at.is_some(), true);
        assert_eq!(
            panel.signers[2].certificate_subject.as_deref(),
            Some("O=Test Organization, CN=Maria Santos")
        );
        assert_eq!(panel.signers[2].display_name, "Maria Santos");
        assert_eq!(
            panel.request.requested_by_name.as_deref(),
            Some("maria display")
        );
        assert_eq!(
            panel
                .details
                .iter()
                .map(|d| (d.key.as_str(), d.value.clone()))
                .collect::<Vec<_>>(),
            [
                ("document_sha256", sha("er")),
                ("report_type", "election-returns".to_string()),
                ("template_id", String::new())
            ]
        );

        // Who may read it.
        for (who, allowed) in [
            (caller("maria", &[], &[]), true),
            (
                caller("reader", &[Permissions::SIGNING_REQUESTS_READ], &[label]),
                true,
            ),
            (
                caller(
                    "reader",
                    &[Permissions::SIGNING_REQUESTS_READ],
                    &["another-post"],
                ),
                false,
            ),
            // Without labels, no labelled Post is theirs.
            (
                caller("reader", &[Permissions::SIGNING_REQUESTS_READ], &[]),
                false,
            ),
            (
                caller("signer", &[ACTION.sign_permission()], &["another-post"]),
                false,
            ),
            (
                caller("stranger", &[Permissions::SIGNING_RULES_READ], &[]),
                false,
            ),
        ] {
            let result = get_panel(&htx, &ktx, &who, w.tenant, request.id).await;
            assert_eq!(result.is_ok(), allowed, "{}: {result:?}", who.user_id);
            if !allowed {
                assert!(matches!(result, Err(SigningError::Forbidden(_))));
            }
        }
        // Another tenant doesn't find it.
        assert!(matches!(
            get_panel(&htx, &ktx, &jose, Uuid::new_v4(), request.id).await,
            Err(SigningError::NotFound(_))
        ));
    }
}

async fn in_tx<T>(
    w: &World,
    f: impl for<'a> FnOnce(
        &'a Transaction<'a>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = T> + 'a>>,
) -> T {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let result = f(&tx).await;
    tx.commit().await.unwrap();
    result
}

#[tokio::test]
async fn cancel_handover_and_open_failures_are_logged_for_who_may_take_them() {
    let w = world("madrid-pe").await;
    w.rule(ACTION, 2, RequesterSigning::Allowed, None).await;
    let maria = w.signer("maria", ACTION);
    let request = summary(
        w.guard_request(&maria, &returns(&w, "er", None), at(0))
            .await
            .unwrap(),
    );
    let id = request.id;
    let tenant = w.tenant;
    let jose = w.signer("jose", ACTION);
    let outsider: SigningCaller = caller("outsider", &[ACTION.sign_permission()], &["tokyo-pe"]);

    // A signer of another Post neither hands over nor reports.
    let o = outsider.clone();
    let result = in_tx(&w, |tx| {
        Box::pin(async move { handover(tx, &o, tenant, id).await })
    })
    .await;
    assert!(matches!(result, Err(SigningError::Forbidden(_))));
    // Nor does someone who can't sign it.
    let stranger = caller("stranger", &[], &["madrid-pe"]);
    let result = in_tx(&w, |tx| {
        Box::pin(async move {
            report_open_failure(
                tx,
                &stranger,
                tenant,
                id,
                "stranger.p12",
                CertificateOpenFailure::Unreadable,
            )
            .await
        })
    })
    .await;
    assert!(matches!(result, Err(SigningError::Forbidden(_))));
    let j = jose.clone();
    in_tx(&w, |tx| {
        Box::pin(async move { handover(tx, &j, tenant, id).await })
    })
    .await
    .unwrap();
    let j = jose.clone();
    let failures = in_tx(&w, |tx| {
        Box::pin(async move {
            report_open_failure(
                tx,
                &j,
                tenant,
                id,
                "C:\\tokens\\jose.p12",
                CertificateOpenFailure::WrongPassword,
            )
            .await
        })
    })
    .await
    .unwrap();
    assert_eq!(failures, 1);
    assert_eq!(w.request(id).await.open_failures, 1);
    // Repeated within the throttle: counted, and logged once.
    let j = jose.clone();
    in_tx(&w, |tx| {
        Box::pin(async move { handover(tx, &j, tenant, id).await })
    })
    .await
    .unwrap();
    let j = jose.clone();
    let failures = in_tx(&w, |tx| {
        Box::pin(async move {
            report_open_failure(
                tx,
                &j,
                tenant,
                id,
                "jose.p12",
                CertificateOpenFailure::NoKey,
            )
            .await
        })
    })
    .await
    .unwrap();
    assert_eq!(failures, 2);

    // A signer can't cancel; an operator of the Post can.
    let j = jose.clone();
    let result = in_tx(&w, |tx| {
        Box::pin(async move { cancel(tx, &j, tenant, id, None).await })
    })
    .await;
    assert!(matches!(result, Err(SigningError::Forbidden(_))));
    // Nor can an operator without the Post's label.
    let unlabelled = caller("ofov", &[Permissions::SIGNING_REQUESTS_CANCEL], &[]);
    let result = in_tx(&w, |tx| {
        Box::pin(async move { cancel(tx, &unlabelled, tenant, id, None).await })
    })
    .await;
    assert!(matches!(result, Err(SigningError::Forbidden(_))));
    let operator = caller(
        "ofov",
        &[Permissions::SIGNING_REQUESTS_CANCEL],
        &["madrid-pe"],
    );
    let cancelled = in_tx(&w, |tx| {
        Box::pin(
            async move { cancel(tx, &operator, tenant, id, Some(&"stuck ".repeat(200))).await },
        )
    })
    .await
    .unwrap();
    assert_eq!(cancelled.cancel_reason, Some(CancelReason::ByOperator));
    assert_eq!(cancelled.cancelled_by.as_deref(), Some("ofov"));
    // Nothing more happens to a cancelled request.
    let m = maria.clone();
    let result = in_tx(&w, |tx| {
        Box::pin(async move { cancel(tx, &m, tenant, id, None).await })
    })
    .await;
    assert!(matches!(result, Err(SigningError::Closed { .. })));

    let rows = w.outbox().await;
    let failed: Vec<_> = rows
        .iter()
        .filter(|r| r.1 == "SigningCertificateOpenFailed")
        .collect();
    assert_eq!(
        failed
            .iter()
            .map(|r| (r.2.as_str(), r.3.as_str()))
            .collect::<Vec<_>>(),
        [("USER", "INFO"), ("SYSTEM", "ERROR")]
    );
    let client = w.pool.get().await.unwrap();
    let file_name: String = client
        .query_one(
            "SELECT body->'details'->>'file_name' FROM sequent_backend.signing_log_outbox
             WHERE election_event_id = $1 AND statement_kind = 'SigningCertificateOpenFailed' LIMIT 1",
            &[&w.event],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(file_name, "jose.p12");
    assert_eq!(
        w.steps().await,
        [
            "SigningRequestCreated",
            "SigningSignatureRefused",
            "SigningSignatureRefused",
            "SigningHandover",
            "SigningCertificateOpenFailed",
            "SigningSignatureRefused",
            "SigningSignatureRefused",
            "SigningRequestCancelled"
        ]
    );
    // Each forbidden step is logged as a refusal of that step.
    let refused: Vec<(Option<String>, String, String)> = w
        .entries("SigningSignatureRefused")
        .await
        .into_iter()
        .map(|(user, details)| {
            (
                user,
                details["step"].as_str().unwrap().to_string(),
                details["check"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    let entry = |user: &str, step: &str| {
        (
            Some(user.to_string()),
            step.to_string(),
            "forbidden".to_string(),
        )
    };
    assert_eq!(
        refused,
        [
            entry("outsider", "handover"),
            entry("stranger", "open-failure"),
            entry("jose", "cancel"),
            entry("ofov", "cancel"),
        ]
    );
    // An operator's cancel names the permission that allowed it.
    let cancelled = w.entries("SigningRequestCancelled").await;
    assert_eq!(cancelled[0].0.as_deref(), Some("ofov"));
    assert_eq!(
        cancelled[0].1["allowed_by"],
        json!(["signing-requests-cancel"])
    );
    assert_eq!(cancelled[0].1["voided_approvals"], json!([]));
    w.assert_two_entries_per_step().await;
}

#[tokio::test]
async fn the_export_holds_the_requests_of_the_readers_posts_and_logs_its_hash() {
    let w = world("madrid-pe").await;
    w.rule(ACTION, 2, RequesterSigning::Allowed, None).await;
    // A second Post with another label and a request of each.
    let client = w.pool.get().await.unwrap();
    let tokyo = Uuid::new_v4();
    client
        .execute(
            "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id, presentation, permission_label)
             VALUES ($1, $2, $3, $4, 'tokyo-pe')",
            &[&tokyo, &w.tenant, &w.event, &post_presentation("Post tokyo")],
        )
        .await
        .unwrap();
    let maria = caller("maria", &[], &[]);
    let madrid_request = summary(
        w.guard_request(&maria, &returns(&w, "er", None), at(0))
            .await
            .unwrap(),
    );
    let mut in_tokyo = returns(&w, "er-tokyo", None);
    in_tokyo.scope.election_id = Some(tokyo);
    let tokyo_request = summary(w.guard_request(&maria, &in_tokyo, at(0)).await.unwrap());
    // A Post without a label is everybody's.
    let open = Uuid::new_v4();
    client
        .execute(
            "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id, presentation)
             VALUES ($1, $2, $3, $4)",
            &[&open, &w.tenant, &w.event, &post_presentation("Post open")],
        )
        .await
        .unwrap();
    let mut in_open = returns(&w, "er-open", None);
    in_open.scope.election_id = Some(open);
    let open_request = summary(w.guard_request(&maria, &in_open, at(0)).await.unwrap());

    for (labels, expected) in [
        (vec!["madrid-pe"], vec![madrid_request.id, open_request.id]),
        (vec!["tokyo-pe"], vec![tokyo_request.id, open_request.id]),
        (
            vec!["madrid-pe", "tokyo-pe"],
            vec![madrid_request.id, tokyo_request.id, open_request.id],
        ),
        // Without labels, a reader sees no labelled Post, as in Hasura.
        (vec![], vec![open_request.id]),
    ] {
        let auditor = caller("auditor", &[Permissions::SIGNING_REQUESTS_EXPORT], &labels);
        let mut client = w.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let export = export_requests(
            &tx,
            &auditor,
            w.tenant,
            w.event,
            &ExportFilter::default(),
            at(10),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        let mut lines = export.content.lines();
        assert!(lines
            .next()
            .unwrap()
            .starts_with("request_id,action,election_id"));
        let ids: Vec<Uuid> = lines
            .map(|line| Uuid::parse_str(line.split(',').next().unwrap()).unwrap())
            .collect();
        let mut sorted_expected = expected.clone();
        sorted_expected.sort();
        let mut sorted_ids = ids.clone();
        sorted_ids.sort();
        assert_eq!(sorted_ids, sorted_expected, "{labels:?}");
        assert_eq!(export.rows, expected.len());
        assert_eq!(export.sha256, sha(&export.content));
        let logged: String = client
            .query_one(
                "SELECT body->'details'->>'sha256' FROM sequent_backend.signing_log_outbox
                 WHERE election_event_id = $1 AND statement_kind = 'SigningRequestsExported'
                 ORDER BY id DESC LIMIT 1",
                &[&w.event],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(logged, export.sha256);
        let (_, details) = w.entries("SigningRequestsExported").await.pop().unwrap();
        assert_eq!(details["allowed_by"], json!(["signing-requests-export"]));
    }
    // Filters: the waiting ones, and none cancelled.
    let auditor = caller(
        "auditor",
        &[Permissions::SIGNING_REQUESTS_EXPORT],
        &["madrid-pe", "tokyo-pe"],
    );
    let mut client = w.pool.get().await.unwrap();
    for (status, rows) in [
        (SigningRequestStatus::Waiting, 3),
        (SigningRequestStatus::Cancelled, 0),
    ] {
        let tx = client.transaction().await.unwrap();
        let filter = ExportFilter {
            status: Some(status),
            action: Some(ACTION),
        };
        let export = export_requests(&tx, &auditor, w.tenant, w.event, &filter, at(11))
            .await
            .unwrap();
        tx.commit().await.unwrap();
        assert_eq!(export.rows, rows, "{status}");
    }
    let tx = client.transaction().await.unwrap();
    let other = ExportFilter {
        status: None,
        action: Some(SigningAction::OpenVoting),
    };
    assert_eq!(
        export_requests(&tx, &auditor, w.tenant, w.event, &other, at(12))
            .await
            .unwrap()
            .rows,
        0
    );
    tx.commit().await.unwrap();
    w.assert_two_entries_per_step().await;
}

#[test]
fn a_caller_is_read_from_the_token() {
    let claims: sequent_core::services::jwt::JwtClaims = serde_json::from_value(json!({
        "exp": 0, "iat": 0, "auth_time": 1_800_000_000, "jti": "j", "iss": "i", "sub": "s",
        "typ": "Bearer", "azp": "admin-portal", "acr": "1", "allowed-origins": [],
        "scope": "", "email_verified": true, "preferred_username": "maria.santos",
        "trustee": "trustee-1",
        "https://hasura.io/jwt/claims": {
            "x-hasura-default-role": "admin-user", "x-hasura-tenant-id": "t",
            "x-hasura-user-id": "maria", "x-hasura-allowed-roles": ["sign-close-voting"],
            "x-hasura-permission-labels": "{madrid-pe,\"tokyo-pe\"}"
        }
    }))
    .unwrap();
    let caller = SigningCaller::from_claims(&claims);
    assert_eq!(caller.user_id, "maria");
    assert_eq!(caller.username, "maria.santos");
    assert_eq!(caller.labels, ["madrid-pe", "tokyo-pe"]);
    assert_eq!(caller.trustee.as_deref(), Some("trustee-1"));
    assert_eq!(caller.auth_time.map(|t| t.timestamp()), Some(1_800_000_000));
    assert!(caller.has(SigningAction::CloseVoting.sign_permission()));
    assert!(!caller.has(Permissions::SIGNING_REQUESTS_READ));
    assert!(caller.reaches(Some("tokyo-pe")) && caller.reaches(None));
    assert!(!caller.reaches(Some("faculty-of-law")));
    assert!(caller.signs_for(Some("tokyo-pe")) && caller.signs_for(None));
    assert!(!caller.signs_for(Some("faculty-of-law")));
    // Without labels, a person reaches and signs for unlabelled Posts
    // only, the Posts Hasura shows them.
    let unlabelled = SigningCaller {
        labels: vec![],
        ..caller.clone()
    };
    assert!(!unlabelled.reaches(Some("tokyo-pe")));
    assert!(unlabelled.reaches(None));
    assert!(!unlabelled.signs_for(Some("tokyo-pe")));
    assert!(unlabelled.signs_for(None));
    assert_eq!(caller.actor().username, "maria.santos");
}

/// The tally and the scheduled reports start requests with no token: the
/// system reaches every Post, and holds no permission to sign or read.
#[tokio::test]
async fn the_system_requester_starts_requests_for_any_post() {
    let system = SigningCaller::system("tally");
    assert!(system.reaches(Some("tokyo-pe")) && system.reaches(None));
    assert!(system.roles.is_empty() && system.labels.is_empty());
    assert!(!system.signs_for(Some("tokyo-pe")));
    assert_eq!(system.actor().username, "tally");

    let w = world("madrid-pe").await;
    w.rule(ACTION, 2, RequesterSigning::Allowed, None).await;
    let started = summary(
        w.guard_request(&system, &returns(&w, "er", None), at(0))
            .await
            .unwrap(),
    );
    let row = w.request(started.id).await;
    assert_eq!(row.permission_label.as_deref(), Some("madrid-pe"));
    assert_eq!(row.requested_by, "tally");
    // Its own request, for a labelled Post, is cancelled by its requester.
    let (tenant, id) = (w.tenant, started.id);
    let cancelled = in_tx(&w, |tx| {
        Box::pin(async move { cancel(tx, &system, tenant, id, None).await })
    })
    .await
    .unwrap();
    assert_eq!(cancelled.cancel_reason, Some(CancelReason::ByRequester));
}

#[test]
fn details_show_each_subject_field_as_text() {
    let details = windmill::services::signing::requests::subject_details(&json!({
        "channels": ["ONLINE", "KIOSK"], "count": 3, "flag": true, "name": "Madrid", "none": null
    }));
    let shown: Vec<(&str, &str)> = details
        .iter()
        .map(|d| (d.key.as_str(), d.value.as_str()))
        .collect();
    assert_eq!(
        shown,
        [
            ("channels", "ONLINE, KIOSK"),
            ("count", "3"),
            ("flag", "true"),
            ("name", "Madrid"),
            ("none", "")
        ]
    );
    assert!(
        windmill::services::signing::requests::subject_details(&json!("not an object")).is_empty()
    );
}

#[tokio::test]
async fn a_cancel_note_is_capped_and_export_cells_never_read_as_formulas() {
    let w = world("madrid-pe").await;
    w.rule(ACTION, 2, RequesterSigning::Allowed, None).await;
    let mut formula = caller("=cmd|' /C calc'!A0", &[], &[]);
    formula.username = "=HYPERLINK(\"x\")".into();
    let request = summary(
        w.guard_request(&formula, &returns(&w, "er", None), at(0))
            .await
            .unwrap(),
    );
    let id = request.id;
    let tenant = w.tenant;
    let f = formula.clone();
    in_tx(&w, |tx| {
        Box::pin(async move { cancel(tx, &f, tenant, id, Some(&"x".repeat(2000))).await })
    })
    .await
    .unwrap();
    let client = w.pool.get().await.unwrap();
    let note: String = client
        .query_one(
            "SELECT body->'details'->>'note' FROM sequent_backend.signing_log_outbox
             WHERE election_event_id = $1 AND statement_kind = 'SigningRequestCancelled' LIMIT 1",
            &[&w.event],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(note.chars().count(), 500);
    let auditor = caller(
        "auditor",
        &[Permissions::SIGNING_REQUESTS_EXPORT],
        &["madrid-pe"],
    );
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let export = export_requests(
        &tx,
        &auditor,
        w.tenant,
        w.event,
        &ExportFilter::default(),
        at(1),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let row = export.content.lines().nth(1).unwrap();
    assert!(row.contains("\"'=HYPERLINK(\"\"x\"\")\""), "{row}");
    assert!(!row.contains(",=") && !row.contains(",\"="), "{row}");
}
