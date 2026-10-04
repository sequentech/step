// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Staff certificate checks without a database: [`evaluate`] over a
//! generated PKI. Each refusal starts from a passing control and changes one
//! thing; the expectations come from the checks configured, so a hardcoded
//! policy fails.

#[path = "support/signing_pki.rs"]
mod signing_pki;

use chrono::{DateTime, Duration, TimeZone, Utc};
use openssl::hash::MessageDigest;
use sequent_core::signing::{
    CertificateCheckId, CertificatePostBinding, CertificateRegistration, CrlUnavailablePolicy,
    RevocationCheck, RevocationStatus, SignatureAlgorithm, SigningAction, SigningChecks,
    SigningRequestStatus, StaffCertificateRegistration, StaffCertificateStatus,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use signing_pki::{
    crl, crl_number, crl_signed_by, crl_with, delta_indicator, issue_named, issued,
    issuing_distribution_point, person, pki_now, rsa_key, unknown_critical_extension, Idp, Issued,
    Pki, Spec, Usage, CRL_URL, REVOKED_SERIAL, ROOT_CRL_URL,
};
use strum::IntoEnumIterator;
use uuid::Uuid;
use windmill::postgres::signing::{SigningApprovalRow, SigningRequestRow, StaffCertificateRow};
use windmill::services::signing::certificates::{
    evaluate, parse_chain, parse_pem_or_der, verified_path, verify_signature, CertificateIdentity,
    CertificateVerification, CertificateVerificationInput, DocumentSignatureInput,
    RegistrationState, SignatureInputs, StoredCrl, VerificationContext,
};
use windmill::services::signing::log::Actor;

const TENANT: Uuid = Uuid::from_u128(0xa);
const EVENT: Uuid = Uuid::from_u128(0xe);
const POST_A: Uuid = Uuid::from_u128(0x1a);
const POST_B: Uuid = Uuid::from_u128(0x1b);
const PAYLOAD: &str = r#"{"action":"close-voting","domain":"step-signing/v1"}"#;

fn now() -> DateTime<Utc> {
    Utc.timestamp_opt(pki_now(), 0).unwrap()
}

fn sha(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn request(action: SigningAction, election_id: Option<Uuid>) -> SigningRequestRow {
    SigningRequestRow {
        id: Uuid::from_u128(0x77),
        tenant_id: TENANT,
        election_event_id: EVENT,
        action,
        election_id,
        area_id: None,
        trustee_id: None,
        scope_key: "scope".to_owned(),
        subject: json!({}),
        canonical_payload: PAYLOAD.to_owned(),
        payload_sha256: sha(PAYLOAD.as_bytes()),
        document_id: None,
        document_sha256: None,
        code: "7F3A-91C2".to_owned(),
        config_revision: None,
        rule_revision: 1,
        rule_snapshot: json!({}),
        required: 2,
        status: SigningRequestStatus::Waiting,
        cancel_reason: None,
        cancelled_by: None,
        requested_by: "requester".to_owned(),
        requested_by_username: "requester".to_owned(),
        requested_by_name: None,
        expires_at: None,
        completed_at: None,
        executed_at: None,
        execution_result: None,
        task_execution_id: None,
        execution_started_at: None,
        execution_attempts: 0,
        open_failures: 0,
        permission_label: None,
        created_at: now(),
    }
}

fn actor(user: &str) -> Actor {
    Actor {
        user_id: user.to_owned(),
        username: format!("{user}-name"),
    }
}

fn identity(signer: &Issued) -> CertificateIdentity {
    CertificateIdentity::of(&signer.cert, &[]).unwrap()
}

/// An active registration of `signer`'s certificate to `user`.
fn registration(signer: &Issued, user: &str, election_id: Option<Uuid>) -> StaffCertificateRow {
    let identity = identity(signer);
    StaffCertificateRow {
        id: Uuid::new_v4(),
        tenant_id: TENANT,
        election_event_id: EVENT,
        user_id: user.to_owned(),
        username: format!("{user}-name"),
        user_display_name: Some(format!("Display {user}")),
        election_id,
        fingerprint_sha256: identity.fingerprint_sha256,
        spki_sha256: identity.spki_sha256,
        holder_sha256: identity.holder_sha256,
        serial: identity.serial,
        subject: identity.subject,
        issuer: identity.issuer,
        not_before: identity.not_before,
        not_after: identity.not_after,
        pem: identity.pem,
        status: StaffCertificateStatus::Active,
        registration: StaffCertificateRegistration::FirstUse,
        linked_to: None,
        registered_by: user.to_owned(),
        registered_by_name: None,
        registered_at: now() - Duration::days(3),
        revoked_by: None,
        revoked_by_name: None,
        revoked_at: None,
        revoke_reason: None,
        created_at: now(),
    }
}

fn approval(signer: &Issued, user: &str) -> SigningApprovalRow {
    let identity = identity(signer);
    SigningApprovalRow {
        id: Uuid::new_v4(),
        tenant_id: TENANT,
        election_event_id: EVENT,
        request_id: Uuid::from_u128(0x77),
        user_id: user.to_owned(),
        username: format!("{user}-name"),
        auth_time: None,
        certificate_id: Uuid::new_v4(),
        certificate_pem: identity.pem,
        chain_pem: String::new(),
        fingerprint_sha256: identity.fingerprint_sha256,
        spki_sha256: identity.spki_sha256,
        holder_sha256: identity.holder_sha256,
        algorithm: SignatureAlgorithm::RsaPkcs1Sha256,
        payload_signature: vec![1],
        document_signature: None,
        pdf_cms: None,
        revocation_status: RevocationStatus::Checked,
        display_name: None,
        signed_at: now(),
        created_at: now(),
    }
}

fn stored_at(url: &str, der: Vec<u8>) -> StoredCrl {
    StoredCrl {
        url: url.to_owned(),
        ..stored(der)
    }
}

fn stored(der: Vec<u8>) -> StoredCrl {
    StoredCrl {
        url: CRL_URL.to_owned(),
        der: Some(der),
        fetched_at: now() - Duration::hours(1),
    }
}

/// The root and the individual CA trusted, their current lists, the
/// default checks and no registrations.
fn context() -> VerificationContext {
    let pki = Pki::get();
    VerificationContext {
        issuers: vec![pki.root.cert.clone(), pki.individual_ca.cert.clone()],
        crls: vec![
            stored(pki.individual_crl()),
            stored_at(ROOT_CRL_URL, pki.root_crl()),
        ],
        checks: SigningChecks::default(),
        registrations: vec![],
        revoked: vec![],
        approvals: vec![],
        signed_posts: vec![],
    }
}

fn pem(issued: &Issued) -> String {
    issued.pem()
}

fn algorithm_of(signer: &Issued) -> SignatureAlgorithm {
    identity(signer).key_algorithm.unwrap()
}

/// `signer` signs `request` as `user`, sending `chain` after its certificate.
fn signing<'a>(
    request: &'a SigningRequestRow,
    user: &str,
    signer: &Issued,
    chain: &[&Issued],
) -> CertificateVerificationInput<'a> {
    let mut chain_pem = vec![pem(signer)];
    chain_pem.extend(chain.iter().map(|issued| pem(issued)));
    CertificateVerificationInput {
        request,
        signer: actor(user),
        chain_pem,
        signatures: Some(SignatureInputs {
            algorithm: algorithm_of(signer),
            payload_signature: signer.sign(request.canonical_payload.as_bytes()),
            document: None,
        }),
        now: now(),
    }
}

fn run(
    context: &VerificationContext,
    input: &CertificateVerificationInput<'_>,
) -> CertificateVerification {
    evaluate(context, input).unwrap()
}

fn result(verification: &CertificateVerification, id: CertificateCheckId) -> (bool, String) {
    let check = verification
        .check(id)
        .unwrap_or_else(|| panic!("no {id} result: {:?}", verification.checks));
    (check.ok, check.detail.clone().unwrap_or_default())
}

/// The refusal is `id`, and only `id` failed.
fn assert_refused_by(verification: &CertificateVerification, id: CertificateCheckId) {
    let failed: Vec<_> = verification
        .checks
        .iter()
        .filter(|check| !check.ok)
        .map(|check| check.id)
        .collect();
    assert_eq!(failed, vec![id], "{:?}", verification.checks);
    assert!(!verification.passed());
    assert_eq!(verification.refusal().map(|check| check.id), Some(id));
}

/// Refused because the certificate, its key or its holder is another
/// account's: `registered-to-other` names them, and `registered` fails too
/// rather than promising a first-use registration that can't happen.
fn assert_registered_to_other(verification: &CertificateVerification) {
    let failed: Vec<_> = verification
        .checks
        .iter()
        .filter(|check| !check.ok)
        .map(|check| check.id)
        .collect();
    assert_eq!(
        failed,
        vec![
            CertificateCheckId::Registered,
            CertificateCheckId::RegisteredToOther
        ],
        "{:?}",
        verification.checks
    );
    assert!(!verification.passed());
    assert_eq!(
        verification.refusal().map(|check| check.id),
        Some(CertificateCheckId::RegisteredToOther)
    );
}

fn assert_passes(verification: &CertificateVerification) {
    assert!(verification.passed(), "{:?}", verification.checks);
}

// Identities

#[test]
fn identities_hash_the_certificate_the_key_and_the_subject_name() {
    let pki = Pki::get();
    let maria = identity(&pki.maria);
    assert_eq!(
        maria.fingerprint_sha256,
        hex::encode(pki.maria.cert.digest(MessageDigest::sha256()).unwrap())
    );
    assert_eq!(
        maria.spki_sha256,
        sha(&pki
            .maria
            .cert
            .public_key()
            .unwrap()
            .public_key_to_der()
            .unwrap())
    );
    assert_eq!(
        maria.holder_sha256,
        // The canonical subject: OIDs and lowercased values, in RDN order.
        sha(b"/2.5.4.6=ph/2.5.4.10=test staff pki/2.5.4.11=individual/2.5.4.3=maria santos/2.5.4.5=ph-0001")
    );
    // 1001, in whole bytes.
    assert_eq!(maria.serial, "03E9");
    assert_eq!(maria.common_name, "Maria Santos");
    assert_eq!(
        maria.key_algorithm,
        Some(SignatureAlgorithm::RsaPkcs1Sha256)
    );
    assert_eq!(
        identity(&pki.jose).key_algorithm,
        Some(SignatureAlgorithm::EcdsaP256Sha256)
    );
    assert_eq!(identity(&pki.p384).key_algorithm, None);
    for hash in [
        &maria.fingerprint_sha256,
        &maria.spki_sha256,
        &maria.holder_sha256,
    ] {
        assert!(hash.len() == 64 && hash.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')));
    }

    // A reissue keeps the key and the holder; a second certificate of one
    // holder keeps only the holder.
    let reissue = identity(&pki.maria_reissue);
    assert_ne!(reissue.fingerprint_sha256, maria.fingerprint_sha256);
    assert_eq!(reissue.spki_sha256, maria.spki_sha256);
    assert_eq!(reissue.holder_sha256, maria.holder_sha256);
    let (juan_a, juan_b) = (identity(&pki.juan_a), identity(&pki.juan_b));
    assert_ne!(juan_a.spki_sha256, juan_b.spki_sha256);
    assert_eq!(juan_a.holder_sha256, juan_b.holder_sha256);
}

#[test]
fn chains_parse_from_bundles_and_der_and_refuse_garbage() {
    let pki = Pki::get();
    let bundle = format!("{}{}", pem(&pki.maria), pem(&pki.individual_ca));
    let chain = parse_chain(&[bundle, pem(&pki.root)]).unwrap();
    assert_eq!(chain.len(), 3);
    assert_eq!(chain[0].to_der().unwrap(), pki.maria.der());

    assert!(parse_chain(&[]).is_err());
    assert!(parse_chain(&["not a certificate".to_owned()]).is_err());
    let too_many: Vec<String> = (0..11).map(|_| pem(&pki.maria)).collect();
    assert!(parse_chain(&too_many).is_err());

    use base64::Engine;
    let der_b64 = base64::engine::general_purpose::STANDARD.encode(pki.root.der());
    let parsed = parse_pem_or_der(None, Some(&der_b64)).unwrap();
    assert_eq!(parsed[0].to_der().unwrap(), pki.root.der());
    assert_eq!(
        parse_pem_or_der(Some(&pem(&pki.root)), None).unwrap().len(),
        1
    );
    assert!(parse_pem_or_der(None, Some("%%%")).is_err());
    assert!(parse_pem_or_der(Some("x"), Some("y")).is_err());
    assert!(parse_pem_or_der(None, None).is_err());
}

#[test]
fn signatures_verify_only_over_their_bytes_with_the_matching_algorithm() {
    let pki = Pki::get();
    for signer in [&pki.maria, &pki.jose] {
        let algorithm = algorithm_of(signer);
        let signature = signer.sign(b"payload");
        assert!(verify_signature(&signer.cert, algorithm, b"payload", &signature).unwrap());
        assert!(!verify_signature(&signer.cert, algorithm, b"payload!", &signature).unwrap());
        assert!(!verify_signature(&signer.cert, algorithm, b"payload", b"garbage").unwrap());
        let other = SignatureAlgorithm::iter()
            .find(|a| *a != algorithm)
            .unwrap();
        assert!(!verify_signature(&signer.cert, other, b"payload", &signature).unwrap());
    }
    // Someone else's key never verifies.
    let signature = pki.ana.sign(b"payload");
    assert!(!verify_signature(
        &pki.maria.cert,
        SignatureAlgorithm::RsaPkcs1Sha256,
        b"payload",
        &signature
    )
    .unwrap());
}

// Happy paths

#[test]
fn a_registered_rsa_signer_passes_every_check() {
    let pki = Pki::get();
    let request = request(SigningAction::CloseVoting, Some(POST_A));
    let mut context = context();
    let row = registration(&pki.maria, "maria", Some(POST_A));
    context.registrations = vec![row.clone()];
    let verification = run(&context, &signing(&request, "maria", &pki.maria, &[]));
    assert_passes(&verification);
    let ids: Vec<_> = verification.checks.iter().map(|check| check.id).collect();
    assert_eq!(ids, CertificateCheckId::iter().collect::<Vec<_>>());
    assert_eq!(verification.revocation_status, RevocationStatus::Checked);
    assert_eq!(
        verification.registration,
        RegistrationState::Registered(row.clone())
    );
    // The details the dialog shows: the root CA's name (both CAs are
    // trusted, and the path stops at the issuing one), the list's download
    // time and the registration date.
    let detail = |id| verification.check(id).unwrap().detail.clone();
    assert_eq!(
        detail(CertificateCheckId::TrustedIssuer).as_deref(),
        Some("Test Staff Root CA")
    );
    assert_eq!(
        detail(CertificateCheckId::NotRevoked),
        Some((now() - Duration::hours(1)).to_rfc3339())
    );
    assert_eq!(
        detail(CertificateCheckId::Registered),
        Some(row.registered_at.to_rfc3339())
    );
    assert_eq!(detail(CertificateCheckId::RegisteredToOther), None);
}

#[test]
fn an_ec_signer_passes_on_first_use_and_the_dry_run_has_no_signature_check() {
    let pki = Pki::get();
    let request = request(SigningAction::CloseVoting, Some(POST_A));
    let mut input = signing(&request, "jose", &pki.jose, &[]);
    let verification = run(&context(), &input);
    assert_passes(&verification);
    assert_eq!(verification.registration, RegistrationState::FirstUse);
    // First use: no registration date yet.
    assert_eq!(
        verification
            .check(CertificateCheckId::Registered)
            .unwrap()
            .detail,
        None
    );

    input.signatures = None;
    let dry_run = run(&context(), &input);
    assert_passes(&dry_run);
    assert!(dry_run.check(CertificateCheckId::Signature).is_none());
    assert_eq!(dry_run.checks.len(), CertificateCheckId::iter().count() - 1);
}

#[test]
fn only_trusted_issuers_are_anchors_and_browser_intermediates_help() {
    let pki = Pki::get();
    let request = request(SigningAction::CloseVoting, Some(POST_A));
    let mut context = context();
    // Only the root is trusted: the browser's intermediate completes the path.
    context.issuers = vec![pki.root.cert.clone()];
    assert_eq!(
        result(
            &run(
                &context,
                &signing(&request, "maria", &pki.maria, &[&pki.individual_ca])
            ),
            CertificateCheckId::TrustedIssuer
        ),
        (true, "Test Staff Root CA".to_owned())
    );
    let without = run(&context, &signing(&request, "maria", &pki.maria, &[]));
    assert_eq!(result(&without, CertificateCheckId::TrustedIssuer).0, false);

    // The intermediate alone is a trust anchor too. Without its root among
    // the issuers, the check names the highest trusted issuer.
    context.issuers = vec![pki.individual_ca.cert.clone()];
    let alone = run(&context, &signing(&request, "maria", &pki.maria, &[]));
    assert_passes(&alone);
    assert_eq!(
        result(&alone, CertificateCheckId::TrustedIssuer),
        (true, "Test Staff Individual CA".to_owned())
    );
    // With both, in either order, it names the root.
    for issuers in [
        vec![pki.individual_ca.cert.clone(), pki.root.cert.clone()],
        vec![pki.root.cert.clone(), pki.individual_ca.cert.clone()],
    ] {
        context.issuers = issuers;
        assert_eq!(
            result(
                &run(&context, &signing(&request, "maria", &pki.maria, &[])),
                CertificateCheckId::TrustedIssuer
            ),
            (true, "Test Staff Root CA".to_owned())
        );
    }

    let path = verified_path(
        &pki.maria.cert,
        &[pki.individual_ca.cert.clone()],
        &[pki.root.cert.clone()],
    )
    .unwrap()
    .unwrap();
    assert_eq!(path.len(), 3);
}

// Refusals

#[test]
fn an_untrusted_chain_is_refused_even_with_its_own_root_sent() {
    let pki = Pki::get();
    let request = request(SigningAction::CloseVoting, Some(POST_A));
    let control = run(&context(), &signing(&request, "maria", &pki.maria, &[]));
    assert_passes(&control);

    let verification = run(
        &context(),
        &signing(
            &request,
            "rosa",
            &pki.foreign_signer,
            &[&pki.foreign_ca, &pki.foreign_root],
        ),
    );
    let (ok, detail) = result(&verification, CertificateCheckId::TrustedIssuer);
    assert!(
        !ok && detail.starts_with("not issued by a trusted issuer"),
        "{detail}"
    );
    assert_eq!(
        verification.refusal().unwrap().id,
        CertificateCheckId::TrustedIssuer
    );
    assert_eq!(verification.revocation_status, RevocationStatus::Unchecked);

    // No issuers at all.
    let mut empty = context();
    empty.issuers.clear();
    let verification = run(&empty, &signing(&request, "maria", &pki.maria, &[]));
    assert_eq!(
        result(&verification, CertificateCheckId::TrustedIssuer),
        (
            false,
            "not issued by a trusted issuer: the event has no trusted issuers".to_owned()
        )
    );
}

#[test]
fn a_trusted_issuer_itself_cant_sign() {
    let pki = Pki::get();
    let request = request(SigningAction::CloseVoting, Some(POST_A));
    // Trusted itself, or issued by a trusted root: a CA never signs.
    for issuer in [&pki.root, &pki.individual_ca] {
        let mut input = signing(&request, "ca", issuer, &[]);
        input.signatures = None;
        let verification = run(&context(), &input);
        assert_eq!(
            result(&verification, CertificateCheckId::TrustedIssuer),
            (
                false,
                "not issued by a trusted issuer: a certificate authority can't sign as a person"
                    .to_owned()
            )
        );
    }
}

#[test]
fn unreadable_chains_fail_the_first_check_only() {
    let pki = Pki::get();
    let request = request(SigningAction::CloseVoting, Some(POST_A));
    let mut input = signing(&request, "maria", &pki.maria, &[]);
    input.chain_pem =
        vec!["-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n".to_owned()];
    let verification = run(&context(), &input);
    assert_eq!(verification.checks.len(), 1);
    assert_eq!(verification.checks[0].id, CertificateCheckId::TrustedIssuer);
    assert!(!verification.checks[0].ok);
    assert!(verification.certificate.is_none());
    assert!(!verification.passed());
}

#[test]
fn expired_and_not_yet_valid_certificates_are_refused() {
    let pki = Pki::get();
    let request = request(SigningAction::CloseVoting, Some(POST_A));
    let expired = run(&context(), &signing(&request, "ramon", &pki.expired, &[]));
    assert_refused_by(&expired, CertificateCheckId::ValidNow);
    let detail = result(&expired, CertificateCheckId::ValidNow).1;
    assert_eq!(
        detail,
        format!(
            "the certificate expired on {}",
            identity(&pki.expired).not_after.format("%Y-%m-%d")
        )
    );

    let early = run(
        &context(),
        &signing(&request, "liza", &pki.not_yet_valid, &[]),
    );
    assert_refused_by(&early, CertificateCheckId::ValidNow);
    assert!(result(&early, CertificateCheckId::ValidNow)
        .1
        .contains("not valid before"));

    // The same certificate is valid a year later.
    let mut later = signing(&request, "liza", &pki.not_yet_valid, &[]);
    later.now = now() + Duration::days(400);
    let verification = run(&context(), &later);
    assert!(result(&verification, CertificateCheckId::ValidNow).0);
}

#[test]
fn certificates_not_made_for_signing_are_refused() {
    let pki = Pki::get();
    let request = request(SigningAction::CloseVoting, Some(POST_A));
    for signer in [&pki.no_signing_usage, &pki.no_key_usage] {
        let verification = run(&context(), &signing(&request, "pedro", signer, &[]));
        assert_refused_by(&verification, CertificateCheckId::SigningKeyUsage);
    }
    // An EC key on another curve can't sign either.
    let input = CertificateVerificationInput {
        request: &request,
        signer: actor("tomas"),
        chain_pem: vec![pem(&pki.p384)],
        signatures: None,
        now: now(),
    };
    let verification = run(&context(), &input);
    assert_refused_by(&verification, CertificateCheckId::SigningKeyUsage);
    assert!(result(&verification, CertificateCheckId::SigningKeyUsage)
        .1
        .contains("unsupported key"));
}

#[test]
fn a_revoked_certificate_is_refused_by_its_issuers_list() {
    let pki = Pki::get();
    let request = request(SigningAction::CloseVoting, Some(POST_A));
    let verification = run(&context(), &signing(&request, "carmen", &pki.revoked, &[]));
    assert_refused_by(&verification, CertificateCheckId::NotRevoked);
    assert_eq!(
        result(&verification, CertificateCheckId::NotRevoked).1,
        "Carmen Villanueva is revoked"
    );

    // A list that is no longer current still revokes it.
    let mut context = context();
    context.crls = vec![stored(crl_signed_by(
        &pki.individual_ca,
        &pki.individual_ca.key,
        &[REVOKED_SERIAL],
        pki_now() - 30 * 86_400,
        Some(pki_now() - 20 * 86_400),
    ))];
    let verification = run(&context, &signing(&request, "carmen", &pki.revoked, &[]));
    assert_refused_by(&verification, CertificateCheckId::NotRevoked);

    // A list signed with another key revokes nothing, and isn't a list.
    context.crls = vec![stored(crl_signed_by(
        &pki.individual_ca,
        &pki.ana.key,
        &[REVOKED_SERIAL],
        pki_now() - 86_400,
        Some(pki_now() + 86_400),
    ))];
    let verification = run(&context, &signing(&request, "carmen", &pki.revoked, &[]));
    assert!(result(&verification, CertificateCheckId::NotRevoked)
        .1
        .starts_with("no current revocation list"));
}

/// Revocation under each policy when the individual CA's list is `crls`.
fn revocation_with(
    crls: Vec<StoredCrl>,
    revocation_check: RevocationCheck,
    crl_unavailable: CrlUnavailablePolicy,
) -> CertificateVerification {
    let pki = Pki::get();
    let request = request(SigningAction::CloseVoting, Some(POST_A));
    let mut context = context();
    context.crls = crls;
    context.checks = SigningChecks {
        revocation_check,
        crl_unavailable,
        ..SigningChecks::default()
    };
    run(&context, &signing(&request, "maria", &pki.maria, &[]))
}

#[test]
fn a_missing_list_is_refused_or_accepted_unchecked_as_configured() {
    let pki = Pki::get();
    let stale = crl_signed_by(
        &pki.individual_ca,
        &pki.individual_ca.key,
        &[],
        pki_now() - 9 * 86_400,
        Some(pki_now() - 86_400),
    );
    for crls in [
        vec![],
        vec![stored(stale)],
        vec![StoredCrl {
            der: None,
            ..stored(vec![])
        }],
    ] {
        let refused = revocation_with(
            crls.clone(),
            RevocationCheck::Check,
            CrlUnavailablePolicy::Refuse,
        );
        assert_refused_by(&refused, CertificateCheckId::NotRevoked);
        assert_eq!(
            result(&refused, CertificateCheckId::NotRevoked).1,
            "no current revocation list for Maria Santos"
        );

        let accepted = revocation_with(
            crls.clone(),
            RevocationCheck::Check,
            CrlUnavailablePolicy::AcceptUnchecked,
        );
        assert_passes(&accepted);
        assert_eq!(
            accepted
                .check(CertificateCheckId::NotRevoked)
                .unwrap()
                .detail,
            None
        );
        assert_eq!(accepted.revocation_status, RevocationStatus::Unchecked);

        let unchecked = revocation_with(
            crls,
            RevocationCheck::DontCheck,
            CrlUnavailablePolicy::Refuse,
        );
        assert_passes(&unchecked);
        assert_eq!(unchecked.revocation_status, RevocationStatus::Unchecked);
    }
    // The control: a current list checks it under either policy.
    for policy in CrlUnavailablePolicy::iter() {
        let checked = revocation_with(
            vec![stored(pki.individual_crl())],
            RevocationCheck::Check,
            policy,
        );
        assert_passes(&checked);
        assert_eq!(checked.revocation_status, RevocationStatus::Checked);
    }
    // Not checking revocation also lets a revoked certificate through.
    let request = request(SigningAction::CloseVoting, Some(POST_A));
    let mut context = context();
    context.checks.revocation_check = RevocationCheck::DontCheck;
    assert_passes(&run(
        &context,
        &signing(&request, "carmen", &pki.revoked, &[]),
    ));
}

#[test]
fn a_browser_intermediate_needs_its_issuers_list_too() {
    let pki = Pki::get();
    let request = request(SigningAction::CloseVoting, Some(POST_A));
    let mut context = context();
    context.issuers = vec![pki.root.cert.clone()];
    context.crls = vec![stored(pki.individual_crl())];
    let verification = run(
        &context,
        &signing(&request, "maria", &pki.maria, &[&pki.individual_ca]),
    );
    assert_refused_by(&verification, CertificateCheckId::NotRevoked);
    assert!(result(&verification, CertificateCheckId::NotRevoked)
        .1
        .contains("Test Staff Individual CA"));
    context.crls.push(stored_at(ROOT_CRL_URL, pki.root_crl()));
    assert_passes(&run(
        &context,
        &signing(&request, "maria", &pki.maria, &[&pki.individual_ca]),
    ));
    // A revoked intermediate revokes its signers.
    context.crls = vec![
        stored(pki.individual_crl()),
        stored_at(ROOT_CRL_URL, crl(&pki.root, &[2])),
    ];
    let verification = run(
        &context,
        &signing(&request, "maria", &pki.maria, &[&pki.individual_ca]),
    );
    assert_refused_by(&verification, CertificateCheckId::NotRevoked);
}

#[test]
fn a_certificate_registered_to_someone_else_is_refused() {
    let pki = Pki::get();
    let request = request(SigningAction::CloseVoting, Some(POST_A));
    let bob = |row: StaffCertificateRow| vec![row];
    for (case, registrations, signer) in [
        // Bob signs with Maria's certificate.
        (
            "fingerprint",
            bob(registration(&pki.maria, "maria", Some(POST_A))),
            &pki.maria,
        ),
        // Bob signs with a reissue of Maria's key.
        (
            "key",
            bob(registration(&pki.maria, "maria", Some(POST_A))),
            &pki.maria_reissue,
        ),
        // Bob signs with a second certificate of Juan's.
        (
            "holder",
            bob(registration(&pki.juan_a, "juan", Some(POST_A))),
            &pki.juan_b,
        ),
    ] {
        let mut context = context();
        context.registrations = registrations;
        let verification = run(&context, &signing(&request, "bob", signer, &[]));
        assert_registered_to_other(&verification);
        let owner = if case == "holder" {
            "Display juan"
        } else {
            "Display maria"
        };
        // The detail names the owner.
        assert_eq!(
            result(&verification, CertificateCheckId::RegisteredToOther).1,
            owner,
            "{case}"
        );
        assert_eq!(
            verification.registration,
            RegistrationState::NotRegistered,
            "{case}"
        );

        // The owner herself registers a reissue or a second certificate on
        // first use.
        let verification = run(
            &context,
            &signing(
                &request,
                if case == "holder" { "juan" } else { "maria" },
                signer,
                &[],
            ),
        );
        assert_passes(&verification);
    }
}

#[test]
fn a_security_officer_link_lets_the_second_account_sign_only() {
    let pki = Pki::get();
    let request = request(SigningAction::ApproveConfiguration, None);
    let maria = registration(&pki.maria, "maria", None);
    let mut linked = registration(&pki.maria, "maria-trustee", None);
    linked.linked_to = Some("maria".to_owned());
    linked.registration = StaffCertificateRegistration::SecurityOfficer;
    let mut context = context();
    context.registrations = vec![maria, linked.clone()];

    for user in ["maria", "maria-trustee"] {
        assert_passes(&run(&context, &signing(&request, user, &pki.maria, &[])));
    }
    let linked_run = run(
        &context,
        &signing(&request, "maria-trustee", &pki.maria, &[]),
    );
    assert_eq!(
        linked_run.registration,
        RegistrationState::Registered(linked)
    );
    // A third account is not linked: refused.
    let verification = run(&context, &signing(&request, "bob", &pki.maria, &[]));
    assert_registered_to_other(&verification);
}

#[test]
fn security_officer_only_registration_refuses_unregistered_certificates() {
    let pki = Pki::get();
    let request = request(SigningAction::CloseVoting, Some(POST_A));
    let mut context = context();
    context.checks.registration = CertificateRegistration::SecurityOfficerOnly;
    let verification = run(&context, &signing(&request, "maria", &pki.maria, &[]));
    assert_refused_by(&verification, CertificateCheckId::Registered);
    assert_eq!(verification.registration, RegistrationState::NotRegistered);

    context.registrations = vec![registration(&pki.maria, "maria", Some(POST_A))];
    assert_passes(&run(&context, &signing(&request, "maria", &pki.maria, &[])));
    // Registered to her in another event only: still not registered here.
    context.registrations[0].election_event_id = Uuid::from_u128(0xee);
    let verification = run(&context, &signing(&request, "maria", &pki.maria, &[]));
    assert_refused_by(&verification, CertificateCheckId::Registered);
}

#[test]
fn a_revoked_certificate_or_key_is_not_registered_again_on_first_use() {
    let pki = Pki::get();
    let request = request(SigningAction::CloseVoting, Some(POST_A));
    let mut revoked = registration(&pki.maria, "maria", Some(POST_A));
    revoked.status = StaffCertificateStatus::Revoked;
    let mut context = context();
    context.revoked = vec![revoked];
    for signer in [&pki.maria, &pki.maria_reissue] {
        let verification = run(&context, &signing(&request, "maria", signer, &[]));
        assert_refused_by(&verification, CertificateCheckId::Registered);
        assert!(result(&verification, CertificateCheckId::Registered)
            .1
            .contains("revoked"));
    }
    // Even an active registration doesn't sign with a revoked key.
    context.registrations = vec![registration(&pki.maria, "maria", Some(POST_A))];
    let verification = run(&context, &signing(&request, "maria", &pki.maria, &[]));
    assert_refused_by(&verification, CertificateCheckId::Registered);
}

#[test]
fn one_post_refuses_a_key_that_signed_for_another_post() {
    let pki = Pki::get();
    let mut context = context();
    // The key signed for Post A (through any account and certificate).
    context.signed_posts = vec![POST_A];
    let at_b = request(SigningAction::CloseVoting, Some(POST_B));
    for signer in [&pki.maria, &pki.maria_reissue] {
        let verification = run(&context, &signing(&at_b, "maria", signer, &[]));
        assert_refused_by(&verification, CertificateCheckId::PostBinding);
    }

    // Its own Post, any Post, and event-level actions pass.
    let at_a = request(SigningAction::CloseVoting, Some(POST_A));
    assert_passes(&run(&context, &signing(&at_a, "maria", &pki.maria, &[])));
    let event_level = request(SigningAction::ApproveConfiguration, None);
    assert_passes(&run(
        &context,
        &signing(&event_level, "maria", &pki.maria, &[]),
    ));
    context.checks.post_binding = CertificatePostBinding::AnyPost;
    assert_passes(&run(&context, &signing(&at_b, "maria", &pki.maria, &[])));

    // A registration's Post doesn't bind: only a signature does.
    let mut context = self::context();
    context.registrations = vec![registration(&pki.maria, "maria", Some(POST_A))];
    assert_passes(&run(&context, &signing(&at_b, "maria", &pki.maria, &[])));
}

#[test]
fn one_person_fills_one_slot() {
    let pki = Pki::get();
    let request = request(SigningAction::CloseVoting, Some(POST_A));
    for (earlier, user, signer, reason) in [
        (
            approval(&pki.jose, "maria"),
            "maria",
            &pki.maria,
            "you already signed this request",
        ),
        (
            approval(&pki.maria, "ana"),
            "maria",
            &pki.maria,
            "this certificate already signed this request",
        ),
        (
            approval(&pki.maria_reissue, "ana"),
            "maria",
            &pki.maria,
            "this key already signed this request",
        ),
        (
            approval(&pki.juan_a, "ana"),
            "juan",
            &pki.juan_b,
            "this certificate's holder already signed this request",
        ),
    ] {
        let mut context = context();
        context.approvals = vec![earlier];
        let verification = run(&context, &signing(&request, user, signer, &[]));
        assert_refused_by(&verification, CertificateCheckId::AlreadySigned);
        assert_eq!(
            result(&verification, CertificateCheckId::AlreadySigned).1,
            reason
        );
    }
    let mut context = context();
    context.approvals = vec![approval(&pki.jose, "jose")];
    assert_passes(&run(&context, &signing(&request, "maria", &pki.maria, &[])));
}

#[test]
fn a_signature_that_doesnt_cover_the_payload_is_refused() {
    let pki = Pki::get();
    let request = request(SigningAction::CloseVoting, Some(POST_A));
    let mut input = signing(&request, "maria", &pki.maria, &[]);
    let signatures = input.signatures.as_mut().unwrap();
    signatures.payload_signature = pki.maria.sign(b"{\"action\":\"open-voting\"}");
    let verification = run(&context(), &input);
    assert_refused_by(&verification, CertificateCheckId::Signature);

    // Someone else's signature, and a claimed algorithm the key doesn't make.
    let mut input = signing(&request, "maria", &pki.maria, &[]);
    input.signatures.as_mut().unwrap().payload_signature = pki.ana.sign(PAYLOAD.as_bytes());
    assert_refused_by(&run(&context(), &input), CertificateCheckId::Signature);
    let mut input = signing(&request, "maria", &pki.maria, &[]);
    input.signatures.as_mut().unwrap().algorithm = SignatureAlgorithm::EcdsaP256Sha256;
    assert_refused_by(&run(&context(), &input), CertificateCheckId::Signature);
}

#[test]
fn eml_actions_need_a_document_signature_over_the_request_document() {
    let pki = Pki::get();
    let eml = b"<EML>results</EML>".to_vec();
    let mut transmit = request(SigningAction::TransmitResults, Some(POST_A));
    transmit.document_sha256 = Some(sha(&eml));
    let with_document = |document: Vec<u8>, signature: Vec<u8>| {
        let mut input = signing(&transmit, "jose", &pki.jose, &[]);
        input.signatures.as_mut().unwrap().document = Some(DocumentSignatureInput {
            document,
            signature,
        });
        run(&context(), &input)
    };
    assert_passes(&with_document(eml.clone(), pki.jose.sign(&eml)));
    let other = b"<EML>other</EML>".to_vec();
    assert_refused_by(
        &with_document(other.clone(), pki.jose.sign(&other)),
        CertificateCheckId::Signature,
    );
    assert_refused_by(
        &with_document(eml.clone(), pki.ana.sign(&eml)),
        CertificateCheckId::Signature,
    );
    let missing = run(&context(), &signing(&transmit, "jose", &pki.jose, &[]));
    assert_refused_by(&missing, CertificateCheckId::Signature);
    assert_eq!(
        result(&missing, CertificateCheckId::Signature).1,
        "the document signature is missing"
    );

    // An action without a document refuses one.
    let close = request(SigningAction::CloseVoting, Some(POST_A));
    let mut input = signing(&close, "jose", &pki.jose, &[]);
    input.signatures.as_mut().unwrap().document = Some(DocumentSignatureInput {
        signature: pki.jose.sign(&eml),
        document: eml,
    });
    assert_refused_by(&run(&context(), &input), CertificateCheckId::Signature);
}

#[test]
fn check_results_serialize_with_their_kebab_case_ids() {
    let pki = Pki::get();
    let request = request(SigningAction::CloseVoting, Some(POST_A));
    let verification = run(&context(), &signing(&request, "maria", &pki.maria, &[]));
    let ids = serde_json::to_value(&verification.checks).unwrap();
    assert_eq!(ids[0]["id"], "trusted-issuer");
    assert_eq!(ids[8]["id"], "signature");
}

/// Answers one HTTP request on a local port with `status` and `body`.
async fn serve_once(status: &'static str, body: Vec<u8>) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = [0u8; 4096];
        let _ = stream.read(&mut request).await;
        let head = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(head.as_bytes()).await.unwrap();
        stream.write_all(&body).await.unwrap();
    });
    format!("http://{address}/list.crl")
}

fn http_fetcher(
    allowed: &[&str],
    max_bytes: usize,
) -> windmill::services::signing::crl::HttpCrlFetcher {
    windmill::services::signing::crl::HttpCrlFetcher {
        allowed_hosts: allowed.iter().map(|host| host.to_string()).collect(),
        max_bytes,
        ..Default::default()
    }
}

#[tokio::test]
async fn the_http_fetcher_downloads_from_allowed_hosts_only_without_redirects() {
    use windmill::services::signing::crl::{CrlFetcher, MAX_CRL_BYTES};
    let pki = Pki::get();
    let allowed = http_fetcher(&["127.0.0.1"], MAX_CRL_BYTES);
    let url = serve_once("200 OK", pki.individual_crl()).await;
    assert_eq!(allowed.fetch(&url).await.unwrap(), pki.individual_crl());

    // A loopback address is refused unless allowed.
    let url = serve_once("200 OK", pki.individual_crl()).await;
    let error = http_fetcher(&[], MAX_CRL_BYTES)
        .fetch(&url)
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "refused: the list's address is not public"
    );

    // Errors are generic; redirects aren't followed; the size is capped.
    let url = serve_once("404 Not Found", b"secret body".to_vec()).await;
    assert_eq!(
        allowed.fetch(&url).await.unwrap_err().to_string(),
        "download failed: HTTP 4xx"
    );
    let url = serve_once(
        "302 Found\r\nLocation: http://169.254.169.254/latest",
        vec![],
    )
    .await;
    assert_eq!(
        allowed.fetch(&url).await.unwrap_err().to_string(),
        "download failed: HTTP 3xx"
    );
    let url = serve_once("200 OK", vec![0; 64]).await;
    assert_eq!(
        http_fetcher(&["127.0.0.1"], 32)
            .fetch(&url)
            .await
            .unwrap_err()
            .to_string(),
        "the list is larger than 32 bytes"
    );
    let error = allowed
        .fetch("ldap://directory.invalid/cn=CA")
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "only http and https distribution points are downloaded"
    );
}

#[test]
fn only_public_addresses_are_downloaded_from() {
    use windmill::services::signing::crl::is_public_address;
    for (address, public) in [
        ("8.8.8.8", true),
        ("2606:4700::1111", true),
        ("127.0.0.1", false),
        ("10.1.2.3", false),
        ("172.16.0.1", false),
        ("192.168.1.1", false),
        ("169.254.169.254", false),
        ("100.64.0.1", false),
        ("0.0.0.0", false),
        ("224.0.0.1", false),
        ("::1", false),
        ("fd00::1", false),
        ("fe80::1", false),
        ("::ffff:10.0.0.1", false),
        ("::ffff:8.8.8.8", true),
    ] {
        assert_eq!(
            is_public_address(address.parse().unwrap()),
            public,
            "{address}"
        );
    }
}

#[test]
fn first_use_refuses_a_certificate_linked_to_another_account_in_any_event() {
    let pki = Pki::get();
    let maria = registration(&pki.maria, "maria", None);
    let mut linked = registration(&pki.maria, "maria-trustee", None);
    linked.linked_to = Some("maria".to_owned());
    linked.registration = StaffCertificateRegistration::SecurityOfficer;
    let mut other_event = request(SigningAction::ApproveConfiguration, None);
    other_event.election_event_id = Uuid::from_u128(0xe2);
    let mut context = context();
    context.registrations = vec![maria, linked];
    // In another event neither account has a row, and each shares the
    // certificate with the other: a Security Officer must link it there.
    for user in ["maria", "maria-trustee"] {
        let verification = run(&context, &signing(&other_event, user, &pki.maria, &[]));
        assert_refused_by(&verification, CertificateCheckId::Registered);
        assert_eq!(verification.registration, RegistrationState::NotRegistered);
        assert!(result(&verification, CertificateCheckId::Registered)
            .1
            .contains("must link"));
    }
}

#[test]
fn extended_key_usages_must_allow_signing() {
    let pki = Pki::get();
    let request = request(SigningAction::CloseVoting, Some(POST_A));
    let subject = person("Eku Signer", "PH-0020");
    let server_only = issued(
        &Spec {
            usage: Usage::ServerAuthOnly,
            ..Spec::signer(&subject, 2001)
        },
        rsa_key(),
        Some(&pki.individual_ca),
    );
    let verification = run(&context(), &signing(&request, "eku", &server_only, &[]));
    assert_refused_by(&verification, CertificateCheckId::SigningKeyUsage);
    assert!(result(&verification, CertificateCheckId::SigningKeyUsage)
        .1
        .contains("extended key usage"));
    let document_signing = issued(
        &Spec {
            usage: Usage::DocumentSigningOnly,
            ..Spec::signer(&subject, 2002)
        },
        rsa_key(),
        Some(&pki.individual_ca),
    );
    assert_passes(&run(
        &context(),
        &signing(&request, "eku", &document_signing, &[]),
    ));
}

#[test]
fn weak_keys_and_sha1_chains_are_refused() {
    let pki = Pki::get();
    let request = request(SigningAction::CloseVoting, Some(POST_A));
    let subject = person("Weak Signer", "PH-0021");
    let weak = issued(
        &Spec::signer(&subject, 2003),
        openssl::pkey::PKey::from_rsa(openssl::rsa::Rsa::generate(1024).unwrap()).unwrap(),
        Some(&pki.individual_ca),
    );
    let verification = run(&context(), &signing(&request, "weak", &weak, &[]));
    assert!(!verification.passed());
    assert!(
        !verification
            .check(CertificateCheckId::TrustedIssuer)
            .unwrap()
            .ok
    );
    assert!(
        !verification
            .check(CertificateCheckId::SigningKeyUsage)
            .unwrap()
            .ok
    );

    let mut name = openssl::x509::X509NameBuilder::new().unwrap();
    name.append_entry_by_text("CN", "Sha One").unwrap();
    let key = rsa_key();
    let sha1 = Issued {
        cert: issue_named(
            &Spec::signer(&subject, 2004),
            name.build(),
            &key,
            Some(&pki.individual_ca),
            MessageDigest::sha1(),
        ),
        key,
    };
    let verification = run(&context(), &signing(&request, "sha1", &sha1, &[]));
    assert_eq!(
        verification.refusal().unwrap().id,
        CertificateCheckId::TrustedIssuer
    );
}

#[test]
fn forged_chains_are_refused() {
    let pki = Pki::get();
    let request = request(SigningAction::CloseVoting, Some(POST_A));
    // A leaf "issued" by Maria's end-entity certificate.
    let by_maria = issued(
        &Spec::signer(&person("By Maria", "PH-0022"), 2005),
        rsa_key(),
        Some(&pki.maria),
    );
    let verification = run(
        &context(),
        &signing(&request, "x", &by_maria, &[&pki.maria]),
    );
    assert_eq!(
        verification.refusal().unwrap().id,
        CertificateCheckId::TrustedIssuer
    );
    // An intermediate named like a trusted issuer, with another key.
    let fake_ca = issued(
        &Spec {
            subject: &[
                ("C", "PH"),
                ("O", "Test Staff PKI"),
                ("CN", "Test Staff Individual CA"),
            ],
            serial: 99,
            not_before: pki_now() - 86_400,
            not_after: pki_now() + 86_400 * 365,
            usage: Usage::Ca,
            crl_url: None,
        },
        rsa_key(),
        None,
    );
    let by_fake = issued(
        &Spec::signer(&person("By Fake", "PH-0023"), 2006),
        rsa_key(),
        Some(&fake_ca),
    );
    let verification = run(&context(), &signing(&request, "x", &by_fake, &[&fake_ca]));
    assert_eq!(
        verification.refusal().unwrap().id,
        CertificateCheckId::TrustedIssuer
    );
}

#[test]
fn holders_compare_regardless_of_string_type_case_and_spacing() {
    use openssl::asn1::Asn1Type;
    use openssl::nid::Nid;
    let pki = Pki::get();
    let name = |cn: &str, kind: Asn1Type| {
        let mut name = openssl::x509::X509NameBuilder::new().unwrap();
        name.append_entry_by_nid_with_type(Nid::COUNTRYNAME, "PH", Asn1Type::PRINTABLESTRING)
            .unwrap();
        name.append_entry_by_nid_with_type(Nid::COMMONNAME, cn, kind)
            .unwrap();
        name.build()
    };
    let make = |cn: &str, kind: Asn1Type, serial: u32| {
        let key = rsa_key();
        Issued {
            cert: issue_named(
                &Spec::signer(&[], serial),
                name(cn, kind),
                &key,
                Some(&pki.individual_ca),
                MessageDigest::sha256(),
            ),
            key,
        }
    };
    let printable = make("Juan Dela Cruz", Asn1Type::PRINTABLESTRING, 2007);
    let utf8 = make("  JUAN   dela CRUZ ", Asn1Type::UTF8STRING, 2008);
    let other = make("Juana Dela Cruz", Asn1Type::UTF8STRING, 2009);
    assert_eq!(
        identity(&printable).holder_sha256,
        identity(&utf8).holder_sha256
    );
    assert_ne!(
        identity(&printable).holder_sha256,
        identity(&other).holder_sha256
    );

    // Registered to Juan, the other encoding of his name is his too.
    let request = request(SigningAction::CloseVoting, Some(POST_A));
    let mut context = context();
    context.registrations = vec![registration(&printable, "juan", None)];
    let verification = run(&context, &signing(&request, "bob", &utf8, &[]));
    assert_registered_to_other(&verification);
}

/// The revocation result of Carmen's revoked certificate (or Maria's) with
/// the individual CA's list as `extensions` and `url` make it.
fn with_list(
    signer: &Issued,
    url: &str,
    revoked: &[u32],
    extensions: &[Vec<u8>],
) -> CertificateVerification {
    let pki = Pki::get();
    let request = request(SigningAction::CloseVoting, Some(POST_A));
    let mut context = context();
    context.crls = vec![
        StoredCrl {
            url: url.to_owned(),
            ..stored(crl_with(
                &pki.individual_ca,
                &pki.individual_ca.key,
                revoked,
                pki_now() - 86_400,
                Some(pki_now() + 86_400),
                extensions,
            ))
        },
        stored_at(ROOT_CRL_URL, pki.root_crl()),
    ];
    run(&context, &signing(&request, "signer", signer, &[]))
}

#[test]
fn a_list_speaks_only_for_the_certificates_in_its_scope() {
    let pki = Pki::get();
    let revoked = [REVOKED_SERIAL];
    // The control: a complete list from the certificate's point revokes it.
    let control = with_list(&pki.revoked, CRL_URL, &revoked, &[crl_number(5)]);
    assert_refused_by(&control, CertificateCheckId::NotRevoked);
    assert!(result(&control, CertificateCheckId::NotRevoked)
        .1
        .ends_with("is revoked"));
    let ok = with_list(&pki.maria, CRL_URL, &[], &[crl_number(5)]);
    assert_passes(&ok);
    let user_only = Idp {
        uri: Some(CRL_URL),
        only_user_certs: true,
        ..Idp::default()
    };
    assert_passes(&with_list(
        &pki.maria,
        CRL_URL,
        &[],
        &[issuing_distribution_point(&user_only)],
    ));

    // Lists that don't speak for an end entity's revocation: none current.
    for (case, url, extensions) in [
        ("moved", "http://crl.staff-ca.invalid/elsewhere.crl", vec![]),
        ("delta", CRL_URL, vec![delta_indicator(4)]),
        (
            "ca only",
            CRL_URL,
            vec![issuing_distribution_point(&Idp {
                only_ca_certs: true,
                ..Idp::default()
            })],
        ),
        (
            "some reasons",
            CRL_URL,
            vec![issuing_distribution_point(&Idp {
                only_some_reasons: true,
                ..Idp::default()
            })],
        ),
        (
            "indirect",
            CRL_URL,
            vec![issuing_distribution_point(&Idp {
                indirect: true,
                ..Idp::default()
            })],
        ),
        (
            "other point",
            CRL_URL,
            vec![issuing_distribution_point(&Idp {
                uri: Some("http://other.invalid/x.crl"),
                ..Idp::default()
            })],
        ),
        (
            "unknown critical",
            CRL_URL,
            vec![unknown_critical_extension()],
        ),
    ] {
        let verification = with_list(&pki.maria, url, &[], &extensions);
        assert_refused_by(&verification, CertificateCheckId::NotRevoked);
        assert_eq!(
            result(&verification, CertificateCheckId::NotRevoked).1,
            "no current revocation list for Maria Santos",
            "{case}"
        );
        assert_eq!(
            verification.revocation_status,
            RevocationStatus::Unchecked,
            "{case}"
        );
        // Nor do they revoke it.
        let verification = with_list(&pki.revoked, url, &revoked, &extensions);
        assert!(
            result(&verification, CertificateCheckId::NotRevoked)
                .1
                .starts_with("no current"),
            "{case}"
        );
    }
    // A certificate without a distribution point takes any of its issuer's lists.
    let no_point = issued(
        &Spec {
            crl_url: None,
            ..Spec::signer(&person("No Point", "PH-0024"), 2010)
        },
        rsa_key(),
        Some(&pki.individual_ca),
    );
    assert_passes(&with_list(
        &no_point,
        "http://crl.staff-ca.invalid/any.crl",
        &[],
        &[],
    ));
}

#[test]
fn downloaded_lists_never_roll_back() {
    use windmill::postgres::signing_certificates::StaffCrlRow;
    use windmill::services::signing::crl::{check_crl, list_scope};
    let pki = Pki::get();
    let ca = &pki.individual_ca;
    let list = |this: i64, number: u32, extensions: Vec<Vec<u8>>| {
        let mut all = vec![crl_number(number)];
        all.extend(extensions);
        crl_with(ca, &ca.key, &[], this, Some(this + 7 * 86_400), &all)
    };
    let stored_row = |der: Vec<u8>| StaffCrlRow {
        id: Uuid::nil(),
        tenant_id: TENANT,
        election_event_id: EVENT,
        issuer_id: Uuid::nil(),
        issuer_fingerprint: String::new(),
        url: CRL_URL.to_owned(),
        der: Some(der),
        this_update: None,
        next_update: None,
        fetched_at: now(),
        last_ok_at: Some(now()),
        status: sequent_core::signing::CrlStatus::Ok,
        last_error: None,
    };
    let current = list(pki_now() - 86_400, 10, vec![]);
    let previous = stored_row(current.clone());
    // The same list again, or a newer one, is accepted.
    assert!(check_crl(&current, &ca.cert, Some(&previous), now()).is_ok());
    assert!(check_crl(
        &list(pki_now() - 3600, 11, vec![]),
        &ca.cert,
        Some(&previous),
        now()
    )
    .is_ok());
    for (bytes, error) in [
        (
            list(pki_now() - 2 * 86_400, 11, vec![]),
            "the list is older than the stored one",
        ),
        (
            list(pki_now() - 3600, 9, vec![]),
            "the list's number is lower than the stored one's",
        ),
        (
            list(pki_now() + 3600, 12, vec![]),
            "the list is dated in the future",
        ),
        (
            list(pki_now() - 3600, 12, vec![delta_indicator(10)]),
            "a delta list is not accepted",
        ),
    ] {
        assert_eq!(
            check_crl(&bytes, &ca.cert, Some(&previous), now())
                .unwrap_err()
                .to_string(),
            error
        );
    }
    // Without a next update a list is current for seven days.
    let open_ended = list_scope(&crl_with(
        ca,
        &ca.key,
        &[],
        pki_now() - 6 * 86_400,
        None,
        &[],
    ))
    .unwrap();
    assert!(open_ended.is_current(now()));
    assert!(!open_ended.is_current(now() + Duration::days(2)));
}
