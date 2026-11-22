// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The default-off `operational-signing-candidate` services of an Initialize
//! voting approval: the scope Harvest's unmounted routes build, the check
//! that the caller's own receipt is applied, and the independent CA
//! assessment of a live device reservation.
//!
//! A scope exists only for a waiting `initialize-voting` request of a tenant
//! that requires hardware, for a caller the request's rule lets sign and
//! whose user id is a UUID. Only the caller's own applied receipt counts.
//! The CA assessment answers only an exact live reservation of the pinned
//! tenant and device profile, from the event's trusted issuers and stored
//! revocation lists.
//!
//! TEST FICTION: there is no hardware here. The officials' keys are software
//! P-256 keys of the test PKI, enrolled under the device profile `yubihsm`
//! by calling the ledger's contract functions as the test database's
//! superuser, which also plays the Initialize broker and the device worker.

#![cfg(feature = "operational-signing-candidate")]
#![recursion_limit = "256"]

#[path = "support/schema.rs"]
mod schema;
#[path = "support/signing.rs"]
mod signing;
#[path = "support/signing_pki.rs"]
mod signing_pki;

use async_trait::async_trait;
use sequent_core::signing::{RequesterSigning, SigningAction, StaffCertificateRegistration};
use sequent_core::types::permissions::Permissions;
use signing::*;
use signing_pki::{Issued, Pki, CRL_URL, ROOT_CRL_URL};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;
use windmill::postgres::signing::{insert_staff_certificate, NewStaffCertificate};
use windmill::services::signing::approve::{approve_operational, NoDocumentSigner, SigningServices};
use windmill::services::signing::certificates::{CertificateIdentity, OpensslCertificateVerifier};
use windmill::services::signing::crl::{refresh_chain_crls, refresh_event_crls, CrlFetcher};
use windmill::services::signing::directory::UserDirectory;
use windmill::services::signing::executors::SigningExecutorRegistry;
use windmill::services::signing::issuers::import_staff_issuers;
use windmill::services::signing::log::Actor;
use windmill::services::signing::operational_approval::OperationalApprovalInput;
use windmill::services::signing::operational_ca::{
    assess_reserved_ca, DeviceCaProbe, PositiveCaAssessment, RequiredCaDevice,
};
use windmill::services::signing::operational_initialize::{
    initialize_signing_applied, initialize_signing_scope, prepare_initialize_signing,
    InitializeSigningScope,
};
use windmill::services::signing::{Allowance, SigningCaller, SigningError, SigningResult};

const ACTION: SigningAction = SigningAction::InitializeVoting;
/// The verified issuer the broker records; nothing connects to it.
const ISSUER: &str = "https://idp.invalid/realms/operational-signing";
const UNAVAILABLE: &str = "Device signing for this request is not available.";

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

/// Promotes the world's tenant to hardware signing: one-way, committed.
async fn promote(w: &World) {
    w.execute(
        "INSERT INTO sequent_backend.operational_signing_required_tenant (tenant_id) VALUES ($1)",
        &[&w.tenant],
    )
    .await;
}

/// An official with an enrolled `yubihsm` key and its attached event
/// certificate.
struct Official {
    caller: SigningCaller,
    signer: &'static Issued,
    identity: CertificateIdentity,
    certificate: Uuid,
}

async fn enroll(w: &World, signer: &'static Issued) -> Official {
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
    let enrollment: Option<Uuid> = tx
        .query_one(
            "SELECT sequent_backend.operational_signing_enroll_official(
                 $1, $2, $3, 1, $4, 'yubihsm', true, $5)",
            &[
                &w.tenant,
                &user,
                &format!("official/{}/{user}", w.tenant),
                &identity.spki_sha256,
                &sha(&format!("device evidence of {user}")),
            ],
        )
        .await
        .unwrap()
        .get(0);
    let attached: bool = tx
        .query_one(
            "SELECT sequent_backend.operational_signing_bind_official_certificate(
                 $1, $2, $3, $4, $5, $6, $7)",
            &[
                &w.tenant,
                &w.event,
                &user,
                &enrollment.expect("the key is enrolled"),
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
    let mut client = w.pool.get().await.unwrap();
    refresh_chain_crls(
        &mut client,
        w.tenant,
        w.event,
        &[signer.pem()],
        &Lists,
        at(0),
    )
    .await
    .unwrap();
    Official {
        caller,
        signer,
        identity,
        certificate: certificate.id,
    }
}

/// A live device reservation of `official`'s challenge for `request`, as
/// the broker leaves it before the device claims it.
struct Reservation {
    challenge: Uuid,
    binding: String,
    fence: Uuid,
}

async fn reserve(w: &World, official: &Official, request: Uuid) -> Reservation {
    let request = w.request(request).await;
    let challenge = Uuid::new_v4();
    let binding = sha(&format!("binding of {challenge}"));
    let nonce = sha(&format!("nonce of {challenge}"));
    let user = &official.caller.user_id;
    let client = w.pool.get().await.unwrap();
    let prepared: bool = client
        .query_one(
            "SELECT sequent_backend.operational_initialize_prepare(
                 $1, $2, $3, $4, $5, 'yubihsm', $6, $7, $8, $9)",
            &[
                &w.tenant,
                &w.event,
                &request.id,
                user,
                &request.payload_sha256,
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
            "SELECT sequent_backend.operational_initialize_take_callback($1, $2, $3, $4, $5, $6)",
            &[&w.tenant, &w.event, &request.id, user, &challenge, &binding],
        )
        .await
        .unwrap()
        .get(0);
    assert!(redeemed, "the broker redeems the callback");
    // The proof's authentication time must be after the challenge's second.
    tokio::time::sleep(Duration::from_millis(1100)).await;
    let fence: Option<Uuid> = client
        .query_one(
            "SELECT sequent_backend.operational_initialize_reserve_own_callback(
                 $1, $2, $3, $4, $5, $6, $7, $8,
                 floor(extract(epoch FROM clock_timestamp()))::bigint,
                 clock_timestamp() + interval '1 minute', 'yubihsm')",
            &[
                &w.tenant,
                &w.event,
                &request.id,
                user,
                &challenge,
                &binding,
                &nonce,
                &ISSUER,
            ],
        )
        .await
        .unwrap()
        .get(0);
    Reservation {
        challenge,
        binding,
        fence: fence.expect("the broker reserves the device use"),
    }
}

/// The device claims `reservation` and settles a signature of the request.
async fn settle(w: &World, official: &Official, request: Uuid, reservation: &Reservation) {
    let row = w.request(request).await;
    let signature = official.signer.sign(row.canonical_payload.as_bytes());
    let user = &official.caller.user_id;
    let client = w.pool.get().await.unwrap();
    let claimed: bool = client
        .query_one(
            "SELECT sequent_backend.operational_initialize_claim_current_device(
                 $1, $2, $3, $4, $5, $6, $7, 'yubihsm',
                 sequent_backend.operational_initialize_ca_version($1, $2, $8, $9))",
            &[
                &w.tenant,
                &w.event,
                &request,
                user,
                &reservation.challenge,
                &reservation.fence,
                &reservation.binding,
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
                &request,
                user,
                &reservation.challenge,
                &reservation.fence,
                &reservation.binding,
                &signature,
            ],
        )
        .await
        .unwrap()
        .get(0);
    assert!(settled, "the device settles its receipt");
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

fn operator() -> SigningCaller {
    caller("operator", &[], &[])
}

async fn scope(
    w: &World,
    caller: &SigningCaller,
    request: Uuid,
) -> SigningResult<InitializeSigningScope> {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    initialize_signing_scope(&tx, caller, w.tenant, request).await
}

async fn prepare(
    w: &World,
    caller: &SigningCaller,
    request: Uuid,
) -> SigningResult<InitializeSigningScope> {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    prepare_initialize_signing(&tx, caller, w.tenant, request).await
}

async fn applied(w: &World, caller: &SigningCaller, request: Uuid) -> bool {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    initialize_signing_applied(&tx, caller, w.tenant, request)
        .await
        .unwrap()
}

/// The refusal message of a scope that must not exist.
fn refused(result: SigningResult<InitializeSigningScope>) -> String {
    match result {
        Err(SigningError::Forbidden(message)) => message,
        Err(other) => panic!("expected a forbidden scope, got {other:?}"),
        Ok(_) => panic!("expected no scope"),
    }
}

async fn assess(
    w: &World,
    pinned_tenant: Uuid,
    profile: RequiredCaDevice,
    probe: &DeviceCaProbe,
) -> Option<PositiveCaAssessment> {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    assess_reserved_ca(&tx, pinned_tenant, profile, probe)
        .await
        .unwrap()
}

fn probe(w: &World, official: &Official, request: Uuid, reservation: &Reservation) -> DeviceCaProbe {
    DeviceCaProbe {
        tenant: w.tenant,
        event: w.event,
        request,
        official: Uuid::parse_str(&official.caller.user_id).unwrap(),
        challenge: reservation.challenge,
        fence: reservation.fence,
    }
}

#[tokio::test]
async fn a_scope_exists_only_for_a_waiting_hardware_initialize_request() {
    let w = world("init-scope").await;
    w.rule(ACTION, 2, RequesterSigning::NotAllowed, Some(60))
        .await;
    w.rule(
        SigningAction::OpenVoting,
        1,
        RequesterSigning::NotAllowed,
        Some(60),
    )
    .await;
    let other = w
        .start(&operator(), SigningAction::OpenVoting, subject(1), at(0))
        .await;
    let request = w.start(&operator(), ACTION, subject(2), at(0)).await;
    let official = w.signer(&Uuid::new_v4().to_string(), ACTION);

    // A legacy tenant signs with P12 certificates, not a device.
    assert_eq!(refused(scope(&w, &official, request.id).await), UNAVAILABLE);
    assert_eq!(refused(prepare(&w, &official, request.id).await), UNAVAILABLE);

    promote(&w).await;
    let row = w.request(request.id).await;
    let scoped = scope(&w, &official, request.id).await.unwrap();
    assert_eq!(
        (
            scoped.tenant(),
            scoped.event(),
            scoped.request(),
            scoped.election(),
            scoped.actor().to_string(),
            scoped.payload_sha256(),
        ),
        (
            w.tenant,
            w.event,
            request.id,
            Some(w.post),
            official.user_id.clone(),
            row.payload_sha256.as_str(),
        )
    );

    // Another action, a caller the rule does not let sign and an official
    // whose id is not a UUID get no scope.
    assert_eq!(refused(scope(&w, &official, other.id).await), UNAVAILABLE);
    let unpermitted = caller(&Uuid::new_v4().to_string(), &[], &[&w.label]);
    assert_eq!(
        refused(scope(&w, &unpermitted, request.id).await),
        UNAVAILABLE
    );
    assert_eq!(
        refused(scope(&w, &w.signer("officer", ACTION), request.id).await),
        UNAVAILABLE
    );

    // A waiting request can receive the official's approval; nothing is
    // applied yet.
    assert_eq!(
        prepare(&w, &official, request.id).await.unwrap().request(),
        request.id
    );
    assert!(!applied(&w, &official, request.id).await);

    // An overdue request keeps its scope but takes no more approvals.
    w.make_overdue(request.id).await;
    assert!(scope(&w, &official, request.id).await.is_ok());
    match prepare(&w, &official, request.id).await {
        Err(SigningError::Closed { .. }) => {}
        Err(other) => panic!("expected a closed request, got {other:?}"),
        Ok(_) => panic!("an overdue request took an approval"),
    }
}

#[tokio::test]
async fn only_the_officials_own_applied_receipt_counts() {
    let w = hardware_world("init-applied", 2).await;
    let pki = Pki::get();
    let first = enroll(&w, &pki.jose).await;
    let second = enroll(&w, &pki.juan_b).await;
    let request = w.start(&operator(), ACTION, subject(3), at(0)).await;

    let reservation = reserve(&w, &first, request.id).await;
    settle(&w, &first, request.id, &reservation).await;
    // A settled receipt is not an approval until the approve step applies it.
    assert!(!applied(&w, &first.caller, request.id).await);

    let services = SigningServices {
        verifier: Arc::new(OpensslCertificateVerifier::with_directory(Arc::new(Names))),
        executors: SigningExecutorRegistry::default(),
        documents: Arc::new(NoDocumentSigner),
        exports: Arc::new(RowOnlyExports),
    };
    let mut client = w.pool.get().await.unwrap();
    approve_operational(
        &mut client,
        &services,
        &first.caller,
        w.tenant,
        &OperationalApprovalInput {
            request_id: request.id,
            approval_challenge: reservation.challenge,
            artifact_challenge: None,
        },
        at(1),
    )
    .await
    .unwrap();

    assert!(applied(&w, &first.caller, request.id).await);
    // The other official's approval is not this one's.
    assert!(!applied(&w, &second.caller, request.id).await);
    assert_eq!(w.approvals(request.id).await.len(), 1);
}

#[tokio::test]
async fn the_ca_assessment_answers_only_an_exact_live_reservation() {
    let w = hardware_world("init-ca", 2).await;
    let pki = Pki::get();
    let official = enroll(&w, &pki.jose).await;
    let request = w.start(&operator(), ACTION, subject(4), at(0)).await;
    let reservation = reserve(&w, &official, request.id).await;
    let exact = probe(&w, &official, request.id, &reservation);

    let assessment = assess(&w, w.tenant, RequiredCaDevice::YubiHsm, &exact)
        .await
        .expect("the live reservation of a current certificate is assessed");
    assert_eq!(assessment.scope, exact);
    assert_eq!(
        assessment.key_id,
        format!("official/{}/{}", w.tenant, official.caller.user_id)
    );
    assert_eq!(
        (assessment.key_version, assessment.certificate_id, assessment.profile),
        (1, official.certificate, "yubihsm")
    );
    assert_eq!(
        (assessment.cert_fingerprint.as_str(), assessment.cert_spki.as_str()),
        (
            official.identity.fingerprint_sha256.as_str(),
            official.identity.spki_sha256.as_str()
        )
    );
    assert!(!assessment.leaf_der_b64.is_empty());
    assert_eq!(assessment.ca_version.len(), 64);
    assert!(assessment.assessed_at < assessment.usable_until);
    // Bounded by the reservation's one-minute proof, not the certificate.
    assert!(assessment.usable_until <= assessment.assessed_at + 61);

    // Another pinned tenant, a nil identifier, another device profile and
    // a reservation nobody holds are not assessed.
    assert!(assess(&w, Uuid::new_v4(), RequiredCaDevice::YubiHsm, &exact)
        .await
        .is_none());
    let nil = DeviceCaProbe {
        fence: Uuid::nil(),
        ..exact.clone()
    };
    assert!(assess(&w, w.tenant, RequiredCaDevice::YubiHsm, &nil)
        .await
        .is_none());
    assert!(
        assess(&w, w.tenant, RequiredCaDevice::DisposableSoftHsm, &exact)
            .await
            .is_none()
    );
    let unknown = DeviceCaProbe {
        challenge: Uuid::new_v4(),
        ..exact.clone()
    };
    assert!(assess(&w, w.tenant, RequiredCaDevice::YubiHsm, &unknown)
        .await
        .is_none());

    // A revoked certificate is not current.
    w.execute(
        "UPDATE sequent_backend.staff_certificate
         SET status = 'revoked', revoked_at = clock_timestamp() WHERE id = $1",
        &[&official.certificate],
    )
    .await;
    assert!(assess(&w, w.tenant, RequiredCaDevice::YubiHsm, &exact)
        .await
        .is_none());
}

#[tokio::test]
async fn a_claimed_reservation_is_not_assessed_again() {
    let w = hardware_world("init-ca-claimed", 2).await;
    let pki = Pki::get();
    let official = enroll(&w, &pki.juan_b).await;
    let request = w.start(&operator(), ACTION, subject(5), at(0)).await;
    let reservation = reserve(&w, &official, request.id).await;
    let exact = probe(&w, &official, request.id, &reservation);
    assert!(assess(&w, w.tenant, RequiredCaDevice::YubiHsm, &exact)
        .await
        .is_some());

    settle(&w, &official, request.id, &reservation).await;
    assert!(assess(&w, w.tenant, RequiredCaDevice::YubiHsm, &exact)
        .await
        .is_none());
}
