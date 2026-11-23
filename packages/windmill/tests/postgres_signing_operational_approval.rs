// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The operational approval of an `initialize-voting` request: for a tenant
//! that requires hardware, an approval is a settled device receipt of the
//! caller's own challenge, of a device recorded as `yubihsm`, and nothing
//! else. A wrong challenge, another official's receipt, a receipt of the
//! test device profile, a legacy P12 signature and a quorum that mixes
//! legacy and hardware signatures are refused; a quorum of hardware
//! receipts completes the request and its action runs. A legacy tenant signs
//! as before.
//!
//! TEST FICTION: there is no hardware here. The officials' keys are software
//! P-256 keys of the test PKI, enrolled under the device profile `yubihsm`
//! by calling the ledger's contract functions directly as the test
//! database's superuser, which also plays the Initialize broker and the
//! device worker (prepare, redeem, reserve, claim, settle). A real
//! installation never enrolls a software key as `yubihsm`; only the ledger
//! and the approval step are under test.
//!
//! Each test commits its own tenant: the approve step owns its transactions,
//! and a tenant's promotion to hardware signing is one-way.

#![recursion_limit = "256"]

#[path = "support/schema.rs"]
mod schema;
#[path = "support/signing.rs"]
mod signing;
#[path = "support/signing_actions.rs"]
mod signing_actions;
#[path = "support/signing_pki.rs"]
mod signing_pki;

use async_trait::async_trait;
use sequent_core::signing::{
    RequesterSigning, SigningAction, SigningRequestStatus, StaffCertificateRegistration,
};
use sequent_core::types::permissions::Permissions;
use signing::*;
use signing_actions::{sign_all, signers, Dispatched, Effect, FakeEffects};
use signing_pki::{Issued, Pki, CRL_URL, ROOT_CRL_URL};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;
use windmill::postgres::signing::{
    insert_staff_certificate, NewStaffCertificate, SigningRequestRow,
};
use windmill::services::signing::actions::{executors, run_dispatched, NoSeal, RunOutcome};
use windmill::services::signing::approve::{
    approve_operational, ApproveOutcome, NoDocumentSigner, SigningServices,
};
use windmill::services::signing::certificates::{CertificateIdentity, OpensslCertificateVerifier};
use windmill::services::signing::crl::{refresh_chain_crls, refresh_event_crls, CrlFetcher};
use windmill::services::signing::directory::UserDirectory;
use windmill::services::signing::executors::SigningExecutorRegistry;
use windmill::services::signing::issuers::import_staff_issuers;
use windmill::services::signing::log::Actor;
use windmill::services::signing::operational_approval::OperationalApprovalInput;
use windmill::services::signing::{Allowance, SigningCaller, SigningError, SigningResult};

const ACTION: SigningAction = SigningAction::InitializeVoting;
/// The verified issuer the broker records; nothing connects to it.
const ISSUER: &str = "https://idp.invalid/realms/operational-signing";
const NO_RECEIPT: &str = "No exact operational signing receipt is available.";
const NOT_THIS_METHOD: &str = "This signing method is not allowed for the tenant.";
const MIXED_QUORUM: &str =
    "An earlier approval did not use the required signing device. Start a new request.";

/// Serves the PKI's revocation lists.
struct Lists;

#[async_trait]
impl CrlFetcher for Lists {
    async fn fetch(&self, url: &str) -> anyhow::Result<Vec<u8>> {
        let pki = Pki::get();
        match url {
            CRL_URL => Ok(pki.individual_crl()),
            ROOT_CRL_URL => Ok(pki.root_crl()),
            _ => anyhow::bail!("connection refused"),
        }
    }
}

struct Names;

#[async_trait]
impl UserDirectory for Names {
    async fn display_names(
        &self,
        _: Uuid,
        user_ids: &[String],
    ) -> anyhow::Result<HashMap<String, String>> {
        Ok(user_ids
            .iter()
            .map(|user| (user.clone(), format!("Display {user}")))
            .collect())
    }
}

/// The event trusts the PKI's issuers and holds their current lists.
async fn trust(w: &World) {
    let pki = Pki::get();
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    import_staff_issuers(
        &tx,
        w.tenant,
        w.event,
        &[pki.root.cert.clone(), pki.individual_ca.cert.clone()],
        &Actor {
            user_id: "officer".into(),
            username: "officer".into(),
        },
        Allowance::Permission(Permissions::SIGNING_ISSUERS_WRITE),
        at(0),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    refresh_event_crls(&mut client, w.tenant, w.event, &Lists, at(0))
        .await
        .unwrap();
}

/// Promotes the world's tenant to hardware signing: one-way, committed. A
/// statement of its own, so the policy's exclusive lock is the first lock
/// its transaction takes.
async fn promote(w: &World) {
    w.execute(
        "INSERT INTO sequent_backend.operational_signing_required_tenant (tenant_id) VALUES ($1)",
        &[&w.tenant],
    )
    .await;
}

/// The approve step's services with the real OpenSSL verifier and the
/// protected actions' executors.
fn hardware_services(dispatched: &Dispatched) -> SigningServices {
    SigningServices {
        verifier: Arc::new(OpensslCertificateVerifier::with_directory(Arc::new(Names))),
        executors: executors(Arc::new(dispatched.clone()))
            .into_iter()
            .fold(SigningExecutorRegistry::default(), |registry, executor| {
                registry.with(executor)
            }),
        documents: Arc::new(NoDocumentSigner),
        exports: Arc::new(RowOnlyExports),
    }
}

/// An official with an enrolled key and its attached event certificate.
struct Official {
    caller: SigningCaller,
    signer: &'static Issued,
    identity: CertificateIdentity,
    /// The device profile the key was enrolled under.
    profile: &'static str,
}

/// Registers `signer`'s certificate to a new official of the world's Post,
/// enrolls its key under `profile` and attaches the certificate: what the
/// Security Officer's setup and the key-generation device do together.
async fn enroll(w: &World, signer: &'static Issued, profile: &'static str) -> Official {
    // The ledger takes an official's Keycloak id: a UUID.
    let user = Uuid::new_v4().to_string();
    let caller = w.signer(&user, ACTION);
    let identity = CertificateIdentity::of(&signer.cert, &[]).unwrap();
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let certificate = insert_staff_certificate(
        &tx,
        &NewStaffCertificate {
            tenant_id: w.tenant,
            election_event_id: w.event,
            user_id: user.clone(),
            username: caller.username.clone(),
            election_id: Some(w.post),
            fingerprint_sha256: identity.fingerprint_sha256.clone(),
            spki_sha256: identity.spki_sha256.clone(),
            holder_sha256: identity.holder_sha256.clone(),
            serial: identity.serial.clone(),
            subject: identity.subject.clone(),
            issuer: identity.issuer.clone(),
            not_before: identity.not_before,
            not_after: identity.not_after,
            pem: identity.pem.clone(),
            registration: StaffCertificateRegistration::SecurityOfficer,
            linked_to: None,
            registered_by: "officer".into(),
            user_display_name: None,
            registered_by_name: None,
        },
    )
    .await
    .unwrap();
    let key_id = format!("official/{}/{user}", w.tenant);
    let enrollment: Option<Uuid> = tx
        .query_one(
            "SELECT sequent_backend.operational_signing_enroll_official(
                 $1, $2, $3, 1, $4, $5, true, $6)",
            &[
                &w.tenant,
                &user,
                &key_id,
                &identity.spki_sha256,
                &profile,
                &sha(&format!("device evidence of {user}")),
            ],
        )
        .await
        .unwrap()
        .get(0);
    let enrollment = enrollment.expect("the key is enrolled");
    let attached: bool = tx
        .query_one(
            "SELECT sequent_backend.operational_signing_bind_official_certificate(
                 $1, $2, $3, $4, $5, $6, $7)",
            &[
                &w.tenant,
                &w.event,
                &user,
                &enrollment,
                &certificate.id,
                &identity.fingerprint_sha256,
                &vec![identity.pem.clone()],
            ],
        )
        .await
        .unwrap()
        .get(0);
    assert!(attached, "the certificate is attached to the enrollment");
    tx.commit().await.unwrap();
    Official {
        caller,
        signer,
        identity,
        profile,
    }
}

/// Takes `official`'s Initialize challenge for `request` through the ledger
/// to a settled receipt of `signature`, as the broker and the device worker
/// do, and answers the challenge. Each step is a transaction of its own.
async fn settled_receipt(
    w: &World,
    official: &Official,
    request: &SigningRequestRow,
    signature: &[u8],
) -> Uuid {
    let challenge = Uuid::new_v4();
    let binding = sha(&format!("binding of {challenge}"));
    let nonce = sha(&format!("nonce of {challenge}"));
    let user = &official.caller.user_id;
    let client = w.pool.get().await.unwrap();
    let prepared: bool = client
        .query_one(
            "SELECT sequent_backend.operational_initialize_prepare(
                 $1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            &[
                &w.tenant,
                &w.event,
                &request.id,
                user,
                &request.payload_sha256,
                &official.profile,
                &challenge,
                &ISSUER,
                &nonce,
                &binding,
            ],
        )
        .await
        .unwrap()
        .get(0);
    assert!(prepared, "the broker prepares the challenge");
    let redeemed: bool = client
        .query_one(
            "SELECT sequent_backend.operational_initialize_take_callback(
                 $1, $2, $3, $4, $5, $6)",
            &[&w.tenant, &w.event, &request.id, user, &challenge, &binding],
        )
        .await
        .unwrap()
        .get(0);
    assert!(redeemed, "the broker redeems the callback");
    // The proof's authentication time has second resolution and must be
    // after the second the challenge was issued in.
    tokio::time::sleep(Duration::from_millis(1100)).await;
    let fence: Option<Uuid> = client
        .query_one(
            "SELECT sequent_backend.operational_initialize_reserve_own_callback(
                 $1, $2, $3, $4, $5, $6, $7, $8,
                 floor(extract(epoch FROM clock_timestamp()))::bigint,
                 clock_timestamp() + interval '1 minute', $9)",
            &[
                &w.tenant,
                &w.event,
                &request.id,
                user,
                &challenge,
                &binding,
                &nonce,
                &ISSUER,
                &official.profile,
            ],
        )
        .await
        .unwrap()
        .get(0);
    let fence = fence.expect("the broker reserves the device use");
    let claimed: bool = client
        .query_one(
            "SELECT sequent_backend.operational_initialize_claim_current_device(
                 $1, $2, $3, $4, $5, $6, $7, $8,
                 sequent_backend.operational_initialize_ca_version($1, $2, $9, $10))",
            &[
                &w.tenant,
                &w.event,
                &request.id,
                user,
                &challenge,
                &fence,
                &binding,
                &official.profile,
                &official.identity.fingerprint_sha256,
                &official.identity.spki_sha256,
            ],
        )
        .await
        .unwrap()
        .get(0);
    assert!(claimed, "the device claims its reservation");
    let settled: bool = client
        .query_one(
            "SELECT sequent_backend.operational_signing_settle_device_from_reference(
                 $1, $2, $3, $4, $5, $6, $7, $8)",
            &[
                &w.tenant,
                &w.event,
                &request.id,
                user,
                &challenge,
                &fence,
                &binding,
                &signature,
            ],
        )
        .await
        .unwrap()
        .get(0);
    assert!(settled, "the device settles its receipt");
    challenge
}

/// `official` signs the request's canonical payload with its key and the
/// ledger keeps the receipt.
async fn device_receipt(w: &World, official: &Official, request_id: Uuid) -> Uuid {
    let request = w.request(request_id).await;
    let signature = official.signer.sign(request.canonical_payload.as_bytes());
    settled_receipt(w, official, &request, &signature).await
}

/// The approve step's operational ingress, as the private route calls it.
async fn approve_receipt(
    w: &World,
    services: &SigningServices,
    caller: &SigningCaller,
    request_id: Uuid,
    approval_challenge: Uuid,
    artifact_challenge: Option<Uuid>,
) -> SigningResult<ApproveOutcome> {
    let mut client = w.pool.get().await.unwrap();
    approve_operational(
        &mut client,
        services,
        caller,
        w.tenant,
        &OperationalApprovalInput {
            request_id,
            approval_challenge,
            artifact_challenge,
        },
        at(1),
    )
    .await
}

/// Downloads the lists of `official`'s chain, as the approve route does.
async fn refresh_lists(w: &World, official: &Official) {
    let mut client = w.pool.get().await.unwrap();
    refresh_chain_crls(
        &mut client,
        w.tenant,
        w.event,
        &[official.signer.pem()],
        &Lists,
        at(1),
    )
    .await
    .unwrap();
}

fn forbidden(result: SigningResult<ApproveOutcome>) -> String {
    match result {
        Err(SigningError::Forbidden(message)) => message,
        other => panic!("expected a forbidden step, got {other:?}"),
    }
}

/// How many of the request's ledger intents are in `phase`.
async fn intents(w: &World, request_id: Uuid, phase: &str) -> i64 {
    w.pool
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT count(*) FROM sequent_backend.operational_signing_intent
             WHERE tenant_id = $1 AND event_id = $2 AND request_id = $3 AND phase = $4",
            &[&w.tenant, &w.event, &request_id, &phase],
        )
        .await
        .unwrap()
        .get(0)
}

/// A world that trusts the test PKI, with an initialize-voting rule of
/// `signatures`, whose tenant requires hardware signing.
async fn hardware_world(label: &str, signatures: u16) -> World {
    let w = world(label).await;
    w.rule(ACTION, signatures, RequesterSigning::NotAllowed, Some(60))
        .await;
    trust(&w).await;
    promote(&w).await;
    w
}

#[tokio::test]
async fn two_device_receipts_complete_the_request_and_its_action_runs() {
    let w = hardware_world("init-hardware", 2).await;
    let pki = Pki::get();
    let dispatched = Dispatched::default();
    let services = hardware_services(&dispatched);
    let first = enroll(&w, &pki.jose, "yubihsm").await;
    let second = enroll(&w, &pki.juan_b, "yubihsm").await;
    let request = w
        .start(&caller("operator", &[], &[]), ACTION, subject(1), at(0))
        .await;
    assert_eq!(request.required, 2);

    let challenge = device_receipt(&w, &first, request.id).await;
    assert_eq!(intents(&w, request.id, "completed").await, 1);
    refresh_lists(&w, &first).await;

    // A challenge nobody settled, the receipt of a challenge that is
    // someone else's, and a document receipt for an action without a
    // document: none is this official's exact receipt.
    assert_eq!(
        forbidden(
            approve_receipt(
                &w,
                &services,
                &first.caller,
                request.id,
                Uuid::new_v4(),
                None
            )
            .await
        ),
        NO_RECEIPT
    );
    assert_eq!(
        forbidden(
            approve_receipt(&w, &services, &second.caller, request.id, challenge, None).await
        ),
        NO_RECEIPT
    );
    assert_eq!(
        forbidden(
            approve_receipt(
                &w,
                &services,
                &first.caller,
                request.id,
                challenge,
                Some(Uuid::new_v4())
            )
            .await
        ),
        NO_RECEIPT
    );
    assert!(w.approvals(request.id).await.is_empty());
    assert_eq!(intents(&w, request.id, "completed").await, 1);

    // The receipt itself is the approval.
    let outcome = approve_receipt(&w, &services, &first.caller, request.id, challenge, None)
        .await
        .unwrap();
    assert_eq!(
        (outcome.count, outcome.required, outcome.status),
        (1, 2, SigningRequestStatus::Waiting)
    );
    assert_eq!(intents(&w, request.id, "applied").await, 1);
    let approvals = w.approvals(request.id).await;
    assert_eq!(approvals.len(), 1);
    assert_eq!(approvals[0].user_id, first.caller.user_id);
    assert_eq!(
        approvals[0].fingerprint_sha256,
        first.identity.fingerprint_sha256
    );
    assert!(dispatched.tasks().is_empty());

    // The second receipt is counted only with the first one's evidence, and
    // completes the request.
    let challenge = device_receipt(&w, &second, request.id).await;
    refresh_lists(&w, &second).await;
    let outcome = approve_receipt(&w, &services, &second.caller, request.id, challenge, None)
        .await
        .unwrap();
    assert_eq!(
        (outcome.count, outcome.status),
        (2, SigningRequestStatus::Completed)
    );
    assert_eq!(intents(&w, request.id, "applied").await, 2);
    let tasks = dispatched.tasks();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].request_id, request.id);

    // The dispatched action finds an applied hardware quorum and runs.
    let effects = FakeEffects::new(Effect::Succeed);
    let mut client = w.pool.get().await.unwrap();
    let run = run_dispatched(&mut client, &effects, &NoSeal, &tasks[0])
        .await
        .unwrap();
    assert!(matches!(run, RunOutcome::Executed(_)), "{run:?}");
    assert_eq!(effects.runs(), 1);
    assert_eq!(
        w.request(request.id).await.status,
        SigningRequestStatus::Executed
    );
    let steps = w.steps().await;
    assert_eq!(
        steps
            .iter()
            .filter(|kind| *kind == "SigningRequestSigned")
            .count(),
        2
    );
}

#[tokio::test]
async fn a_receipt_of_the_test_device_profile_is_not_an_approval() {
    let w = hardware_world("init-softhsm", 1).await;
    let pki = Pki::get();
    let dispatched = Dispatched::default();
    let services = hardware_services(&dispatched);
    let official = enroll(&w, &pki.jose, "softhsm-test").await;
    let request = w
        .start(&caller("operator", &[], &[]), ACTION, subject(2), at(0))
        .await;

    // The ledger settles the receipt under the profile it was enrolled as.
    let challenge = device_receipt(&w, &official, request.id).await;
    assert_eq!(intents(&w, request.id, "completed").await, 1);
    refresh_lists(&w, &official).await;
    assert_eq!(
        forbidden(
            approve_receipt(&w, &services, &official.caller, request.id, challenge, None).await
        ),
        NO_RECEIPT
    );
    assert!(w.approvals(request.id).await.is_empty());
    assert_eq!(intents(&w, request.id, "applied").await, 0);
    assert_eq!(
        w.request(request.id).await.status,
        SigningRequestStatus::Waiting
    );
    assert!(dispatched.tasks().is_empty());
}

#[tokio::test]
async fn a_legacy_signature_and_a_hardware_receipt_never_share_a_quorum() {
    let w = world("init-mixed").await;
    w.rule(ACTION, 2, RequesterSigning::NotAllowed, Some(60))
        .await;
    let dispatched = Dispatched::default();
    let legacy = signers(&w, ACTION, 2, "sbei").await;
    let services = services(
        FakeVerifier::knowing(&legacy.iter().map(|(_, c)| c).collect::<Vec<_>>()),
        executors(Arc::new(dispatched.clone())),
    );
    let request = w
        .start(&caller("operator", &[], &[]), ACTION, subject(3), at(0))
        .await;
    let official = w.signer(&Uuid::new_v4().to_string(), ACTION);

    // A legacy tenant has no operational ingress, and signs as before.
    assert_eq!(
        forbidden(
            approve_receipt(&w, &services, &official, request.id, Uuid::new_v4(), None).await
        ),
        NOT_THIS_METHOD
    );
    let outcome = w
        .sign(&services, &legacy[0].0, request.id, &legacy[0].1, at(1))
        .await
        .unwrap();
    assert_eq!(
        (outcome.count, outcome.status),
        (1, SigningRequestStatus::Waiting)
    );

    // Once the tenant requires hardware, a P12 signature is not taken.
    promote(&w).await;
    assert_eq!(
        forbidden(
            w.sign(&services, &legacy[1].0, request.id, &legacy[1].1, at(1))
                .await
        ),
        NOT_THIS_METHOD
    );
    // And the request's earlier legacy signature can't be part of a
    // hardware quorum: the request has to be started again.
    match approve_receipt(&w, &services, &official, request.id, Uuid::new_v4(), None).await {
        Err(SigningError::Conflict(message)) => assert_eq!(message, MIXED_QUORUM),
        other => panic!("expected a conflict, got {other:?}"),
    }
    assert_eq!(w.approvals(request.id).await.len(), 1);
    assert_eq!(
        w.request(request.id).await.status,
        SigningRequestStatus::Waiting
    );
    assert!(dispatched.tasks().is_empty());
}

#[tokio::test]
async fn a_request_completed_with_legacy_signatures_does_not_run_after_promotion() {
    let w = world("init-late").await;
    w.rule(ACTION, 1, RequesterSigning::NotAllowed, Some(60))
        .await;
    let request = w
        .start(&caller("operator", &[], &[]), ACTION, subject(4), at(0))
        .await;
    let dispatched = Dispatched::default();
    sign_all(&w, ACTION, request.id, 1, "sbei", &dispatched).await;
    let tasks = dispatched.tasks();
    assert_eq!(tasks.len(), 1);
    assert_eq!(
        w.request(request.id).await.status,
        SigningRequestStatus::Completed
    );

    // The tenant is promoted before the dispatched action runs.
    promote(&w).await;
    let effects = FakeEffects::new(Effect::Succeed);
    let mut client = w.pool.get().await.unwrap();
    let run = run_dispatched(&mut client, &effects, &NoSeal, &tasks[0])
        .await
        .unwrap();
    assert_eq!(
        run,
        RunOutcome::Failed("hardware-signatures-missing".into())
    );
    assert_eq!(effects.runs(), 0);
    assert_eq!(
        w.request(request.id).await.status,
        SigningRequestStatus::Failed
    );
}

#[tokio::test]
async fn a_hardware_tenant_starts_nothing_without_a_required_rule() {
    let w = world("init-no-rule").await;
    promote(&w).await;
    // No rule is configured: a legacy tenant would proceed unsigned.
    let outcome = w
        .guard(
            &caller("operator", &[], &[]),
            SigningAction::CloseVoting,
            subject(5),
            at(0),
        )
        .await;
    match outcome {
        Err(SigningError::Internal(error)) => assert!(
            format!("{error:#}").contains("Hardware signing needs a required rule"),
            "{error:#}"
        ),
        other => panic!("expected the missing rule to stop the action, got {other:?}"),
    }
}
