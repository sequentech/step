// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Staff certificates against the database: the OpenSSL verifier with the
//! event's staff issuers, lists and registrations; first-use and Security
//! Officer registration; key-level revocation and the requests it cancels;
//! the Certificates settings and their log entries; revocation list
//! refreshes and the dry run; concurrent registrations and revocations on
//! two connections; and that staff issuers never reach the voters'
//! certificate authority queries.
//!
//! Most tests work in one transaction they roll back. The ones that need
//! commits (several connections, refreshes in their own transactions) use a
//! tenant of their own and delete it at the end.

#[path = "support/schema.rs"]
mod schema;
#[path = "support/signing_pki.rs"]
mod signing_pki;

use anyhow::bail;
use async_trait::async_trait;
use chrono::{DateTime, Duration, TimeZone, Utc};
use deadpool_postgres::{Client, Transaction};
use sequent_core::signing::{
    CancelReason, CertificateCheckId, CertificatePostBinding, CertificateRegistration, CrlStatus,
    CrlUnavailablePolicy, RevocationStatus, SignatureAlgorithm, SigningAction, SigningChecks,
    SigningRequestStatus, StaffCertificateRegistration, StaffCertificateStatus,
};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use signing_pki::{issued, pki_now, rsa_key, Issued, Pki, Spec, Usage, CRL_URL, ROOT_CRL_URL};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use windmill::postgres::certificate_authority::{
    delete_certificate_authorities, get_certificate_authorities_pem,
    get_certificate_authorities_pem_by_ids, insert_certificate_authority,
    CertificateAuthorityRecord,
};
use windmill::postgres::signing::{
    get_signing_checks, get_signing_request, insert_signing_approval, insert_signing_request,
    list_signing_approvals, lock_signing_event, NewSigningApproval, NewSigningRequest,
    SigningRequestRow, StaffCertificateRow,
};
use windmill::postgres::signing_certificates::{list_staff_crls, CrlDownload};
use windmill::services::certificate_authority::parse_certificate_pem;
use windmill::services::signing::certificates::{
    holds_signing_event_lock, load_staff_anchors, register_first_use, CertificateIdentity,
    CertificateVerification, CertificateVerificationInput, CertificateVerifier,
    OpensslCertificateVerifier, RegistrationState, SignatureInputs,
};
use windmill::services::signing::crl::{
    distribution_points, download, plan_event_crls, refresh_chain_crls, refresh_event_crls,
    store_download, CrlFetcher, CrlTarget,
};
use windmill::services::signing::directory::{display_name, display_names_in_realm, UserDirectory};
use windmill::services::signing::issuers::{
    import_staff_issuers, remove_staff_issuer, update_signing_checks,
};
use windmill::services::signing::log::Actor;
use windmill::services::signing::staff_certificates::{
    check_certificate, my_staff_certificates, register_staff_certificate, revoke_registration,
    RegistrationRefusalReason, RevokeOutcome, StaffCertificateRegistrationInput,
};
use windmill::services::signing::Allowance;
use windmill::tasks::refresh_staff_crls::refresh_all_staff_crls;

#[derive(Clone, Copy)]
struct Scope {
    tenant: Uuid,
    event: Uuid,
    post_a: Uuid,
    post_b: Uuid,
}

fn id(seed: u32, n: u32) -> Uuid {
    Uuid::parse_str(&format!("{seed:08x}-0000-4000-8000-{n:012x}")).unwrap()
}

fn now() -> DateTime<Utc> {
    Utc.timestamp_opt(pki_now(), 0).unwrap()
}

fn actor(user: &str) -> Actor {
    Actor {
        user_id: user.to_owned(),
        username: format!("{user}-name"),
    }
}

async fn tenant(tx: &Transaction<'_>, tenant: Uuid) {
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &format!("tenant-{tenant}")],
    )
    .await
    .unwrap();
}

async fn event(tx: &Transaction<'_>, tenant: Uuid, event: Uuid) {
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&event, &tenant],
    )
    .await
    .unwrap();
}

async fn election(tx: &Transaction<'_>, tenant: Uuid, event: Uuid, election: Uuid) {
    tx.execute(
        "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id)
         VALUES ($1, $2, $3)",
        &[&election, &tenant, &event],
    )
    .await
    .unwrap();
}

/// A tenant with an event of two Posts.
async fn scope(tx: &Transaction<'_>, seed: u32) -> Scope {
    let s = Scope {
        tenant: id(seed, 1),
        event: id(seed, 2),
        post_a: id(seed, 3),
        post_b: id(seed, 4),
    };
    tenant(tx, s.tenant).await;
    event(tx, s.tenant, s.event).await;
    election(tx, s.tenant, s.event, s.post_a).await;
    election(tx, s.tenant, s.event, s.post_b).await;
    s
}

/// Another event of the same tenant.
async fn second_event(tx: &Transaction<'_>, s: Scope, seed: u32) -> Scope {
    let other = Scope {
        event: id(seed, 12),
        post_a: id(seed, 13),
        post_b: id(seed, 14),
        ..s
    };
    event(tx, s.tenant, other.event).await;
    election(tx, s.tenant, other.event, other.post_a).await;
    election(tx, s.tenant, other.event, other.post_b).await;
    other
}

/// Deletes a committed tenant with its events and their rows.
async fn purge(tx: &Transaction<'_>, tenant: Uuid) {
    for table in ["election", "election_event", "tenant"] {
        let column = if table == "tenant" { "id" } else { "tenant_id" };
        tx.execute(
            &format!("DELETE FROM sequent_backend.{table} WHERE {column} = $1"),
            &[&tenant],
        )
        .await
        .unwrap();
    }
}

/// The current lists of the test root and individual CA.
fn current_lists() -> FakeFetcher {
    let pki = Pki::get();
    FakeFetcher::with(&[
        (CRL_URL, pki.individual_crl()),
        (ROOT_CRL_URL, pki.root_crl()),
    ])
}

/// A target to download `url` from, signed by the staff issuer `name`.
async fn crl_target(tx: &Transaction<'_>, s: Scope, name: &str, url: &str) -> CrlTarget {
    let (issuer, certificate) = load_staff_anchors(tx, s.tenant, s.event)
        .await
        .unwrap()
        .into_iter()
        .find(|(row, _)| row.common_name == name)
        .unwrap();
    let stored = list_staff_crls(tx, s.tenant, s.event)
        .await
        .unwrap()
        .into_iter()
        .find(|row| row.url == url);
    CrlTarget {
        url: url.to_owned(),
        issuer,
        issuer_certificate: certificate,
        stored,
    }
}

/// Downloads and stores `url` in `tx`.
async fn fetch_into(
    tx: &Transaction<'_>,
    s: Scope,
    name: &str,
    url: &str,
    fetcher: &FakeFetcher,
) -> CrlDownload {
    let target = crl_target(tx, s, name, url).await;
    let outcome = download(&target, fetcher, now()).await;
    store_download(tx, &target, &outcome).await.unwrap();
    outcome
}

/// Trusts the test root and individual CA, and stores their current lists.
async fn trust(tx: &Transaction<'_>, s: Scope) {
    let pki = Pki::get();
    let import = import_staff_issuers(
        tx,
        s.tenant,
        s.event,
        &[pki.root.cert.clone(), pki.individual_ca.cert.clone()],
        &actor("officer"),
        Allowance::Permission(Permissions::SIGNING_ISSUERS_WRITE),
        now(),
    )
    .await
    .unwrap();
    assert_eq!(import.imported.len(), 2, "{:?}", import.errors);
    let lists = current_lists();
    fetch_into(tx, s, "Test Staff Individual CA", CRL_URL, &lists).await;
    fetch_into(tx, s, "Test Staff Root CA", ROOT_CRL_URL, &lists).await;
}

async fn new_request(
    tx: &Transaction<'_>,
    s: Scope,
    action: SigningAction,
    post: Option<Uuid>,
) -> SigningRequestRow {
    let payload = format!("{{\"action\":\"{action}\",\"n\":\"{}\"}}", Uuid::new_v4());
    insert_signing_request(
        tx,
        &NewSigningRequest {
            id: Uuid::new_v4(),
            tenant_id: s.tenant,
            election_event_id: s.event,
            action,
            election_id: post,
            area_id: None,
            trustee_id: None,
            scope_key: format!("{post:?}-{}", Uuid::new_v4()),
            subject: json!({}),
            payload_sha256: hex::encode(Sha256::digest(payload.as_bytes())),
            canonical_payload: payload,
            document_id: None,
            document_sha256: None,
            code: "7F3A-91C2".to_owned(),
            config_revision: None,
            rule_revision: 1,
            rule_snapshot: json!({}),
            required: 2,
            requested_by: "requester".to_owned(),
            requested_by_username: "requester-name".to_owned(),
            requested_by_name: None,
            expires_at: None,
            permission_label: None,
            created_at: now(),
        },
    )
    .await
    .unwrap()
    .unwrap()
}

fn input<'a>(
    request: &'a SigningRequestRow,
    user: &str,
    signer: &Issued,
) -> CertificateVerificationInput<'a> {
    let algorithm = CertificateIdentity::of(&signer.cert, &[])
        .unwrap()
        .key_algorithm
        .unwrap();
    CertificateVerificationInput {
        request,
        signer: actor(user),
        chain_pem: vec![signer.pem()],
        signatures: Some(SignatureInputs {
            algorithm,
            payload_signature: signer.sign(request.canonical_payload.as_bytes()),
            document: None,
        }),
        now: now(),
    }
}

/// Display names by user id: "Display <id>".
struct FakeDirectory;

#[async_trait]
impl UserDirectory for FakeDirectory {
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

fn verifier() -> OpensslCertificateVerifier {
    OpensslCertificateVerifier::with_directory(Arc::new(FakeDirectory))
}

async fn verify(
    tx: &Transaction<'_>,
    input: &CertificateVerificationInput<'_>,
) -> CertificateVerification {
    verifier().verify(tx, input).await.unwrap()
}

fn refusal(verification: &CertificateVerification) -> Option<CertificateCheckId> {
    verification.refusal().map(|check| check.id)
}

/// Registers on first use as the approval does: under the event's signing
/// lock.
async fn first_use(
    tx: &Transaction<'_>,
    input: &CertificateVerificationInput<'_>,
    verification: &CertificateVerification,
) -> anyhow::Result<StaffCertificateRow> {
    lock_signing_event(tx, input.request.tenant_id, input.request.election_event_id).await?;
    register_first_use(tx, input, verification, Some("Display name")).await
}

/// Verifies and registers `signer` to `user` on first use.
async fn register_on_first_use(
    tx: &Transaction<'_>,
    request: &SigningRequestRow,
    user: &str,
    signer: &Issued,
) -> StaffCertificateRow {
    let input = input(request, user, signer);
    let verification = verify(tx, &input).await;
    assert!(verification.passed(), "{:?}", verification.checks);
    first_use(tx, &input, &verification).await.unwrap()
}

/// Records `row`'s certificate signing `request` for its user.
async fn sign(tx: &Transaction<'_>, request: &SigningRequestRow, row: &StaffCertificateRow) {
    insert_signing_approval(
        tx,
        &NewSigningApproval {
            tenant_id: request.tenant_id,
            election_event_id: request.election_event_id,
            request_id: request.id,
            user_id: row.user_id.clone(),
            username: row.username.clone(),
            display_name: None,
            auth_time: None,
            certificate_id: row.id,
            certificate_pem: row.pem.clone(),
            chain_pem: row.pem.clone(),
            fingerprint_sha256: row.fingerprint_sha256.clone(),
            spki_sha256: row.spki_sha256.clone(),
            holder_sha256: row.holder_sha256.clone(),
            algorithm: SignatureAlgorithm::RsaPkcs1Sha256,
            payload_signature: vec![1],
            document_signature: None,
            pdf_cms: None,
            revocation_status: RevocationStatus::Checked,
        },
    )
    .await
    .unwrap()
    .unwrap();
}

type Entry = (String, String, String, Option<String>, String);

/// The (statement kind, event type, log type, user, description) of the
/// event's outbox rows, in order.
async fn outbox(tx: &Transaction<'_>, s: Scope) -> Vec<Entry> {
    tx.query(
        "SELECT statement_kind, event_type, log_type, user_id, body->>'description'
         FROM sequent_backend.signing_log_outbox
         WHERE tenant_id = $1 AND election_event_id = $2 ORDER BY id",
        &[&s.tenant, &s.event],
    )
    .await
    .unwrap()
    .iter()
    .map(|row| (row.get(0), row.get(1), row.get(2), row.get(3), row.get(4)))
    .collect()
}

async fn outbox_details(tx: &Transaction<'_>, s: Scope, kind: &str) -> Vec<Value> {
    tx.query(
        "SELECT body->'details' FROM sequent_backend.signing_log_outbox
         WHERE tenant_id = $1 AND election_event_id = $2 AND statement_kind = $3 AND entry = 0
         ORDER BY id",
        &[&s.tenant, &s.event, &kind],
    )
    .await
    .unwrap()
    .iter()
    .map(|row| row.get(0))
    .collect()
}

/// A step's two entries: USER for `user`, SYSTEM INFO.
fn step(kind: &str, user: &str, description: &str) -> [Entry; 2] {
    [
        (
            kind.to_owned(),
            "USER".to_owned(),
            "INFO".to_owned(),
            Some(user.to_owned()),
            description.to_owned(),
        ),
        (
            kind.to_owned(),
            "SYSTEM".to_owned(),
            "INFO".to_owned(),
            None,
            description.to_owned(),
        ),
    ]
}

/// Serves revocation lists by URL; any other URL fails.
#[derive(Default)]
struct FakeFetcher {
    lists: Mutex<HashMap<String, Vec<u8>>>,
    fetched: Mutex<Vec<String>>,
}

impl FakeFetcher {
    fn with(lists: &[(&str, Vec<u8>)]) -> Self {
        let fetcher = FakeFetcher::default();
        for (url, der) in lists {
            fetcher
                .lists
                .lock()
                .unwrap()
                .insert(url.to_string(), der.clone());
        }
        fetcher
    }

    fn fetched(&self) -> Vec<String> {
        self.fetched.lock().unwrap().clone()
    }
}

#[async_trait]
impl CrlFetcher for FakeFetcher {
    async fn fetch(&self, url: &str) -> anyhow::Result<Vec<u8>> {
        self.fetched.lock().unwrap().push(url.to_owned());
        match self.lists.lock().unwrap().get(url) {
            Some(der) => Ok(der.clone()),
            None => bail!("unreachable"),
        }
    }
}

fn voter_record(s: Scope, issued: &Issued) -> CertificateAuthorityRecord {
    let parsed = parse_certificate_pem(&issued.pem()).unwrap();
    CertificateAuthorityRecord {
        id: Uuid::new_v4(),
        tenant_id: s.tenant,
        election_event_id: s.event,
        common_name: parsed.common_name,
        subject: parsed.subject,
        issuer_common_name: parsed.issuer_common_name,
        issuer: parsed.issuer,
        not_before: parsed.not_before,
        not_after: parsed.not_after,
        fingerprint_sha256: parsed.fingerprint_sha256,
        serial_number: parsed.serial_number,
        pem: parsed.pem,
    }
}

fn registration_input(
    s: Scope,
    holder: &str,
    signer: &Issued,
    linked_to: Option<&str>,
) -> StaffCertificateRegistrationInput {
    StaffCertificateRegistrationInput {
        tenant_id: s.tenant,
        election_event_id: s.event,
        holder: actor(holder),
        holder_name: Some(format!("Display {holder}")),
        election_id: None,
        chain_pem: vec![signer.pem()],
        linked_to: linked_to.map(str::to_owned),
    }
}

async fn register_by_officer(
    tx: &Transaction<'_>,
    input: &StaffCertificateRegistrationInput,
) -> Result<StaffCertificateRow, windmill::services::signing::staff_certificates::RegistrationRefused>
{
    register_staff_certificate(tx, input, &actor("officer"), Some("Olivia Officer"), now())
        .await
        .unwrap()
}

/// A new connection of its own.
async fn connection() -> Client {
    schema::pool().await.get().await.unwrap()
}

#[tokio::test]
async fn staff_issuers_stay_apart_from_the_voters_authorities() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let pki = Pki::get();
    let voter = voter_record(s, &pki.root);
    let voter_id = voter.id;
    assert!(insert_certificate_authority(&tx, voter).await.unwrap());
    let request = new_request(&tx, s, SigningAction::CloseVoting, Some(s.post_a)).await;

    // A voter authority is no staff issuer.
    let verification = verify(&tx, &input(&request, "maria", &pki.maria)).await;
    assert_eq!(
        refusal(&verification),
        Some(CertificateCheckId::TrustedIssuer)
    );

    // The same root as a staff issuer, besides the voter one.
    trust(&tx, s).await;
    let verification = verify(&tx, &input(&request, "maria", &pki.maria)).await;
    assert!(verification.passed(), "{:?}", verification.checks);

    // The unauthenticated PEM route's query, the export query and the voter
    // delete see only the voter authority.
    let voter_pem = pki.root.pem();
    assert_eq!(
        get_certificate_authorities_pem(&tx, s.event).await.unwrap(),
        vec![voter_pem.clone()]
    );
    let staff_ids: Vec<Uuid> = tx
        .query(
            "SELECT id FROM sequent_backend.certificate_authority
             WHERE election_event_id = $1 AND purpose = 'staff-signatures'",
            &[&s.event],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| row.get(0))
        .collect();
    assert_eq!(staff_ids.len(), 2);
    assert!(
        get_certificate_authorities_pem_by_ids(&tx, s.event, &staff_ids)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        get_certificate_authorities_pem_by_ids(&tx, s.event, &[])
            .await
            .unwrap(),
        vec![voter_pem]
    );
    let deleted = delete_certificate_authorities(&tx, &staff_ids, s.event, s.tenant)
        .await
        .unwrap();
    assert!(deleted.is_empty());
    let deleted = delete_certificate_authorities(&tx, &[voter_id], s.event, s.tenant)
        .await
        .unwrap();
    assert_eq!(deleted.len(), 1);
    assert!(get_certificate_authorities_pem(&tx, s.event)
        .await
        .unwrap()
        .is_empty());
    // The staff issuers are still trusted.
    assert!(verify(&tx, &input(&request, "maria", &pki.maria))
        .await
        .passed());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn issuers_import_as_authorities_only_and_every_change_is_logged() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let pki = Pki::get();
    let expired_ca = issued(
        &Spec {
            subject: &[("CN", "Old Staff CA")],
            serial: 9,
            not_before: pki_now() - 900 * 86_400,
            not_after: pki_now() - 86_400,
            usage: Usage::Ca,
            crl_url: None,
        },
        rsa_key(),
        None,
    );
    let import = import_staff_issuers(
        &tx,
        s.tenant,
        s.event,
        &[
            pki.root.cert.clone(),
            pki.maria.cert.clone(),
            expired_ca.cert.clone(),
        ],
        &actor("officer"),
        Allowance::Permission(Permissions::SIGNING_ISSUERS_WRITE),
        now(),
    )
    .await
    .unwrap();
    assert_eq!(import.imported.len(), 1);
    assert_eq!(import.imported[0].common_name, "Test Staff Root CA");
    assert_eq!(
        import.errors,
        vec![
            "Certificate 2: Maria Santos is not a certificate authority".to_owned(),
            format!(
                "Certificate 3: expired on {}",
                Utc.timestamp_opt(pki_now() - 86_400, 0)
                    .unwrap()
                    .format("%Y-%m-%d")
            ),
        ]
    );
    // Importing it again skips it and logs nothing.
    let again = import_staff_issuers(
        &tx,
        s.tenant,
        s.event,
        &[pki.root.cert.clone()],
        &actor("officer"),
        Allowance::Permission(Permissions::SIGNING_ISSUERS_WRITE),
        now(),
    )
    .await
    .unwrap();
    assert!(again.imported.is_empty());
    assert_eq!(again.skipped, vec![import.imported[0].subject.clone()]);

    let root_id = import.imported[0].id;
    let removed = remove_staff_issuer(&tx, s.tenant, s.event, root_id, &actor("officer"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(removed.id, root_id);
    assert!(
        remove_staff_issuer(&tx, s.tenant, s.event, root_id, &actor("officer"))
            .await
            .unwrap()
            .is_none()
    );

    let mut expected = step(
        "SigningIssuerChanged",
        "officer",
        "Imported trusted issuer Test Staff Root CA",
    )
    .to_vec();
    expected.extend(step(
        "SigningIssuerChanged",
        "officer",
        "Removed trusted issuer Test Staff Root CA",
    ));
    assert_eq!(outbox(&tx, s).await, expected);
    let details = outbox_details(&tx, s, "SigningIssuerChanged").await;
    let fingerprint = hex::encode(Sha256::digest(pki.root.der()));
    assert_eq!(details[0]["change"], "imported");
    assert_eq!(
        details[0]["issuers"][0]["fingerprint"],
        fingerprint.as_str()
    );
    assert_eq!(details[1]["change"], "removed");
    for entry in &details {
        assert_eq!(entry["allowed_by"], json!(["signing-issuers-write"]));
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn checks_save_on_their_revision_and_log_what_changed() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let changed = SigningChecks {
        registration: CertificateRegistration::SecurityOfficerOnly,
        crl_unavailable: CrlUnavailablePolicy::AcceptUnchecked,
        ..SigningChecks::default()
    };
    let row = update_signing_checks(
        &tx,
        s.tenant,
        s.event,
        &changed,
        0,
        &actor("officer"),
        Some("Olivia Officer"),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(row.checks.revision, 1);
    assert_eq!(row.updated_by_name.as_deref(), Some("Olivia Officer"));
    assert_eq!(
        row.checks.registration,
        CertificateRegistration::SecurityOfficerOnly
    );
    // A save on a stale revision changes nothing.
    assert!(update_signing_checks(
        &tx,
        s.tenant,
        s.event,
        &SigningChecks::default(),
        0,
        &actor("officer"),
        Some("Olivia Officer")
    )
    .await
    .unwrap()
    .is_none());
    let unchanged = update_signing_checks(
        &tx,
        s.tenant,
        s.event,
        &changed,
        1,
        &actor("officer"),
        Some("Olivia Officer"),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(unchanged.checks.revision, 2);
    assert_eq!(
        get_signing_checks(&tx, s.tenant, s.event)
            .await
            .unwrap()
            .unwrap()
            .checks,
        unchanged.checks
    );
    let mut expected = step(
        "SigningChecksChanged",
        "officer",
        "Changed the certificate checks: when a list can't be downloaded: refuse → accept-unchecked; registration: on-first-use → security-officer-only",
    )
    .to_vec();
    expected.extend(step(
        "SigningChecksChanged",
        "officer",
        "Saved the certificate checks unchanged (revision 2)",
    ));
    assert_eq!(outbox(&tx, s).await, expected);
    let details = outbox_details(&tx, s, "SigningChecksChanged").await;
    assert_eq!(details[0]["before"]["registration"], "on-first-use");
    assert_eq!(details[0]["after"]["registration"], "security-officer-only");
    for entry in &details {
        assert_eq!(entry["allowed_by"], json!(["signing-checks-write"]));
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn first_use_registers_the_signer_and_binds_key_and_holder_tenant_wide() {
    let mut client = connection().await;
    let tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    trust(&tx, s).await;
    let pki = Pki::get();
    let close_a = new_request(&tx, s, SigningAction::CloseVoting, Some(s.post_a)).await;

    let maria = input(&close_a, "maria", &pki.maria);
    let verification = verify(&tx, &maria).await;
    assert!(verification.passed(), "{:?}", verification.checks);
    assert_eq!(verification.registration, RegistrationState::FirstUse);
    assert_eq!(verification.revocation_status, RevocationStatus::Checked);
    // The chain kept is the verified path.
    assert_eq!(
        verification.certificate.as_ref().unwrap().chain_pem,
        format!("{}{}", pki.maria.pem(), pki.individual_ca.pem())
    );
    // Registering needs the event's signing lock: a transaction on another
    // connection holds none.
    {
        let mut other = connection().await;
        let unlocked = other.transaction().await.unwrap();
        assert!(!holds_signing_event_lock(&unlocked, s.tenant, s.event)
            .await
            .unwrap());
        let error = register_first_use(&unlocked, &maria, &verification, None)
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "registering a certificate needs the event's signing lock"
        );
        unlocked.rollback().await.unwrap();
    }
    // This transaction holds it since it imported the issuers.
    assert!(holds_signing_event_lock(&tx, s.tenant, s.event)
        .await
        .unwrap());
    let row = first_use(&tx, &maria, &verification).await.unwrap();
    assert_eq!(row.user_id, "maria");
    assert_eq!(row.election_id, Some(s.post_a));
    assert_eq!(row.registration, StaffCertificateRegistration::FirstUse);
    assert_eq!(row.user_display_name.as_deref(), Some("Display name"));
    assert_eq!(row.registered_by_name.as_deref(), Some("Display name"));
    // A passed first-use verification is needed.
    let again = verify(&tx, &maria).await;
    assert_eq!(
        again.registration,
        RegistrationState::Registered(row.clone())
    );
    assert!(first_use(&tx, &maria, &again).await.is_err());

    // A first-use verification made before (or by a fake verifier) doesn't
    // register her certificate or a reissue of her key to Bob: registering
    // checks again under the locks.
    for signer in [&pki.maria, &pki.maria_reissue] {
        let bob = input(&close_a, "bob", signer);
        let mut stale = verification.clone();
        stale.certificate = Some(CertificateIdentity::of(&signer.cert, &[]).unwrap());
        let error = first_use(&tx, &bob, &stale).await.unwrap_err();
        assert_eq!(
            error.to_string(),
            "the certificate's key or holder is registered to another account"
        );
    }

    let entries = outbox(&tx, s).await;
    let description = format!(
        "Registered certificate Maria Santos ({}) to Display name on first use",
        &row.fingerprint_sha256[..16]
    );
    assert_eq!(
        &entries[entries.len() - 2..],
        step("SigningCertificateRegistered", "maria", &description)
    );
    let details = outbox_details(&tx, s, "SigningCertificateRegistered").await;
    assert_eq!(details[0]["request_id"], json!(close_a.id));
    assert_eq!(
        details[0]["certificate"]["fingerprint"],
        json!(row.fingerprint_sha256)
    );
    // First use registers it as signing the request allows.
    assert_eq!(
        details[0]["allowed_by"],
        json!([close_a.action.sign_permission().to_string()])
    );

    // Bob can't use her certificate or a reissue of her key, here or in
    // another event of the tenant; the refusal names her.
    for signer in [&pki.maria, &pki.maria_reissue] {
        let verification = verify(&tx, &input(&close_a, "bob", signer)).await;
        assert_eq!(
            refusal(&verification),
            Some(CertificateCheckId::RegisteredToOther)
        );
        assert_eq!(
            verification
                .check(CertificateCheckId::RegisteredToOther)
                .unwrap()
                .detail
                .as_deref(),
            Some("Display maria")
        );
        let holder = verification.other_holder.as_ref().unwrap();
        assert_eq!(
            (holder.user_id.as_str(), holder.display_name.as_str()),
            ("maria", "Display maria")
        );
    }
    let other = second_event(&tx, s, line!()).await;
    trust(&tx, other).await;
    let elsewhere = new_request(&tx, other, SigningAction::CloseVoting, Some(other.post_a)).await;
    let verification = verify(&tx, &input(&elsewhere, "bob", &pki.maria)).await;
    assert_eq!(
        refusal(&verification),
        Some(CertificateCheckId::RegisteredToOther)
    );
    // Maria herself registers it in the other event on first use.
    let verification = verify(&tx, &input(&elsewhere, "maria", &pki.maria)).await;
    assert_eq!(verification.registration, RegistrationState::FirstUse);

    let mine = my_staff_certificates(&tx, s.tenant, s.event, "maria")
        .await
        .unwrap();
    assert_eq!(mine, vec![row]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn after_a_link_first_use_needs_a_security_officer_in_every_event() {
    let mut client = connection().await;
    let tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    trust(&tx, s).await;
    let pki = Pki::get();
    register_by_officer(&tx, &registration_input(s, "maria", &pki.maria, None))
        .await
        .unwrap();
    register_by_officer(
        &tx,
        &registration_input(s, "maria-trustee", &pki.maria, Some("maria")),
    )
    .await
    .unwrap();
    let other = second_event(&tx, s, line!()).await;
    trust(&tx, other).await;
    for (scope, user) in [
        // The same certificate in the other event, by either account.
        (other, "maria"),
        (other, "maria-trustee"),
    ] {
        let request = new_request(&tx, scope, SigningAction::ApproveConfiguration, None).await;
        let input = input(&request, user, &pki.maria);
        let verification = verify(&tx, &input).await;
        assert_eq!(
            refusal(&verification),
            Some(CertificateCheckId::Registered),
            "{user}"
        );
        assert_eq!(verification.registration, RegistrationState::NotRegistered);
        // A typed refusal, never an insert that the unique index refuses.
        assert!(first_use(&tx, &input, &verification).await.is_err());
    }
    // In the event of the link both accounts sign.
    let here = new_request(&tx, s, SigningAction::ApproveConfiguration, None).await;
    for user in ["maria", "maria-trustee"] {
        assert!(verify(&tx, &input(&here, user, &pki.maria)).await.passed());
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_security_officer_links_a_persons_certificate_to_their_second_account_only() {
    let mut client = connection().await;
    let tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    trust(&tx, s).await;
    let pki = Pki::get();
    let maria = register_by_officer(&tx, &registration_input(s, "maria", &pki.maria, None))
        .await
        .unwrap();
    assert_eq!(
        maria.registration,
        StaffCertificateRegistration::SecurityOfficer
    );
    assert_eq!(maria.registered_by, "officer");
    assert_eq!(maria.registered_by_name.as_deref(), Some("Olivia Officer"));
    assert_eq!(maria.user_display_name.as_deref(), Some("Display maria"));

    // Without naming the first account, or naming another, it is refused;
    // the refusal names the account holding it.
    for linked_to in [None, Some("carol")] {
        let refused = register_by_officer(
            &tx,
            &registration_input(s, "maria-trustee", &pki.maria, linked_to),
        )
        .await
        .unwrap_err();
        assert_eq!(refused.reason, RegistrationRefusalReason::RegisteredToOther);
        assert_eq!(refused.detail, "Display maria");
        assert_eq!(refused.other_holder.unwrap().user_id, "maria");
    }
    let linked = register_by_officer(
        &tx,
        &registration_input(s, "maria-trustee", &pki.maria, Some("maria")),
    )
    .await
    .unwrap();
    assert_eq!(linked.linked_to.as_deref(), Some("maria"));

    // Both accounts sign with it; a third account is refused.
    let request = new_request(&tx, s, SigningAction::ApproveConfiguration, None).await;
    for user in ["maria", "maria-trustee"] {
        let verification = verify(&tx, &input(&request, user, &pki.maria)).await;
        assert!(verification.passed(), "{user}: {:?}", verification.checks);
    }
    let verification = verify(&tx, &input(&request, "bob", &pki.maria)).await;
    assert_eq!(
        refusal(&verification),
        Some(CertificateCheckId::RegisteredToOther)
    );

    for (input, reason) in [
        (
            registration_input(s, "ana", &pki.ana, Some("maria")),
            RegistrationRefusalReason::NothingToLink,
        ),
        (
            registration_input(s, "maria", &pki.maria, None),
            RegistrationRefusalReason::AlreadyRegistered,
        ),
        (
            registration_input(s, "ca", &pki.individual_ca, None),
            RegistrationRefusalReason::UntrustedIssuer,
        ),
        (
            registration_input(s, "rosa", &pki.foreign_signer, None),
            RegistrationRefusalReason::UntrustedIssuer,
        ),
        (
            registration_input(s, "ramon", &pki.expired, None),
            RegistrationRefusalReason::NotValidNow,
        ),
        (
            registration_input(s, "pedro", &pki.no_signing_usage, None),
            RegistrationRefusalReason::NotForSigning,
        ),
        (
            StaffCertificateRegistrationInput {
                election_id: Some(Uuid::new_v4()),
                ..registration_input(s, "ana", &pki.ana, None)
            },
            RegistrationRefusalReason::UnknownPost,
        ),
        (
            StaffCertificateRegistrationInput {
                chain_pem: vec!["garbage".to_owned()],
                ..registration_input(s, "ana", &pki.ana, None)
            },
            RegistrationRefusalReason::Unreadable,
        ),
    ] {
        let refused = register_by_officer(&tx, &input).await.unwrap_err();
        assert_eq!(refused.reason, reason, "{}", refused.detail);
    }

    let short = &maria.fingerprint_sha256[..16];
    let registered: Vec<Entry> = outbox(&tx, s)
        .await
        .into_iter()
        .filter(|entry| entry.0 == "SigningCertificateRegistered")
        .collect();
    let mut expected = step(
        "SigningCertificateRegistered",
        "officer",
        &format!("Registered certificate Maria Santos ({short}) to Display maria"),
    )
    .to_vec();
    expected.extend(step(
        "SigningCertificateRegistered",
        "officer",
        &format!(
            "Linked certificate Maria Santos ({short}) of Display maria to Display maria-trustee"
        ),
    ));
    assert_eq!(registered, expected);
    let details = outbox_details(&tx, s, "SigningCertificateRegistered").await;
    assert_eq!(details[1]["linked_to"], "maria");
    assert_eq!(details[1]["registration"], "security-officer");
    for entry in &details {
        assert_eq!(
            entry["allowed_by"],
            json!(["signing-certificates-register"])
        );
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn security_officer_only_registration_refuses_first_use() {
    let mut client = connection().await;
    let tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    trust(&tx, s).await;
    let pki = Pki::get();
    let checks = SigningChecks {
        registration: CertificateRegistration::SecurityOfficerOnly,
        ..SigningChecks::default()
    };
    update_signing_checks(&tx, s.tenant, s.event, &checks, 0, &actor("officer"), None)
        .await
        .unwrap()
        .unwrap();
    let request = new_request(&tx, s, SigningAction::OpenVoting, Some(s.post_a)).await;
    let verification = verify(&tx, &input(&request, "jose", &pki.jose)).await;
    assert_eq!(refusal(&verification), Some(CertificateCheckId::Registered));
    assert_eq!(verification.registration, RegistrationState::NotRegistered);

    let mut registration = registration_input(s, "jose", &pki.jose, None);
    registration.election_id = Some(s.post_a);
    register_by_officer(&tx, &registration).await.unwrap();
    let verification = verify(&tx, &input(&request, "jose", &pki.jose)).await;
    assert!(verification.passed(), "{:?}", verification.checks);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn one_post_binds_a_key_on_its_first_post_scoped_signature() {
    let mut client = connection().await;
    let tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    trust(&tx, s).await;
    let pki = Pki::get();
    // An event-level first signature binds no Post.
    let configuration = new_request(&tx, s, SigningAction::ApproveConfiguration, None).await;
    let maria = register_on_first_use(&tx, &configuration, "maria", &pki.maria).await;
    assert_eq!(maria.election_id, None);
    sign(&tx, &configuration, &maria).await;
    let close_a = new_request(&tx, s, SigningAction::CloseVoting, Some(s.post_a)).await;
    let close_b = new_request(&tx, s, SigningAction::CloseVoting, Some(s.post_b)).await;
    for request in [&close_a, &close_b] {
        assert!(verify(&tx, &input(request, "maria", &pki.maria))
            .await
            .passed());
    }
    // Signing for Post A binds the key: not Post B, with any of its
    // certificates.
    sign(&tx, &close_a, &maria).await;
    let open_a = new_request(&tx, s, SigningAction::OpenVoting, Some(s.post_a)).await;
    assert!(verify(&tx, &input(&open_a, "maria", &pki.maria))
        .await
        .passed());
    for signer in [&pki.maria, &pki.maria_reissue] {
        let verification = verify(&tx, &input(&close_b, "maria", signer)).await;
        assert_eq!(
            refusal(&verification),
            Some(CertificateCheckId::PostBinding)
        );
    }
    // Any Post: allowed.
    let any_post = SigningChecks {
        post_binding: CertificatePostBinding::AnyPost,
        ..SigningChecks::default()
    };
    update_signing_checks(
        &tx,
        s.tenant,
        s.event,
        &any_post,
        0,
        &actor("officer"),
        None,
    )
    .await
    .unwrap()
    .unwrap();
    assert!(verify(&tx, &input(&close_b, "maria", &pki.maria))
        .await
        .passed());
    tx.rollback().await.unwrap();

    // A Security Officer's registration for no Post: the first Post-scoped
    // signature binds it.
    let tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    trust(&tx, s).await;
    let jose = register_by_officer(&tx, &registration_input(s, "jose", &pki.jose, None))
        .await
        .unwrap();
    let close_a = new_request(&tx, s, SigningAction::CloseVoting, Some(s.post_a)).await;
    let close_b = new_request(&tx, s, SigningAction::CloseVoting, Some(s.post_b)).await;
    assert!(verify(&tx, &input(&close_b, "jose", &pki.jose))
        .await
        .passed());
    sign(&tx, &close_a, &jose).await;
    let verification = verify(&tx, &input(&close_b, "jose", &pki.jose)).await;
    assert_eq!(
        refusal(&verification),
        Some(CertificateCheckId::PostBinding)
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_revocation_is_of_the_key_everywhere_and_cancels_what_it_signed() {
    let mut client = connection().await;
    let tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    trust(&tx, s).await;
    let other = second_event(&tx, s, line!()).await;
    trust(&tx, other).await;
    let pki = Pki::get();
    // Maria's key: her certificate here, its reissue in the other event;
    // each signed a waiting request.
    let here = new_request(&tx, s, SigningAction::CloseVoting, Some(s.post_a)).await;
    let first = input(&here, "maria", &pki.maria);
    let first_verification = verify(&tx, &first).await;
    let row = first_use(&tx, &first, &first_verification).await.unwrap();
    sign(&tx, &here, &row).await;
    let there = new_request(&tx, other, SigningAction::CloseVoting, Some(other.post_a)).await;
    let reissue = register_on_first_use(&tx, &there, "maria", &pki.maria_reissue).await;
    sign(&tx, &there, &reissue).await;
    let untouched = new_request(&tx, s, SigningAction::OpenVoting, Some(s.post_a)).await;

    let officer = actor("officer");
    let name = Some("Olivia Officer");
    assert!(
        revoke_registration(&tx, s.tenant, s.event, row.id, "  ", &officer, name)
            .await
            .is_err()
    );
    let RevokeOutcome::Revoked {
        certificate,
        revoked,
        cancelled_requests,
    } = revoke_registration(
        &tx,
        s.tenant,
        s.event,
        row.id,
        " token lost ",
        &officer,
        name,
    )
    .await
    .unwrap()
    else {
        panic!("not revoked");
    };
    assert_eq!(certificate.status, StaffCertificateStatus::Revoked);
    assert_eq!(certificate.revoke_reason.as_deref(), Some("token lost"));
    assert_eq!(
        certificate.revoked_by_name.as_deref(),
        Some("Olivia Officer")
    );
    let mut ids: Vec<Uuid> = revoked.iter().map(|row| row.id).collect();
    ids.sort();
    let mut expected = vec![row.id, reissue.id];
    expected.sort();
    assert_eq!(ids, expected);
    let mut cancelled: Vec<Uuid> = cancelled_requests
        .iter()
        .map(|request| request.id)
        .collect();
    cancelled.sort();
    let mut expected = vec![here.id, there.id];
    expected.sort();
    assert_eq!(cancelled, expected);
    for (scope, request, signed) in [(s, &here, &row), (other, &there, &reissue)] {
        // Its signature no longer counts; the revocation names what allowed it.
        let approvals = list_signing_approvals(&tx, scope.tenant, scope.event, request.id)
            .await
            .unwrap();
        let cancelled = outbox_details(&tx, scope, "SigningRequestCancelled").await;
        assert_eq!(
            cancelled.last().unwrap()["voided_approvals"],
            json!([{
                "approval_id": approvals[0].id,
                "user_id": "maria",
                "username": signed.username,
                "fingerprint": signed.fingerprint_sha256,
            }])
        );
        assert_eq!(
            outbox_details(&tx, scope, "SigningCertificateRevoked")
                .await
                .last()
                .unwrap()["allowed_by"],
            json!(["signing-certificates-revoke"])
        );
        let row = get_signing_request(&tx, scope.tenant, scope.event, request.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.status, SigningRequestStatus::Cancelled);
        assert_eq!(row.cancel_reason, Some(CancelReason::CertificateRevoked));
        assert_eq!(row.cancelled_by.as_deref(), Some("officer"));
        // Each event logs the cancellation, then the revocation.
        let entries = outbox(&tx, scope).await;
        let tail = &entries[entries.len() - 4..];
        let cancel = "Cancelled signing request 7F3A-91C2: the certificate Maria Santos that signed it was revoked";
        assert_eq!(
            &tail[..2],
            step("SigningRequestCancelled", "officer", cancel)
        );
        assert_eq!(
            &tail[2..],
            step(
                "SigningCertificateRevoked",
                "officer",
                "Revoked certificate Maria Santos of Display name: token lost"
            )
        );
    }
    let untouched = get_signing_request(&tx, s.tenant, s.event, untouched.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(untouched.status, SigningRequestStatus::Waiting);
    assert!(matches!(
        revoke_registration(&tx, s.tenant, s.event, row.id, "again", &officer, name)
            .await
            .unwrap(),
        RevokeOutcome::AlreadyRevoked(_)
    ));
    assert_eq!(
        revoke_registration(
            &tx,
            s.tenant,
            s.event,
            Uuid::new_v4(),
            "none",
            &officer,
            name
        )
        .await
        .unwrap(),
        RevokeOutcome::NotFound
    );

    // The key never signs again: not with a stale first-use verification,
    // not on first use, not after a Security Officer tries again.
    let error = first_use(&tx, &first, &first_verification)
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "the certificate or its key was revoked");
    let request = new_request(&tx, s, SigningAction::CloseVoting, Some(s.post_a)).await;
    for signer in [&pki.maria, &pki.maria_reissue] {
        let verification = verify(&tx, &input(&request, "maria", signer)).await;
        assert_eq!(refusal(&verification), Some(CertificateCheckId::Registered));
    }
    let refused = register_by_officer(&tx, &registration_input(s, "maria", &pki.maria, None))
        .await
        .unwrap_err();
    assert_eq!(refused.reason, RegistrationRefusalReason::Revoked);
    // A new key of the same person does.
    let configuration = new_request(&tx, s, SigningAction::ApproveConfiguration, None).await;
    assert!(verify(&tx, &input(&configuration, "juan", &pki.juan_a))
        .await
        .passed());
    tx.rollback().await.unwrap();
}

/// A good list's download time and its last good time are one reading of
/// the clock, so they are equal, never a microsecond apart.
#[tokio::test]
async fn a_good_list_is_fetched_and_ok_at_the_same_instant() {
    let mut client = connection().await;
    let tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    trust(&tx, s).await;
    let target = crl_target(&tx, s, "Test Staff Individual CA", CRL_URL).await;
    let outcome = download(&target, &current_lists(), now()).await;
    assert!(matches!(outcome, CrlDownload::Ok { .. }));
    for _ in 0..300 {
        let row = store_download(&tx, &target, &outcome).await.unwrap();
        assert_eq!(row.last_ok_at, Some(row.fetched_at));
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn revocation_lists_refresh_and_never_roll_back() {
    let mut client = connection().await;
    let tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let pki = Pki::get();
    import_staff_issuers(
        &tx,
        s.tenant,
        s.event,
        &[pki.root.cert.clone(), pki.individual_ca.cert.clone()],
        &actor("officer"),
        Allowance::Permission(Permissions::SIGNING_ISSUERS_WRITE),
        now(),
    )
    .await
    .unwrap();
    assert_eq!(
        distribution_points(&pki.maria.cert),
        vec![CRL_URL.to_owned()]
    );
    assert!(distribution_points(&pki.root.cert).is_empty());

    // Only the individual CA names a point (its issuer's list); a
    // registered certificate adds its issuer's.
    let urls = |targets: Vec<CrlTarget>| targets.into_iter().map(|t| t.url).collect::<Vec<_>>();
    assert_eq!(
        urls(plan_event_crls(&tx, s.tenant, s.event).await.unwrap()),
        vec![ROOT_CRL_URL.to_owned()]
    );
    register_by_officer(&tx, &registration_input(s, "maria", &pki.maria, None))
        .await
        .unwrap();
    assert_eq!(
        urls(plan_event_crls(&tx, s.tenant, s.event).await.unwrap()),
        vec![CRL_URL.to_owned(), ROOT_CRL_URL.to_owned()]
    );

    let lists = current_lists();
    let outcome = fetch_into(&tx, s, "Test Staff Individual CA", CRL_URL, &lists).await;
    assert!(matches!(outcome, CrlDownload::Ok { .. }));
    let stored = list_staff_crls(&tx, s.tenant, s.event).await.unwrap();
    let individual = stored
        .iter()
        .find(|row| row.url == CRL_URL)
        .unwrap()
        .clone();
    assert_eq!(individual.status, CrlStatus::Ok);
    assert_eq!(
        individual.issuer_fingerprint,
        hex::encode(Sha256::digest(pki.individual_ca.der()))
    );
    assert_eq!(
        individual.der.as_deref(),
        Some(pki.individual_crl().as_slice())
    );
    assert_eq!(individual.last_ok_at, Some(individual.fetched_at));

    // A failed download, an older list and a list signed by someone else are
    // unavailable; the last good list and its time stay.
    let older = signing_pki::crl_with(
        &pki.individual_ca,
        &pki.individual_ca.key,
        &[],
        pki_now() - 3 * 86_400,
        Some(pki_now() + 86_400),
        &[],
    );
    let forged = signing_pki::crl_signed_by(
        &pki.individual_ca,
        &pki.ana.key,
        &[],
        pki_now(),
        Some(pki_now() + 86_400),
    );
    for (fetcher, error) in [
        (FakeFetcher::default(), "unreachable"),
        (
            FakeFetcher::with(&[(CRL_URL, older)]),
            "the list is older than the stored one",
        ),
        (
            FakeFetcher::with(&[(CRL_URL, forged)]),
            "the list's signature doesn't verify with its issuer's key",
        ),
    ] {
        fetch_into(&tx, s, "Test Staff Individual CA", CRL_URL, &fetcher).await;
        let row = list_staff_crls(&tx, s.tenant, s.event)
            .await
            .unwrap()
            .into_iter()
            .find(|row| row.url == CRL_URL)
            .unwrap();
        assert_eq!(row.status, CrlStatus::Unavailable);
        assert_eq!(row.last_error.as_deref(), Some(error));
        assert_eq!(row.der, individual.der);
        assert_eq!(row.last_ok_at, individual.last_ok_at);
    }
    // The not-revoked detail is the last good download, not the last attempt.
    let request = new_request(&tx, s, SigningAction::ApproveConfiguration, None).await;
    let verification = verify(&tx, &input(&request, "maria", &pki.maria)).await;
    assert_eq!(
        verification
            .check(CertificateCheckId::NotRevoked)
            .unwrap()
            .detail,
        individual.last_ok_at.map(|time| time.to_rfc3339())
    );

    // A stored list goes with its issuer.
    remove_staff_issuer(
        &tx,
        s.tenant,
        s.event,
        individual.issuer_id,
        &actor("officer"),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(list_staff_crls(&tx, s.tenant, s.event)
        .await
        .unwrap()
        .iter()
        .all(|row| row.url != CRL_URL));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn an_unavailable_list_is_refused_or_accepted_unchecked_per_the_event() {
    let mut client = connection().await;
    let tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let pki = Pki::get();
    import_staff_issuers(
        &tx,
        s.tenant,
        s.event,
        &[pki.individual_ca.cert.clone()],
        &actor("officer"),
        Allowance::Permission(Permissions::SIGNING_ISSUERS_WRITE),
        now(),
    )
    .await
    .unwrap();
    let request = new_request(&tx, s, SigningAction::CloseVoting, Some(s.post_a)).await;
    let verification = verify(&tx, &input(&request, "maria", &pki.maria)).await;
    assert_eq!(refusal(&verification), Some(CertificateCheckId::NotRevoked));
    assert_eq!(verification.revocation_status, RevocationStatus::Unchecked);

    let accept = SigningChecks {
        crl_unavailable: CrlUnavailablePolicy::AcceptUnchecked,
        ..SigningChecks::default()
    };
    update_signing_checks(&tx, s.tenant, s.event, &accept, 0, &actor("officer"), None)
        .await
        .unwrap()
        .unwrap();
    let verification = verify(&tx, &input(&request, "maria", &pki.maria)).await;
    assert!(verification.passed(), "{:?}", verification.checks);
    assert_eq!(verification.revocation_status, RevocationStatus::Unchecked);
    tx.rollback().await.unwrap();
}

/// `refresh_all_staff_crls` refreshes every committed event with staff
/// issuers, through its own fetcher. The tests that commit such an event
/// take turns with it, so it never downloads (or fails to download) another
/// test's lists while that test checks them.
static COMMITTED: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// A committed tenant with the test issuers (and no lists), and a waiting
/// Post request.
async fn committed_scope(client: &mut Client, seed: u32) -> (Scope, SigningRequestRow) {
    let tx = client.transaction().await.unwrap();
    purge(&tx, id(seed, 1)).await;
    let s = scope(&tx, seed).await;
    let pki = Pki::get();
    import_staff_issuers(
        &tx,
        s.tenant,
        s.event,
        &[pki.root.cert.clone(), pki.individual_ca.cert.clone()],
        &actor("officer"),
        Allowance::Permission(Permissions::SIGNING_ISSUERS_WRITE),
        now(),
    )
    .await
    .unwrap();
    let request = new_request(&tx, s, SigningAction::CloseVoting, Some(s.post_a)).await;
    tx.commit().await.unwrap();
    (s, request)
}

async fn clean_up(client: &mut Client, s: Scope) {
    let tx = client.transaction().await.unwrap();
    purge(&tx, s.tenant).await;
    tx.commit().await.unwrap();
}

#[tokio::test]
async fn the_dry_run_downloads_the_chains_lists_outside_its_transaction_and_backs_off() {
    let _turn = COMMITTED.lock().await;
    let mut client = connection().await;
    let (s, request) = committed_scope(&mut client, line!()).await;
    let pki = Pki::get();
    let dry_run = |fetcher: &'static FakeFetcher, signer: &'static Issued| {
        let request = request.clone();
        async move {
            let mut client = connection().await;
            check_certificate(
                &mut client,
                &verifier(),
                fetcher,
                &request,
                actor("signer"),
                vec![signer.pem()],
                now(),
            )
            .await
            .unwrap()
        }
    };

    // A failed download refuses (by default) and isn't tried again soon.
    let unreachable: &'static FakeFetcher = Box::leak(Box::new(FakeFetcher::default()));
    let verification = dry_run(unreachable, &pki.maria).await;
    assert_eq!(refusal(&verification), Some(CertificateCheckId::NotRevoked));
    assert!(verification.check(CertificateCheckId::Signature).is_none());
    dry_run(unreachable, &pki.maria).await;
    assert_eq!(unreachable.fetched(), vec![CRL_URL.to_owned()]);

    // After the back-off it downloads the list, which then stays fresh.
    {
        let tx = client.transaction().await.unwrap();
        tx.execute(
            "UPDATE sequent_backend.staff_crl SET fetched_at = fetched_at - interval '1 hour'
             WHERE tenant_id = $1",
            &[&s.tenant],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }
    let lists: &'static FakeFetcher = Box::leak(Box::new(current_lists()));
    let verification = dry_run(lists, &pki.maria).await;
    assert!(verification.passed(), "{:?}", verification.checks);
    assert_eq!(verification.revocation_status, RevocationStatus::Checked);
    assert_eq!(verification.registration, RegistrationState::FirstUse);
    let verification = dry_run(lists, &pki.revoked).await;
    assert_eq!(refusal(&verification), Some(CertificateCheckId::NotRevoked));
    assert_eq!(lists.fetched(), vec![CRL_URL.to_owned()]);

    // Nothing was registered; an untrusted chain downloads nothing.
    let tx = client.transaction().await.unwrap();
    assert!(my_staff_certificates(&tx, s.tenant, s.event, "signer")
        .await
        .unwrap()
        .is_empty());
    tx.rollback().await.unwrap();
    let foreign: &'static FakeFetcher = Box::leak(Box::new(FakeFetcher::default()));
    let verification = dry_run(foreign, &pki.foreign_signer).await;
    assert_eq!(
        refusal(&verification),
        Some(CertificateCheckId::TrustedIssuer)
    );
    assert!(foreign.fetched().is_empty());

    // The chain refresh alone, on its own connection: already fresh.
    let refresh = refresh_chain_crls(
        &mut client,
        s.tenant,
        s.event,
        &[pki.maria.pem()],
        lists,
        now(),
    )
    .await
    .unwrap();
    assert!(refresh.ok.is_empty() && refresh.unavailable.is_empty());
    clean_up(&mut client, s).await;
}

#[tokio::test]
async fn the_job_refreshes_every_event_with_staff_issuers() {
    let _turn = COMMITTED.lock().await;
    let mut client = connection().await;
    let (s, _) = committed_scope(&mut client, line!()).await;
    let fetcher = FakeFetcher::with(&[(ROOT_CRL_URL, Pki::get().root_crl())]);
    let refreshed = refresh_all_staff_crls(&mut client, &fetcher).await.unwrap();
    assert!(refreshed >= 1);
    let refresh = refresh_event_crls(&mut client, s.tenant, s.event, &fetcher, now())
        .await
        .unwrap();
    assert_eq!(refresh.ok, vec![ROOT_CRL_URL.to_owned()]);
    let tx = client.transaction().await.unwrap();
    let stored = list_staff_crls(&tx, s.tenant, s.event).await.unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].status, CrlStatus::Ok);
    tx.rollback().await.unwrap();
    clean_up(&mut client, s).await;
}

/// Verifies and registers `signer` to `user` on first use in a transaction
/// of its own connection, committing it. `false` when it was refused.
async fn first_use_committed(
    request: SigningRequestRow,
    user: &'static str,
    signer: &'static Issued,
) -> bool {
    let mut client = connection().await;
    let tx = client.transaction().await.unwrap();
    lock_signing_event(&tx, request.tenant_id, request.election_event_id)
        .await
        .unwrap();
    let input = input(&request, user, signer);
    let verification = verify(&tx, &input).await;
    if !verification.passed() {
        return false;
    }
    first_use(&tx, &input, &verification).await.unwrap();
    tx.commit().await.unwrap();
    true
}

async fn active_rows_of_key(client: &mut Client, s: Scope, spki: &str) -> Vec<String> {
    let tx = client.transaction().await.unwrap();
    let rows = tx
        .query(
            "SELECT user_id FROM sequent_backend.staff_certificate
             WHERE tenant_id = $1 AND spki_sha256 = $2 AND status = 'active'",
            &[&s.tenant, &spki],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| row.get(0))
        .collect();
    tx.rollback().await.unwrap();
    rows
}

#[tokio::test]
async fn two_accounts_registering_one_key_at_once_leave_one_registration() {
    let _turn = COMMITTED.lock().await;
    let mut client = connection().await;
    let (s, request) = committed_scope(&mut client, line!()).await;
    {
        let tx = client.transaction().await.unwrap();
        let lists = current_lists();
        fetch_into(&tx, s, "Test Staff Individual CA", CRL_URL, &lists).await;
        tx.commit().await.unwrap();
    }
    let pki = Pki::get();
    let (alice, bob) = tokio::join!(
        first_use_committed(request.clone(), "alice", &pki.ana),
        first_use_committed(request.clone(), "bob", &pki.ana),
    );
    assert!(alice ^ bob, "exactly one registers: {alice} {bob}");
    let spki = CertificateIdentity::of(&pki.ana.cert, &[])
        .unwrap()
        .spki_sha256;
    let owners = active_rows_of_key(&mut client, s, &spki).await;
    assert_eq!(owners, vec![if alice { "alice" } else { "bob" }.to_owned()]);
    clean_up(&mut client, s).await;
}

#[tokio::test]
async fn a_first_use_racing_a_revocation_leaves_the_key_revoked() {
    let _turn = COMMITTED.lock().await;
    let mut client = connection().await;
    let (s, request) = committed_scope(&mut client, line!()).await;
    let pki = Pki::get();
    let row = {
        let tx = client.transaction().await.unwrap();
        let lists = current_lists();
        fetch_into(&tx, s, "Test Staff Individual CA", CRL_URL, &lists).await;
        let row = register_on_first_use(&tx, &request, "maria", &pki.maria).await;
        tx.commit().await.unwrap();
        row
    };
    let revoke = async {
        let mut client = connection().await;
        let tx = client.transaction().await.unwrap();
        let outcome = revoke_registration(
            &tx,
            s.tenant,
            s.event,
            row.id,
            "token lost",
            &actor("officer"),
            None,
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        outcome
    };
    // Her reissue (the same key) registers on first use, or is refused.
    let (_, outcome) = tokio::join!(
        first_use_committed(request.clone(), "maria", &pki.maria_reissue),
        revoke,
    );
    assert!(matches!(outcome, RevokeOutcome::Revoked { .. }));
    assert!(active_rows_of_key(&mut client, s, &row.spki_sha256)
        .await
        .is_empty());
    clean_up(&mut client, s).await;
}

#[tokio::test]
async fn display_names_come_from_the_tenant_realm_users() {
    assert_eq!(
        display_name(Some("Maria"), Some("Santos"), "msantos"),
        "Maria Santos"
    );
    assert_eq!(display_name(Some(" Maria "), None, "msantos"), "Maria");
    assert_eq!(display_name(None, Some(""), "msantos"), "msantos");

    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    // Keycloak's tables, as far as the query reads them.
    tx.batch_execute(
        "CREATE TEMP TABLE realm (id varchar(36) PRIMARY KEY, name varchar(255));
         CREATE TEMP TABLE user_entity (id varchar(36) PRIMARY KEY, realm_id varchar(255),
             username varchar(255), first_name varchar(255), last_name varchar(255));
         INSERT INTO realm VALUES ('r1', 'tenant-a'), ('r2', 'tenant-b');
         INSERT INTO user_entity VALUES
             ('u1', 'r1', 'msantos', 'Maria', 'Santos'),
             ('u2', 'r1', 'jreyes', NULL, NULL),
             ('u3', 'r2', 'other', 'Other', 'Realm');",
    )
    .await
    .unwrap();
    let names = display_names_in_realm(
        &tx,
        "tenant-a",
        &[
            "u1".to_owned(),
            "u2".to_owned(),
            "u3".to_owned(),
            "u4".to_owned(),
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        names,
        HashMap::from([
            ("u1".to_owned(), "Maria Santos".to_owned()),
            ("u2".to_owned(), "jreyes".to_owned()),
        ])
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn staff_issuers_export_for_events_with_any_uuid() {
    use windmill::services::signing::issuers::staff_issuers_pem_bundle;
    let mut client = connection().await;
    let tx = client.transaction().await.unwrap();
    // Version 7 and version 1 ids: not v4.
    let tenant_id = Uuid::parse_str("01890a5d-ac96-774b-bcce-b302099a8057").unwrap();
    let event_id = Uuid::parse_str("c232ab00-9414-11ec-b3c8-9e6bdeced846").unwrap();
    tenant(&tx, tenant_id).await;
    event(&tx, tenant_id, event_id).await;
    let (tenant_text, event_text) = (tenant_id.to_string(), event_id.to_string());
    assert_eq!(
        staff_issuers_pem_bundle(&tx, &tenant_text, &event_text)
            .await
            .unwrap(),
        None
    );
    let pki = Pki::get();
    import_staff_issuers(
        &tx,
        tenant_id,
        event_id,
        &[pki.root.cert.clone(), pki.individual_ca.cert.clone()],
        &actor("officer"),
        Allowance::Permission(Permissions::SIGNING_ISSUERS_WRITE),
        now(),
    )
    .await
    .unwrap();
    let bundle = staff_issuers_pem_bundle(&tx, &tenant_text, &event_text)
        .await
        .unwrap()
        .unwrap();
    let certificates = openssl::x509::X509::stack_from_pem(bundle.as_bytes()).unwrap();
    let mut subjects: Vec<String> = certificates
        .iter()
        .map(|certificate| {
            CertificateIdentity::of(certificate, &[])
                .unwrap()
                .common_name
        })
        .collect();
    subjects.sort();
    assert_eq!(subjects, ["Test Staff Individual CA", "Test Staff Root CA"]);
    assert!(staff_issuers_pem_bundle(&tx, "not-a-uuid", &event_text)
        .await
        .is_err());
    tx.rollback().await.unwrap();
}
