// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The trustees' key steps behind their own signature (Gate mode): with
//! the rule off they run as they always did; with it on they wait for a
//! request that names the custody proof's SHA-256, which the trustee signs and
//! then uses exactly once, with the same custody proof. The signature is kept
//! with the ceremony, and the custody proof never reaches a signing table or
//! the log.
//!
//! Each test runs under two configurations (ceremony and trustee names,
//! the trustee's account), and every expectation is derived from the one
//! in use.

#[path = "support/schema.rs"]
mod schema;
#[path = "support/signing.rs"]
mod signing;

use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::signing::{RequesterSigning, SigningAction, SigningRequestStatus};
use serde_json::{json, Value};
use signing::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use uuid::Uuid;
use windmill::domain::trustee_signatures::TrusteeSignatures;
use windmill::services::signing::approve::SigningServices;
use windmill::services::signing::custody_proofs::{
    custody_proof_labels, custody_proof_signature_status, take_custody_proof_step,
    trustee_signatures, CustodyProofCeremony, CustodyProofError, CustodyProofInput,
    CustodyProofKind, CustodyProofOutcome, CustodyProofStep, KeysCeremonyCustodyProof,
    TallyCustodyProof, CUSTODY_PROOF_SIGNATURE_ANNOTATION,
};
use windmill::services::signing::guard::SigningRequestSummary;
use windmill::services::signing::{SigningCaller, SigningError};

/// The custody proof files: distinctive text that must never be stored.
const CUSTODY_PROOF: &str = "PUBLIC-CUSTODY-PROOF-of-the-trustee-7b1f0c";
const OTHER_CUSTODY_PROOF: &str = "PUBLIC-CUSTODY-PROOF-of-someone-else-e93a";

/// A configuration: the ceremony's name and the trustee's.
struct Config {
    label: &'static str,
    ceremony_name: &'static str,
    trustee_name: &'static str,
    other_trustee_name: &'static str,
}

const CONFIGS: [Config; 2] = [
    Config {
        label: "madrid-pe",
        ceremony_name: "Madrid PE keys",
        trustee_name: "trustee-madrid-2",
        other_trustee_name: "trustee-madrid-3",
    },
    Config {
        label: "faculty-of-science",
        ceremony_name: "Student council keys",
        trustee_name: "returning-officer-1",
        other_trustee_name: "returning-officer-2",
    },
];

/// The ceremony side: the stored custody proof, how often the step ran and
/// which trustees counted when it last ran or matched.
struct FakeCeremony {
    stored: Mutex<String>,
    runs: AtomicUsize,
    signatures: Mutex<Option<TrusteeSignatures>>,
}

impl FakeCeremony {
    fn new() -> Self {
        FakeCeremony {
            stored: Mutex::new(CUSTODY_PROOF.into()),
            runs: AtomicUsize::new(0),
            signatures: Mutex::new(None),
        }
    }

    fn signatures(&self) -> Option<TrusteeSignatures> {
        self.signatures.lock().unwrap().clone()
    }

    fn runs(&self) -> usize {
        self.runs.load(Ordering::SeqCst)
    }

    fn is_stored(&self, custody_proof: &str) -> bool {
        *self.stored.lock().unwrap() == custody_proof
    }
}

#[async_trait]
impl CustodyProofCeremony for FakeCeremony {
    async fn matches(
        &self,
        _: &Transaction<'_>,
        custody_proof: &str,
        signatures: &TrusteeSignatures,
    ) -> anyhow::Result<bool> {
        *self.signatures.lock().unwrap() = Some(signatures.clone());
        Ok(self.is_stored(custody_proof))
    }

    async fn run(
        &self,
        _: &Transaction<'_>,
        custody_proof: &str,
        signatures: &TrusteeSignatures,
    ) -> anyhow::Result<bool> {
        *self.signatures.lock().unwrap() = Some(signatures.clone());
        if !self.is_stored(custody_proof) {
            return Ok(false);
        }
        self.runs.fetch_add(1, Ordering::SeqCst);
        Ok(true)
    }
}

/// An event with a named keys ceremony, a tally session of it and two
/// trustees, each with their own account and certificate.
struct Ceremonies {
    w: World,
    config: &'static Config,
    keys_ceremony_id: Uuid,
    tally_session_id: Uuid,
    trustee_id: Uuid,
    trustee: SigningCaller,
    other: SigningCaller,
    services: SigningServices,
    certificate: TestCert,
    other_certificate: TestCert,
}

async fn ceremonies(config: &'static Config) -> Ceremonies {
    let w = world(config.label).await;
    let keys_ceremony_id = Uuid::new_v4();
    let tally_session_id = Uuid::new_v4();
    let trustee_id = Uuid::new_v4();
    let client = w.pool.get().await.unwrap();
    for (id, name) in [
        (trustee_id, config.trustee_name),
        (Uuid::new_v4(), config.other_trustee_name),
    ] {
        client
            .execute(
                "INSERT INTO sequent_backend.trustee (id, name, tenant_id) VALUES ($1, $2, $3)",
                &[&id, &name, &w.tenant],
            )
            .await
            .unwrap();
    }
    client
        .execute(
            "INSERT INTO sequent_backend.keys_ceremony
                (id, tenant_id, election_event_id, trustee_ids, threshold, name, execution_status)
             VALUES ($1, $2, $3, '{}', 1, $4, 'STARTED')",
            &[
                &keys_ceremony_id,
                &w.tenant,
                &w.event,
                &config.ceremony_name,
            ],
        )
        .await
        .unwrap();
    client
        .execute(
            "INSERT INTO sequent_backend.tally_session
                (id, tenant_id, election_event_id, keys_ceremony_id, threshold, election_ids,
                 is_execution_completed, execution_status, tally_type, annotations)
             VALUES ($1, $2, $3, $4, 1, $5, false, 'STARTED', 'ELECTORAL_RESULTS',
                     '{\"miru:existing\": \"kept\"}')",
            &[
                &tally_session_id,
                &w.tenant,
                &w.event,
                &keys_ceremony_id,
                &vec![w.post],
            ],
        )
        .await
        .unwrap();
    let roles = [
        SigningAction::ConfirmCustodyProof.sign_permission(),
        SigningAction::ContributeCustodyProof.sign_permission(),
    ];
    let mut trustee = caller(&format!("{}-user", config.trustee_name), &roles, &[]);
    trustee.trustee = Some(config.trustee_name.into());
    let mut other = caller(&format!("{}-user", config.other_trustee_name), &roles, &[]);
    other.trustee = Some(config.other_trustee_name.into());
    let certificate = cert(
        config.trustee_name,
        config.trustee_name,
        config.trustee_name,
    );
    let other_certificate = cert(
        config.other_trustee_name,
        config.other_trustee_name,
        config.other_trustee_name,
    );
    w.register(&trustee.user_id, &certificate, None).await;
    w.register(&other.user_id, &other_certificate, None).await;
    let services = services(
        FakeVerifier::knowing(&[&certificate, &other_certificate]),
        vec![],
    );
    Ceremonies {
        w,
        config,
        keys_ceremony_id,
        tally_session_id,
        trustee_id,
        trustee,
        other,
        services,
        certificate,
        other_certificate,
    }
}

impl Ceremonies {
    fn steps(&self) -> [CustodyProofStep; 2] {
        [
            CustodyProofStep {
                kind: CustodyProofKind::Confirm,
                id: self.keys_ceremony_id,
            },
            CustodyProofStep {
                kind: CustodyProofKind::Contribute,
                id: self.tally_session_id,
            },
        ]
    }

    async fn require(&self, step: CustodyProofStep) {
        // The stored count doesn't matter: the trustee signs their own step.
        self.w
            .rule(step.action(), 2, RequesterSigning::NotAllowed, None)
            .await;
    }

    /// Takes the step as the route does: committed unless refused or not
    /// the trustee's custody proof.
    async fn take(
        &self,
        who: &SigningCaller,
        ceremony: &FakeCeremony,
        step: CustodyProofStep,
        custody_proof: &str,
        signing_request_id: Option<Uuid>,
    ) -> Result<CustodyProofOutcome, CustodyProofError> {
        self.take_hashed(who, ceremony, step, custody_proof, None, signing_request_id)
            .await
    }

    async fn take_hashed(
        &self,
        who: &SigningCaller,
        ceremony: &FakeCeremony,
        step: CustodyProofStep,
        custody_proof: &str,
        custody_proof_sha256: Option<&str>,
        signing_request_id: Option<Uuid>,
    ) -> Result<CustodyProofOutcome, CustodyProofError> {
        let mut client = self.w.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let (tenant_id, election_event_id, target_id) = (
            self.w.tenant.to_string(),
            self.w.event.to_string(),
            step.id.to_string(),
        );
        let outcome = take_custody_proof_step(
            &tx,
            who,
            ceremony,
            &CustodyProofInput {
                tenant_id: &tenant_id,
                election_event_id: &election_event_id,
                kind: step.kind,
                target_id: &target_id,
                custody_proof,
                custody_proof_sha256,
                signing_request_id,
            },
        )
        .await;
        if matches!(
            outcome,
            Ok(CustodyProofOutcome::Done(_) | CustodyProofOutcome::SigningRequired(_))
        ) {
            tx.commit().await.unwrap();
        }
        outcome
    }

    /// Starts the trustee's request for `step`.
    async fn start(
        &self,
        ceremony: &FakeCeremony,
        step: CustodyProofStep,
    ) -> SigningRequestSummary {
        match self
            .take(&self.trustee, ceremony, step, CUSTODY_PROOF, None)
            .await
            .unwrap()
        {
            CustodyProofOutcome::SigningRequired(summary) => summary,
            other => panic!("{step:?}: expected a request, got {other:?}"),
        }
    }

    /// Starts and signs the trustee's request for `step`.
    async fn signed(
        &self,
        ceremony: &FakeCeremony,
        step: CustodyProofStep,
    ) -> SigningRequestSummary {
        let summary = self.start(ceremony, step).await;
        let outcome = self
            .w
            .sign(
                &self.services,
                &self.trustee,
                summary.id,
                &self.certificate,
                at(1),
            )
            .await
            .unwrap();
        assert_eq!(outcome.status, SigningRequestStatus::Completed);
        summary
    }

    /// What the trustee's request for `step` signs: exactly the contract's subject.
    fn subject(&self, step: CustodyProofStep, custody_proof: &str) -> Value {
        let id_field = match step.kind {
            CustodyProofKind::Confirm => "keys_ceremony_id",
            CustodyProofKind::Contribute => "tally_session_id",
        };
        json!({
            id_field: step.id.to_string(),
            "trustee_id": self.trustee_id.to_string(),
            "custody_proof_sha256": sha(custody_proof),
        })
    }

    /// The step's ceremony annotations.
    async fn annotations(&self, step: CustodyProofStep) -> Value {
        let (table, id) = match step.kind {
            CustodyProofKind::Confirm => ("keys_ceremony", step.id),
            CustodyProofKind::Contribute => ("tally_session", step.id),
        };
        let client = self.w.pool.get().await.unwrap();
        client
            .query_one(
                &format!(
                    "SELECT COALESCE(annotations, '{{}}'::jsonb) FROM sequent_backend.{table}
                     WHERE id = $1"
                ),
                &[&id],
            )
            .await
            .unwrap()
            .get(0)
    }

    async fn request_count(&self) -> i64 {
        let client = self.w.pool.get().await.unwrap();
        client
            .query_one(
                "SELECT count(*) FROM sequent_backend.signing_request
                 WHERE tenant_id = $1 AND election_event_id = $2",
                &[&self.w.tenant, &self.w.event],
            )
            .await
            .unwrap()
            .get(0)
    }
}

fn signing_error(result: Result<CustodyProofOutcome, CustodyProofError>) -> SigningError {
    match result {
        Err(CustodyProofError::Signing(error)) => error,
        other => panic!("expected a signing refusal, got {other:?}"),
    }
}

#[tokio::test]
async fn with_the_rule_off_the_steps_run_as_today_and_start_nothing() {
    for config in &CONFIGS {
        let c = ceremonies(config).await;
        for step in c.steps() {
            let ceremony = FakeCeremony::new();
            assert_eq!(
                c.take(&c.trustee, &ceremony, step, CUSTODY_PROOF, None)
                    .await
                    .unwrap(),
                CustodyProofOutcome::Done(None),
                "{}: {step:?}",
                config.label
            );
            assert_eq!(ceremony.runs(), 1);
            assert_eq!(
                c.take(&c.trustee, &ceremony, step, OTHER_CUSTODY_PROOF, None)
                    .await
                    .unwrap(),
                CustodyProofOutcome::Invalid
            );
            assert_eq!(ceremony.runs(), 1);
        }
        assert_eq!(c.request_count().await, 0);
        assert!(c.w.outbox().await.is_empty());
        assert_eq!(c.annotations(c.steps()[0]).await, json!({}));
    }
}

#[tokio::test]
async fn with_the_rule_on_a_step_waits_for_a_request_naming_the_custody_proof_hash() {
    for config in &CONFIGS {
        let c = ceremonies(config).await;
        for step in c.steps() {
            c.require(step).await;
            let ceremony = FakeCeremony::new();
            let summary = c.start(&ceremony, step).await;
            assert_eq!(
                ceremony.runs(),
                0,
                "{}: {step:?} ran unsigned",
                config.label
            );
            assert_eq!(summary.required, 1);

            let request = c.w.request(summary.id).await;
            assert_eq!(request.action, step.action());
            assert_eq!(request.status, SigningRequestStatus::Waiting);
            assert_eq!(request.trustee_id, Some(c.trustee_id));
            assert_eq!(request.subject, c.subject(step, CUSTODY_PROOF));
            assert_eq!(request.requested_by, c.trustee.user_id);

            // Starting again with the same file answers the same request.
            assert_eq!(c.start(&ceremony, step).await.id, summary.id);
            // The browser's hash, in either case, is the uploaded file's.
            assert!(matches!(
                c.take_hashed(
                    &c.trustee,
                    &ceremony,
                    step,
                    CUSTODY_PROOF,
                    Some(&sha(CUSTODY_PROOF).to_uppercase()),
                    None
                )
                .await
                .unwrap(),
                CustodyProofOutcome::SigningRequired(_)
            ));
        }
        assert_eq!(c.request_count().await, 2);
    }
}

#[tokio::test]
async fn a_file_that_is_not_the_trustees_custody_proof_starts_nothing() {
    for config in &CONFIGS {
        let c = ceremonies(config).await;
        for step in c.steps() {
            c.require(step).await;
            let ceremony = FakeCeremony::new();
            assert_eq!(
                c.take(&c.trustee, &ceremony, step, OTHER_CUSTODY_PROOF, None)
                    .await
                    .unwrap(),
                CustodyProofOutcome::Invalid,
                "{}: {step:?}",
                config.label
            );
            // A hash of another file than the one uploaded is refused.
            assert!(matches!(
                signing_error(
                    c.take_hashed(
                        &c.trustee,
                        &ceremony,
                        step,
                        CUSTODY_PROOF,
                        Some(&sha(OTHER_CUSTODY_PROOF)),
                        None
                    )
                    .await
                ),
                SigningError::Invalid { .. }
            ));
            assert_eq!(ceremony.runs(), 0);
        }
        assert_eq!(c.request_count().await, 0);
        assert!(c.w.outbox().await.is_empty());
    }
}

#[tokio::test]
async fn a_request_that_is_not_signed_yet_does_not_open_the_step() {
    for config in &CONFIGS {
        let c = ceremonies(config).await;
        for step in c.steps() {
            c.require(step).await;
            let ceremony = FakeCeremony::new();
            let summary = c.start(&ceremony, step).await;
            // Nobody signed it: it holds no signature of the trustee's.
            assert!(matches!(
                signing_error(
                    c.take(&c.trustee, &ceremony, step, CUSTODY_PROOF, Some(summary.id))
                        .await
                ),
                SigningError::Forbidden(_)
            ));
            assert_eq!(ceremony.runs(), 0, "{}: {step:?}", config.label);
            assert_eq!(
                c.w.request(summary.id).await.status,
                SigningRequestStatus::Waiting
            );
        }
    }
}

#[tokio::test]
async fn a_signed_request_with_the_same_custody_proof_opens_the_step_once_and_is_recorded() {
    for config in &CONFIGS {
        let c = ceremonies(config).await;
        for step in c.steps() {
            c.require(step).await;
            let ceremony = FakeCeremony::new();
            let summary = c.signed(&ceremony, step).await;

            let record = match c
                .take(&c.trustee, &ceremony, step, CUSTODY_PROOF, Some(summary.id))
                .await
                .unwrap()
            {
                CustodyProofOutcome::Done(Some(record)) => record,
                other => panic!("{}: {step:?}: {other:?}", config.label),
            };
            assert_eq!(ceremony.runs(), 1);
            let request = c.w.request(summary.id).await;
            assert_eq!(request.status, SigningRequestStatus::Executed);

            // The ceremony keeps a reference to the signed step, as a JSON
            // string: an index, without the signature or the signer.
            let annotations = c.annotations(step).await;
            let key = format!("{CUSTODY_PROOF_SIGNATURE_ANNOTATION}{}", c.trustee_id);
            let stored: Value =
                serde_json::from_str(annotations[&key].as_str().expect("a string")).unwrap();
            assert_eq!(stored, record);
            let approval = &c.w.approvals(summary.id).await[0];
            assert_eq!(
                record,
                json!({
                    "signing_request_id": summary.id,
                    "payload_sha256": request.payload_sha256,
                    "code": summary.code,
                    "signed_at": approval.signed_at,
                })
            );
            // The trustee counts for the ceremony through the used request.
            assert_eq!(
                ceremony.signatures(),
                Some(TrusteeSignatures::Needed {
                    signed: Default::default(),
                    signing: Some(config.trustee_name.into()),
                })
            );
            if step.kind == CustodyProofKind::Contribute {
                // The tally session's other annotations are kept.
                assert_eq!(annotations["miru:existing"], json!("kept"));
            }

            // Used once: a second step with it is refused and runs nothing.
            assert!(matches!(
                signing_error(
                    c.take(&c.trustee, &ceremony, step, CUSTODY_PROOF, Some(summary.id))
                        .await
                ),
                SigningError::Closed {
                    status: SigningRequestStatus::Executed,
                    ..
                }
            ));
            assert_eq!(ceremony.runs(), 1);
        }
        assert_eq!(
            c.w.steps().await,
            [
                "SigningRequestCreated",
                "SigningRequestSigned",
                "SigningRequestCompleted",
                "SigningActionExecuted",
            ]
            .repeat(2)
        );
        c.w.assert_two_entries_per_step().await;
    }
}

#[tokio::test]
async fn another_custody_proof_cannot_use_the_signed_request() {
    for config in &CONFIGS {
        let c = ceremonies(config).await;
        for step in c.steps() {
            c.require(step).await;
            let ceremony = FakeCeremony::new();
            let summary = c.signed(&ceremony, step).await;
            assert!(matches!(
                signing_error(
                    c.take(
                        &c.trustee,
                        &ceremony,
                        step,
                        OTHER_CUSTODY_PROOF,
                        Some(summary.id)
                    )
                    .await
                ),
                SigningError::Conflict(_)
            ));
            assert_eq!(ceremony.runs(), 0, "{}: {step:?}", config.label);
            assert_eq!(
                c.w.request(summary.id).await.status,
                SigningRequestStatus::Completed
            );
        }
    }
}

#[tokio::test]
async fn a_step_the_ceremony_refuses_leaves_the_request_unused() {
    for config in &CONFIGS {
        let c = ceremonies(config).await;
        for step in c.steps() {
            c.require(step).await;
            let ceremony = FakeCeremony::new();
            let summary = c.signed(&ceremony, step).await;
            // The ceremony no longer takes this custody proof (e.g. its stored
            // copy changed): the step is refused and rolled back.
            *ceremony.stored.lock().unwrap() = OTHER_CUSTODY_PROOF.into();
            assert_eq!(
                c.take(&c.trustee, &ceremony, step, CUSTODY_PROOF, Some(summary.id))
                    .await
                    .unwrap(),
                CustodyProofOutcome::Invalid
            );
            assert_eq!(
                c.w.request(summary.id).await.status,
                SigningRequestStatus::Completed,
                "{}: {step:?}",
                config.label
            );
            assert_eq!(
                c.annotations(step).await.get(format!(
                    "{CUSTODY_PROOF_SIGNATURE_ANNOTATION}{}",
                    c.trustee_id
                )),
                None
            );
            // Once the ceremony takes it again, the request still opens the step.
            *ceremony.stored.lock().unwrap() = CUSTODY_PROOF.into();
            assert!(matches!(
                c.take(&c.trustee, &ceremony, step, CUSTODY_PROOF, Some(summary.id))
                    .await
                    .unwrap(),
                CustodyProofOutcome::Done(Some(_))
            ));
        }
    }
}

#[tokio::test]
async fn another_trustee_can_neither_sign_nor_use_the_request() {
    for config in &CONFIGS {
        let c = ceremonies(config).await;
        for step in c.steps() {
            c.require(step).await;
            let ceremony = FakeCeremony::new();
            let summary = c.start(&ceremony, step).await;
            assert!(matches!(
                c.w.sign(
                    &c.services,
                    &c.other,
                    summary.id,
                    &c.other_certificate,
                    at(1)
                )
                .await,
                Err(SigningError::Forbidden(_))
            ));
            c.w.sign(&c.services, &c.trustee, summary.id, &c.certificate, at(1))
                .await
                .unwrap();
            assert!(matches!(
                signing_error(
                    c.take(&c.other, &ceremony, step, CUSTODY_PROOF, Some(summary.id))
                        .await
                ),
                SigningError::Forbidden(_)
            ));
            // An account that is no trustee at all.
            let mut stranger = c.other.clone();
            stranger.trustee = None;
            assert!(matches!(
                signing_error(
                    c.take(&stranger, &ceremony, step, CUSTODY_PROOF, Some(summary.id))
                        .await
                ),
                SigningError::Forbidden(_)
            ));
            assert_eq!(ceremony.runs(), 0, "{}: {step:?}", config.label);
            assert_eq!(
                c.w.request(summary.id).await.status,
                SigningRequestStatus::Completed
            );
        }
    }
}

#[tokio::test]
async fn the_custody_proof_never_reaches_the_signing_tables_or_the_log() {
    for config in &CONFIGS {
        let c = ceremonies(config).await;
        for step in c.steps() {
            c.require(step).await;
            let ceremony = FakeCeremony::new();
            // A wrong file, a refused hash and the whole flow.
            c.take(&c.trustee, &ceremony, step, OTHER_CUSTODY_PROOF, None)
                .await
                .unwrap();
            let summary = c.signed(&ceremony, step).await;
            let _ = c
                .take(
                    &c.trustee,
                    &ceremony,
                    step,
                    OTHER_CUSTODY_PROOF,
                    Some(summary.id),
                )
                .await;
            c.take(&c.trustee, &ceremony, step, CUSTODY_PROOF, Some(summary.id))
                .await
                .unwrap();
        }
        let client = c.w.pool.get().await.unwrap();
        let mut stored = String::new();
        for table in [
            "signing_request",
            "signing_approval",
            "signing_log_outbox",
            "keys_ceremony",
            "tally_session",
        ] {
            for row in client
                .query(
                    &format!(
                        "SELECT row_to_json(t)::text FROM sequent_backend.{table} t
                         WHERE tenant_id = $1"
                    ),
                    &[&c.w.tenant],
                )
                .await
                .unwrap()
            {
                stored.push_str(&row.get::<_, String>(0));
            }
        }
        assert!(stored.contains(&sha(CUSTODY_PROOF)), "{}", config.label);
        for custody_proof in [CUSTODY_PROOF, OTHER_CUSTODY_PROOF] {
            assert!(
                !stored.contains(custody_proof),
                "{}: {custody_proof}",
                config.label
            );
        }
    }
}

fn trustee_claims(c: &Ceremonies) -> JwtClaims {
    serde_json::from_value(json!({
        "exp": 1, "iat": 0, "jti": "jti", "iss": "iss", "sub": c.trustee.user_id, "typ": "Bearer",
        "azp": "admin-portal", "acr": "1", "allowed-origins": [], "scope": "openid",
        "email_verified": true, "preferred_username": c.trustee.username,
        "trustee": c.config.trustee_name,
        "https://hasura.io/jwt/claims": {
            "x-hasura-default-role": "trustee", "x-hasura-tenant-id": c.w.tenant.to_string(),
            "x-hasura-user-id": c.trustee.user_id, "x-hasura-allowed-roles": ["trustee"]
        }
    }))
    .unwrap()
}

#[tokio::test]
async fn the_ceremony_services_refuse_a_step_their_state_does_not_allow_before_the_board() {
    for config in &CONFIGS {
        let c = ceremonies(config).await;
        let claims = trustee_claims(&c);
        let (tenant, event) = (c.w.tenant.to_string(), c.w.event.to_string());
        let (keys_ceremony_id, tally_session_id) = (
            c.keys_ceremony_id.to_string(),
            c.tally_session_id.to_string(),
        );
        let keys = KeysCeremonyCustodyProof {
            claims: &claims,
            tenant_id: &tenant,
            election_event_id: &event,
            keys_ceremony_id: &keys_ceremony_id,
        };
        let tally = TallyCustodyProof {
            claims: &claims,
            tenant_id: &tenant,
            election_event_id: &event,
            tally_session_id: &tally_session_id,
        };
        let mut client = c.w.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        // The fixture's keys ceremony has not generated its keys, and its
        // tally session has no execution: both are refused as the routes
        // always refused them.
        for (ceremony, message) in [
            (
                &keys as &dyn CustodyProofCeremony,
                "Keys ceremony not in ExecutionStatus::IN_PROCESS",
            ),
            (
                &tally as &dyn CustodyProofCeremony,
                "Can't find tally session or tally session execution",
            ),
        ] {
            let none = TrusteeSignatures::NotNeeded;
            for result in [
                ceremony.matches(&tx, CUSTODY_PROOF, &none).await,
                ceremony.run(&tx, CUSTODY_PROOF, &none).await,
            ] {
                let error = result.unwrap_err();
                assert!(
                    format!("{error:#}").contains(message),
                    "{}: {error:#}",
                    config.label
                );
            }
        }
    }
}

#[tokio::test]
async fn a_signed_request_for_another_custody_proof_is_replaced_so_the_trustee_is_never_stuck() {
    for config in &CONFIGS {
        let c = ceremonies(config).await;
        for step in c.steps() {
            c.require(step).await;
            let ceremony = FakeCeremony::new();
            let stale = c.signed(&ceremony, step).await;
            // The ceremony now holds another custody proof (e.g. a new ceremony
            // copy): the signed request can never be used with it.
            *ceremony.stored.lock().unwrap() = OTHER_CUSTODY_PROOF.into();
            let fresh = match c
                .take(&c.trustee, &ceremony, step, OTHER_CUSTODY_PROOF, None)
                .await
                .unwrap()
            {
                CustodyProofOutcome::SigningRequired(summary) => summary,
                other => panic!("{}: {step:?}: {other:?}", config.label),
            };
            assert_ne!(fresh.id, stale.id);
            let replaced = c.w.request(stale.id).await;
            assert_eq!(replaced.status, SigningRequestStatus::Cancelled);
            assert_eq!(
                replaced.cancel_reason,
                Some(sequent_core::signing::CancelReason::Superseded)
            );
            c.w.sign(&c.services, &c.trustee, fresh.id, &c.certificate, at(2))
                .await
                .unwrap();
            assert!(matches!(
                c.take(
                    &c.trustee,
                    &ceremony,
                    step,
                    OTHER_CUSTODY_PROOF,
                    Some(fresh.id)
                )
                .await
                .unwrap(),
                CustodyProofOutcome::Done(Some(_))
            ));
        }
    }
}

#[tokio::test]
async fn with_the_rule_on_only_used_signed_steps_count_for_the_ceremony() {
    for config in &CONFIGS {
        let c = ceremonies(config).await;
        for step in c.steps() {
            let read = || async {
                let mut client = c.w.pool.get().await.unwrap();
                let tx = client.transaction().await.unwrap();
                trustee_signatures(&tx, c.w.tenant, c.w.event, step, None)
                    .await
                    .unwrap()
            };
            assert_eq!(read().await, TrusteeSignatures::NotNeeded);
            c.require(step).await;
            assert_eq!(
                read().await,
                TrusteeSignatures::Needed {
                    signed: Default::default(),
                    signing: None
                }
            );

            let ceremony = FakeCeremony::new();
            let summary = c.signed(&ceremony, step).await;
            c.take(&c.trustee, &ceremony, step, CUSTODY_PROOF, Some(summary.id))
                .await
                .unwrap();
            let signed_trustee: std::collections::HashSet<String> =
                [config.trustee_name.to_string()].into();
            assert_eq!(
                read().await,
                TrusteeSignatures::Needed {
                    signed: signed_trustee.clone(),
                    signing: None
                },
                "{}: {step:?}",
                config.label
            );

            // The other trustee's step sees the first one's as signed.
            let other = c
                .take(&c.other, &ceremony, step, CUSTODY_PROOF, None)
                .await
                .unwrap();
            assert!(matches!(other, CustodyProofOutcome::SigningRequired(_)));
            assert_eq!(
                ceremony.signatures(),
                Some(TrusteeSignatures::Needed {
                    signed: signed_trustee,
                    signing: None
                })
            );
        }
    }
}

#[tokio::test]
async fn the_panel_names_the_ceremony_and_the_trustee_beside_their_ids() {
    for config in &CONFIGS {
        let c = ceremonies(config).await;
        for step in c.steps() {
            c.require(step).await;
            let ceremony = FakeCeremony::new();
            let summary = c.start(&ceremony, step).await;
            let request = c.w.request(summary.id).await;
            let mut client = c.w.pool.get().await.unwrap();
            let tx = client.transaction().await.unwrap();
            let labels = custody_proof_labels(&tx, &request).await.unwrap();
            assert_eq!(
                labels.ceremony_id,
                Some(step.id),
                "{}: {step:?}",
                config.label
            );
            assert_eq!(labels.ceremony_name.as_deref(), Some(config.ceremony_name));
            assert_eq!(labels.trustee_name.as_deref(), Some(config.trustee_name));
        }
    }
}

#[tokio::test]
async fn without_signatures_ids_that_are_not_uuids_reach_the_step_as_before() {
    let c = ceremonies(&CONFIGS[0]).await;
    let ceremony = FakeCeremony::new();
    let tenant = c.w.tenant.to_string();
    let event = c.w.event.to_string();
    let input = |target_id| CustodyProofInput {
        tenant_id: &tenant,
        election_event_id: &event,
        kind: CustodyProofKind::Confirm,
        target_id,
        custody_proof: CUSTODY_PROOF,
        custody_proof_sha256: None,
        signing_request_id: None,
    };
    let mut client = c.w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    // The ceremony answers about its own id, as it always did.
    assert_eq!(
        take_custody_proof_step(&tx, &c.trustee, &ceremony, &input("not-a-uuid"))
            .await
            .unwrap(),
        CustodyProofOutcome::Done(None)
    );
    drop(tx);
    c.require(c.steps()[0]).await;
    let tx = client.transaction().await.unwrap();
    assert!(matches!(
        signing_error(
            take_custody_proof_step(&tx, &c.trustee, &ceremony, &input("not-a-uuid")).await
        ),
        SigningError::Invalid { .. }
    ));
}

/// Everything a tracing subscriber writes, kept for the test.
#[derive(Clone, Default)]
struct Captured(std::sync::Arc<Mutex<Vec<u8>>>);

impl std::io::Write for Captured {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// The process-wide subscriber is set once, so every test of this binary
/// writes into the capture: a scoped one misses spans whose interest another
/// test's thread cached first.
#[tokio::test]
async fn the_custody_proof_never_reaches_the_tracing_output() {
    let captured = Captured::default();
    let writer = captured.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || writer.clone())
        .with_max_level(tracing::Level::TRACE)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .with_ansi(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("one global subscriber");

    let config = &CONFIGS[0];
    let c = ceremonies(config).await;
    let claims = trustee_claims(&c);
    let tenant = c.w.tenant.to_string();
    let event = c.w.event.to_string();
    let keys_ceremony_id = c.keys_ceremony_id.to_string();
    let tally_session_id = c.tally_session_id.to_string();
    for step in c.steps() {
        c.require(step).await;
        // The whole signed flow, a wrong file and another custody proof.
        let ceremony = FakeCeremony::new();
        c.take(&c.trustee, &ceremony, step, OTHER_CUSTODY_PROOF, None)
            .await
            .unwrap();
        let summary = c.signed(&ceremony, step).await;
        let _ = c
            .take(
                &c.trustee,
                &ceremony,
                step,
                OTHER_CUSTODY_PROOF,
                Some(summary.id),
            )
            .await;
        c.take(&c.trustee, &ceremony, step, CUSTODY_PROOF, Some(summary.id))
            .await
            .unwrap();
    }
    // The ceremony services themselves, up to their refusal.
    let mut client = c.w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let none = TrusteeSignatures::NotNeeded;
    let keys = KeysCeremonyCustodyProof {
        claims: &claims,
        tenant_id: &tenant,
        election_event_id: &event,
        keys_ceremony_id: &keys_ceremony_id,
    };
    let tally = TallyCustodyProof {
        claims: &claims,
        tenant_id: &tenant,
        election_event_id: &event,
        tally_session_id: &tally_session_id,
    };
    for ceremony in [&keys as &dyn CustodyProofCeremony, &tally] {
        let _ = ceremony.matches(&tx, CUSTODY_PROOF, &none).await;
        let _ = ceremony.run(&tx, CUSTODY_PROOF, &none).await;
    }

    let output = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
    // The capture works: the steps' spans are in it.
    assert!(output.contains("take_custody_proof_step"), "{output}");
    assert!(output.contains("confirm_key_custody"), "{output}");
    for custody_proof in [CUSTODY_PROOF, OTHER_CUSTODY_PROOF] {
        assert!(
            !output.contains(custody_proof),
            "{custody_proof} in the tracing output"
        );
    }
}

#[tokio::test]
async fn a_trustee_learns_whether_their_step_needs_a_signature_and_has_one() {
    for config in &CONFIGS {
        let c = ceremonies(config).await;
        for step in c.steps() {
            let status = |who: SigningCaller| {
                let c = &c;
                async move {
                    let mut client = c.w.pool.get().await.unwrap();
                    let tx = client.transaction().await.unwrap();
                    let status = custody_proof_signature_status(
                        &tx,
                        &who,
                        &c.w.tenant.to_string(),
                        &c.w.event.to_string(),
                        step.kind,
                        &step.id.to_string(),
                    )
                    .await
                    .unwrap();
                    (status.signature_needed, status.signed)
                }
            };
            assert_eq!(status(c.trustee.clone()).await, (false, false));
            c.require(step).await;
            assert_eq!(status(c.trustee.clone()).await, (true, false));
            let ceremony = FakeCeremony::new();
            let summary = c.signed(&ceremony, step).await;
            // Signed but not used yet: the step still has to be taken.
            assert_eq!(status(c.trustee.clone()).await, (true, false));
            c.take(&c.trustee, &ceremony, step, CUSTODY_PROOF, Some(summary.id))
                .await
                .unwrap();
            assert_eq!(
                status(c.trustee.clone()).await,
                (true, true),
                "{}: {step:?}",
                config.label
            );
            assert_eq!(status(c.other.clone()).await, (true, false));
        }
    }
}
