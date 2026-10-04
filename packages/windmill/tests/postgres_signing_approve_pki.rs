// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The approve step with the real OpenSSL verifier and a generated PKI:
//! RSA and EC signatures complete a request and run its action once; a
//! revoked certificate, a key registered to someone else, a certificate
//! bound to another Post and a document signature on an action without a
//! document are refused.

#[path = "support/schema.rs"]
mod schema;
#[path = "support/signing.rs"]
mod signing;
#[path = "support/signing_pki.rs"]
mod signing_pki;

use async_trait::async_trait;
use sequent_core::signing::{
    CertificateCheckId, RequesterSigning, SigningAction, SigningRequestStatus,
    StaffCertificateRegistration,
};
use sequent_core::types::permissions::Permissions;
use serde_json::json;
use signing::*;
use signing_pki::{Issued, Pki, CRL_URL, ROOT_CRL_URL};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use windmill::postgres::signing::{insert_staff_certificate, NewStaffCertificate};
use windmill::services::signing::approve::{
    approve, ApproveInput, ApproveOutcome, NoDocumentSigner, SigningServices,
};
use windmill::services::signing::certificates::{CertificateIdentity, OpensslCertificateVerifier};
use windmill::services::signing::crl::{refresh_chain_crls, refresh_event_crls, CrlFetcher};
use windmill::services::signing::directory::UserDirectory;
use windmill::services::signing::executors::SigningExecutorRegistry;
use windmill::services::signing::issuers::import_staff_issuers;
use windmill::services::signing::log::Actor;
use windmill::services::signing::{
    Allowance, InvalidReason, SigningCaller, SigningError, SigningResult,
};

const ACTION: SigningAction = SigningAction::CloseVoting;

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
    let actor = Actor {
        user_id: "officer".into(),
        username: "officer".into(),
    };
    import_staff_issuers(
        &tx,
        w.tenant,
        w.event,
        &[pki.root.cert.clone(), pki.individual_ca.cert.clone()],
        &actor,
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

fn real_services(executor: Arc<FakeExecutor>) -> SigningServices {
    SigningServices {
        verifier: Arc::new(OpensslCertificateVerifier::with_directory(Arc::new(Names))),
        executors: SigningExecutorRegistry::default().with(executor),
        documents: Arc::new(NoDocumentSigner),
        exports: Arc::new(RowOnlyExports),
    }
}

/// `caller` signs request `id` with `signer`'s key, as the browser does.
async fn sign(
    w: &World,
    services: &SigningServices,
    caller: &SigningCaller,
    id: Uuid,
    signer: &Issued,
    document_signature: Option<Vec<u8>>,
) -> SigningResult<ApproveOutcome> {
    let request = w.request(id).await;
    let algorithm = CertificateIdentity::of(&signer.cert, &[])
        .unwrap()
        .key_algorithm
        .unwrap();
    let mut client = w.pool.get().await.unwrap();
    // As the approve route does before approving.
    refresh_chain_crls(
        &mut client,
        w.tenant,
        w.event,
        &[signer.pem()],
        &Lists,
        at(1),
    )
    .await
    .unwrap();
    approve(
        &mut client,
        services,
        caller,
        w.tenant,
        &ApproveInput {
            request_id: id,
            chain_pem: vec![signer.pem()],
            algorithm,
            payload_signature: signer.sign(request.canonical_payload.as_bytes()),
            document_signature,
            pdf_cms: None,
            revision: None,
        },
        at(1),
    )
    .await
}

fn refused(result: SigningResult<ApproveOutcome>) -> CertificateCheckId {
    match result {
        Err(SigningError::Refused { check, .. }) => check,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// A trusted event with a close-voting rule of `signatures`.
async fn setting(signatures: u16) -> World {
    let w = world("madrid-pe").await;
    w.rule(ACTION, signatures, RequesterSigning::NotAllowed, None)
        .await;
    trust(&w).await;
    w
}

#[tokio::test]
async fn rsa_and_ec_signatures_complete_the_request_and_register_on_first_use() {
    let w = setting(2).await;
    let pki = Pki::get();
    let executor = Arc::new(FakeExecutor::new(ACTION, FakeBehaviour::Succeed(json!({}))));
    let services = real_services(executor.clone());
    let request = w
        .start(&caller("operator", &[], &[]), ACTION, subject(1), at(0))
        .await;
    let first = sign(
        &w,
        &services,
        &w.signer("maria", ACTION),
        request.id,
        &pki.maria,
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        (first.count, first.status),
        (1, SigningRequestStatus::Waiting)
    );
    let last = sign(
        &w,
        &services,
        &w.signer("jose", ACTION),
        request.id,
        &pki.jose,
        None,
    )
    .await
    .unwrap();
    assert_eq!(last.status, SigningRequestStatus::Executed);
    assert_eq!(executor.runs(), [(request.id, 2)]);
    let steps = w.steps().await;
    assert_eq!(
        steps
            .iter()
            .filter(|kind| *kind == "SigningCertificateRegistered")
            .count(),
        2
    );
    w.assert_two_entries_per_step().await;
}

#[tokio::test]
async fn a_revoked_certificate_with_an_active_registration_is_refused() {
    let w = setting(2).await;
    let pki = Pki::get();
    // Registered before its issuer revoked it.
    let identity = CertificateIdentity::of(&pki.revoked.cert, &[]).unwrap();
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    insert_staff_certificate(
        &tx,
        &NewStaffCertificate {
            tenant_id: w.tenant,
            election_event_id: w.event,
            user_id: "rita".into(),
            username: "rita".into(),
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
    tx.commit().await.unwrap();
    let services = real_services(Arc::new(FakeExecutor::new(
        ACTION,
        FakeBehaviour::Succeed(json!({})),
    )));
    let request = w
        .start(&caller("operator", &[], &[]), ACTION, subject(2), at(0))
        .await;
    assert_eq!(
        refused(
            sign(
                &w,
                &services,
                &w.signer("rita", ACTION),
                request.id,
                &pki.revoked,
                None
            )
            .await
        ),
        CertificateCheckId::NotRevoked
    );
    assert!(w.approvals(request.id).await.is_empty());
}

#[tokio::test]
async fn a_key_registered_to_someone_else_can_t_register_on_first_use() {
    let w = setting(2).await;
    let pki = Pki::get();
    let services = real_services(Arc::new(FakeExecutor::new(
        ACTION,
        FakeBehaviour::Succeed(json!({})),
    )));
    let request = w
        .start(&caller("operator", &[], &[]), ACTION, subject(3), at(0))
        .await;
    sign(
        &w,
        &services,
        &w.signer("maria", ACTION),
        request.id,
        &pki.maria,
        None,
    )
    .await
    .unwrap();
    // Bob, with a reissue of Maria's key: the refusal names Maria.
    match sign(
        &w,
        &services,
        &w.signer("bob", ACTION),
        request.id,
        &pki.maria_reissue,
        None,
    )
    .await
    {
        Err(SigningError::Refused {
            check: CertificateCheckId::RegisteredToOther,
            other_holder: Some(holder),
            ..
        }) => assert_eq!(holder.user_id, "maria"),
        other => panic!("{other:?}"),
    }
    assert_eq!(w.approvals(request.id).await.len(), 1);
}

#[tokio::test]
async fn a_certificate_bound_to_one_post_can_t_sign_for_another() {
    let w = setting(1).await;
    let pki = Pki::get();
    let services = real_services(Arc::new(FakeExecutor::new(
        ACTION,
        FakeBehaviour::Succeed(json!({})),
    )));
    // Maria signs for both Posts; her key binds to the first.
    let maria = caller(
        "maria",
        &[ACTION.sign_permission()],
        &["madrid-pe", "tokyo-pe"],
    );
    let here = w
        .start(&caller("operator", &[], &[]), ACTION, subject(4), at(0))
        .await;
    sign(&w, &services, &maria, here.id, &pki.maria, None)
        .await
        .unwrap();
    // A second Post of the event.
    let tokyo = Uuid::new_v4();
    w.execute(
        "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id, presentation, permission_label)
         VALUES ($1, $2, $3, $4, 'tokyo-pe')",
        &[&tokyo, &w.tenant, &w.event, &post_presentation("Post tokyo")],
    )
    .await;
    let mut scope = w.scope(ACTION);
    scope.election_id = Some(tokyo);
    let there = w
        .guard_request(
            &caller("operator", &[], &[]),
            &windmill::services::signing::guard::GuardRequest {
                action: ACTION,
                scope,
                subject: subject(4),
                document: None,
                config_revision: None,
            },
            at(0),
        )
        .await
        .unwrap();
    let windmill::services::signing::guard::GuardOutcome::SigningRequired(there) = there else {
        panic!("{there:?}")
    };
    assert_eq!(
        refused(sign(&w, &services, &maria, there.id, &pki.maria, None).await),
        CertificateCheckId::PostBinding
    );
}

#[tokio::test]
async fn a_document_signature_on_an_action_without_a_document_is_refused() {
    let w = setting(2).await;
    let pki = Pki::get();
    let services = real_services(Arc::new(FakeExecutor::new(
        ACTION,
        FakeBehaviour::Succeed(json!({})),
    )));
    let request = w
        .start(&caller("operator", &[], &[]), ACTION, subject(5), at(0))
        .await;
    let result = sign(
        &w,
        &services,
        &w.signer("maria", ACTION),
        request.id,
        &pki.maria,
        Some(pki.maria.sign(b"document")),
    )
    .await;
    assert!(matches!(
        result,
        Err(SigningError::Invalid {
            reason: InvalidReason::Document,
            ..
        })
    ));
    assert!(w.approvals(request.id).await.is_empty());
}
