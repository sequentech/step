// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The signing tables and their queries: what the constraints refuse, the
//! enum texts they accept, and that the rows go with their election event.

#[path = "support/schema.rs"]
mod schema;

use chrono::{Duration, TimeZone, Utc};
use deadpool_postgres::{Pool, Transaction};
use electoral_log::messages::newtypes::SigningStatementKind;
use electoral_log::messages::statement::{StatementEventType, StatementLogType};
use sequent_core::signing::{
    CancelReason, CertificateAuthorityPurpose, CertificatePostBinding, CertificateRegistration,
    CrlStatus, CrlUnavailablePolicy, DocumentRevisionState, RequesterSigning, RevocationCheck,
    RevocationStatus, SignatureAlgorithm, SigningAction, SigningChecks, SigningRequestStatus,
    SigningRequirement, SigningRule, StaffCertificateRegistration, StaffCertificateStatus,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fmt::Display;
use strum::IntoEnumIterator;
use tokio_postgres::types::ToSql;
use uuid::Uuid;
use windmill::postgres::certificate_authority::{
    insert_certificate_authority, CertificateAuthorityRecord,
};
use windmill::postgres::election_event::delete_election_event;
use windmill::postgres::signing::*;
use windmill::services::signing::log::{stage, Actor, LogScope, LogStep, SystemOutcome};

#[derive(Clone, Copy)]
struct Scope {
    tenant: Uuid,
    event: Uuid,
}

/// A fixed v4 UUID per test (`seed`) and call (`n`).
fn id(seed: u32, n: u32) -> Uuid {
    Uuid::parse_str(&format!("{seed:08x}-0000-4000-8000-{n:012x}")).unwrap()
}

fn sha(text: &str) -> String {
    hex::encode(Sha256::digest(text.as_bytes()))
}

async fn tenant(tx: &Transaction<'_>, tenant: Uuid) {
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &format!("tenant-{tenant}")],
    )
    .await
    .unwrap();
}

async fn event(tx: &Transaction<'_>, tenant: Uuid, event: Uuid) -> Scope {
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&event, &tenant],
    )
    .await
    .unwrap();
    Scope { tenant, event }
}

/// A tenant with one election event.
async fn scope(tx: &Transaction<'_>, seed: u32) -> Scope {
    tenant(tx, id(seed, 1)).await;
    event(tx, id(seed, 1), id(seed, 2)).await
}

fn new_request(s: Scope, action: SigningAction, scope_key: &str, n: u32) -> NewSigningRequest {
    NewSigningRequest {
        id: Uuid::new_v4(),
        tenant_id: s.tenant,
        election_event_id: s.event,
        action,
        election_id: None,
        area_id: None,
        trustee_id: None,
        scope_key: scope_key.to_owned(),
        subject: json!({ "channel": "online" }),
        canonical_payload: format!("{{\"n\":{n}}}"),
        payload_sha256: sha(&format!("payload {n}")),
        document_id: None,
        document_sha256: None,
        code: "7F3A-91C2".to_owned(),
        config_revision: None,
        rule_revision: 1,
        rule_snapshot: json!({ "signatures": 2 }),
        required: 2,
        requested_by: "requester".to_owned(),
        requested_by_username: "requester-name".to_owned(),
        requested_by_name: Some("Requester Name".to_owned()),
        expires_at: None,
        permission_label: Some("post-1".to_owned()),
        created_at: Utc.with_ymd_and_hms(2028, 5, 12, 8, 0, n).unwrap(),
    }
}

/// A certificate of `user` whose certificate, key and holder hashes derive
/// from the given labels.
fn new_certificate(
    s: Scope,
    user: &str,
    certificate: &str,
    key: &str,
    holder: &str,
) -> NewStaffCertificate {
    NewStaffCertificate {
        tenant_id: s.tenant,
        election_event_id: s.event,
        user_id: user.to_owned(),
        username: format!("{user}-name"),
        user_display_name: Some(format!("{user} Display")),
        election_id: None,
        fingerprint_sha256: sha(&format!("certificate {certificate}")),
        spki_sha256: sha(&format!("key {key}")),
        holder_sha256: sha(&format!("holder {holder}")),
        serial: "01".to_owned(),
        subject: format!("CN={holder}"),
        issuer: "CN=Staff CA".to_owned(),
        not_before: Utc.with_ymd_and_hms(2028, 1, 1, 0, 0, 0).unwrap(),
        not_after: Utc.with_ymd_and_hms(2029, 1, 1, 0, 0, 0).unwrap(),
        pem: "-----BEGIN CERTIFICATE-----".to_owned(),
        registration: StaffCertificateRegistration::FirstUse,
        linked_to: None,
        registered_by: user.to_owned(),
        registered_by_name: Some(format!("{user} Display")),
    }
}

/// An approval of `request` by `user` with `certificate`'s identities.
fn new_approval(
    request: &SigningRequestRow,
    user: &str,
    certificate: &StaffCertificateRow,
) -> NewSigningApproval {
    NewSigningApproval {
        tenant_id: request.tenant_id,
        election_event_id: request.election_event_id,
        request_id: request.id,
        user_id: user.to_owned(),
        username: format!("{user}-name"),
        display_name: Some(format!("{user} Display")),
        auth_time: None,
        certificate_id: certificate.id,
        certificate_pem: certificate.pem.clone(),
        chain_pem: certificate.pem.clone(),
        fingerprint_sha256: certificate.fingerprint_sha256.clone(),
        spki_sha256: certificate.spki_sha256.clone(),
        holder_sha256: certificate.holder_sha256.clone(),
        algorithm: SignatureAlgorithm::RsaPkcs1Sha256,
        payload_signature: vec![1, 2, 3],
        document_signature: None,
        pdf_cms: None,
        revocation_status: RevocationStatus::Checked,
    }
}

/// The constraint an integrity error names; panics on any other error, so a
/// mistyped statement cannot pass as a refusal.
fn refusal(error: &tokio_postgres::Error) -> String {
    let db = error
        .as_db_error()
        .unwrap_or_else(|| panic!("not a database error: {error:?}"));
    assert!(
        db.code().code().starts_with("23"),
        "not a constraint refusal: {db:?}"
    );
    db.constraint()
        .unwrap_or_else(|| panic!("a refusal without its constraint: {db:?}"))
        .to_owned()
}

/// Keeps what a savepoint did, or rolls it back and names the constraint
/// that refused it.
async fn settle<T>(savepoint: Transaction<'_>, result: anyhow::Result<T>) -> Result<T, String> {
    match result {
        Ok(value) => {
            savepoint.commit().await.unwrap();
            Ok(value)
        }
        Err(error) => {
            savepoint.rollback().await.unwrap();
            let database = error
                .downcast_ref::<tokio_postgres::Error>()
                .unwrap_or_else(|| panic!("not a database error: {error:?}"));
            Err(refusal(database))
        }
    }
}

/// Runs one statement in a savepoint: its row count, or the constraint that
/// refused it.
async fn attempt(
    tx: &mut Transaction<'_>,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<u64, String> {
    let savepoint = tx.savepoint("attempt").await.unwrap();
    let result = savepoint.execute(sql, params).await.map_err(Into::into);
    settle(savepoint, result).await
}

/// `Ok(None)` while another request waits for the same scope.
async fn try_request(
    tx: &mut Transaction<'_>,
    request: &NewSigningRequest,
) -> Result<Option<SigningRequestRow>, String> {
    let savepoint = tx.savepoint("request").await.unwrap();
    let result = insert_signing_request(&savepoint, request).await;
    settle(savepoint, result).await
}

async fn insert_request(
    tx: &mut Transaction<'_>,
    request: &NewSigningRequest,
) -> Result<SigningRequestRow, String> {
    try_request(tx, request)
        .await
        .map(|row| row.expect("a request already waits for this scope"))
}

async fn insert_certificate(
    tx: &mut Transaction<'_>,
    certificate: &NewStaffCertificate,
) -> Result<StaffCertificateRow, String> {
    let savepoint = tx.savepoint("certificate").await.unwrap();
    let result = insert_staff_certificate(&savepoint, certificate).await;
    settle(savepoint, result).await
}

/// `Ok(Err(_))` when an identity of the approval already signed.
async fn try_approval(
    tx: &mut Transaction<'_>,
    approval: &NewSigningApproval,
) -> Result<Result<SigningApprovalRow, SigningApprovalConflict>, String> {
    let savepoint = tx.savepoint("approval").await.unwrap();
    let result = insert_signing_approval(&savepoint, approval).await;
    settle(savepoint, result).await
}

async fn insert_approval(
    tx: &mut Transaction<'_>,
    approval: &NewSigningApproval,
) -> Result<SigningApprovalRow, String> {
    try_approval(tx, approval)
        .await
        .map(|row| row.expect("an identity of the approval already signed"))
}

/// Sets `column` of the row `id` of `table` to each value in turn: every
/// value the Rust enum writes is accepted, and an unknown text is refused by
/// `constraint`. `also` is appended to the SET list, for columns that must
/// change together.
async fn accepts_exactly<T: Display>(
    tx: &mut Transaction<'_>,
    table: &str,
    column: &str,
    row_id: &(dyn ToSql + Sync),
    values: impl IntoIterator<Item = T>,
    constraint: &str,
    also: &str,
) {
    let sql = format!("UPDATE sequent_backend.{table} SET {column} = $2{also} WHERE id = $1");
    for value in values {
        let text = value.to_string();
        assert_eq!(
            attempt(tx, &sql, &[row_id, &text]).await,
            Ok(1),
            "{table}.{column} refuses {text:?}"
        );
    }
    for unknown in ["unknown", "", "NotRequired"] {
        assert_eq!(
            attempt(tx, &sql, &[row_id, &unknown]).await,
            Err(constraint.to_owned()),
            "{table}.{column} accepts {unknown:?}"
        );
    }
}

/// A staff issuer of the event: its certificate authority id.
async fn staff_issuer(tx: &Transaction<'_>, s: Scope, name: &str) -> Uuid {
    tx.query_one(
        "INSERT INTO sequent_backend.certificate_authority
             (tenant_id, election_event_id, common_name, subject, issuer_common_name,
              issuer, not_before, not_after, fingerprint_sha256, serial_number, pem, purpose)
         VALUES ($1, $2, $3, $3, $3, $3, now(), now(), $4, '01', 'pem', 'staff-signatures')
         RETURNING id",
        &[&s.tenant, &s.event, &name, &sha(name)],
    )
    .await
    .unwrap()
    .get(0)
}

/// The CRL of `issuer`.
async fn staff_crl(tx: &Transaction<'_>, s: Scope, issuer: Uuid, status: &str) -> Uuid {
    tx.query_one(
        "INSERT INTO sequent_backend.staff_crl
             (tenant_id, election_event_id, issuer_id, issuer_fingerprint, url, status)
         VALUES ($1, $2, $3, $4, 'http://crl.example/ca.crl', $5) RETURNING id",
        &[&s.tenant, &s.event, &issuer, &sha("issuer"), &status],
    )
    .await
    .unwrap()
    .get(0)
}

/// Every signing statement kind; the match keeps the list complete.
const STATEMENT_KINDS: [SigningStatementKind; 16] = {
    use SigningStatementKind::*;
    [
        SigningRequestCreated,
        SigningCertificateOpenFailed,
        SigningRequestSigned,
        SigningSignatureRefused,
        SigningCertificateRegistered,
        SigningHandover,
        SigningRequestCancelled,
        SigningRequestExpired,
        SigningRequestCompleted,
        SigningActionExecuted,
        SigningRuleChanged,
        SigningPermissionChanged,
        SigningIssuerChanged,
        SigningChecksChanged,
        SigningCertificateRevoked,
        SigningRequestsExported,
    ]
};

#[allow(dead_code)]
fn every_kind_is_listed(kind: SigningStatementKind) {
    use SigningStatementKind::*;
    match kind {
        SigningRequestCreated
        | SigningCertificateOpenFailed
        | SigningRequestSigned
        | SigningSignatureRefused
        | SigningCertificateRegistered
        | SigningHandover
        | SigningRequestCancelled
        | SigningRequestExpired
        | SigningRequestCompleted
        | SigningActionExecuted
        | SigningRuleChanged
        | SigningPermissionChanged
        | SigningIssuerChanged
        | SigningChecksChanged
        | SigningCertificateRevoked
        | SigningRequestsExported => {}
    }
}

/// A request-created step of the event, by its requester.
fn created_step(s: Scope) -> LogStep {
    LogStep {
        kind: SigningStatementKind::SigningRequestCreated,
        user: Actor {
            user_id: "requester".to_owned(),
            username: "requester-name".to_owned(),
        },
        system: SystemOutcome::Info,
        scope: LogScope {
            tenant_id: s.tenant,
            election_event_id: s.event,
            election_id: None,
            area_id: None,
        },
        description: "Started signing request 7F3A-91C2".to_owned(),
        details: json!({}),
    }
}

async fn count(tx: &Transaction<'_>, sql: &str, params: &[&(dyn ToSql + Sync)]) -> i64 {
    tx.query_one(sql, params).await.unwrap().get(0)
}

#[tokio::test]
async fn a_person_fills_one_slot_of_a_request() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let request = insert_request(
        &mut tx,
        &new_request(s, SigningAction::OpenVoting, "post-1", 1),
    )
    .await
    .unwrap();
    let alice = insert_certificate(&mut tx, &new_certificate(s, "alice", "a", "a", "a"))
        .await
        .unwrap();
    let bob = insert_certificate(&mut tx, &new_certificate(s, "bob", "b", "b", "b"))
        .await
        .unwrap();
    let signed = insert_approval(&mut tx, &new_approval(&request, "alice", &alice))
        .await
        .unwrap();
    assert_eq!(signed.algorithm, SignatureAlgorithm::RsaPkcs1Sha256);
    assert_eq!(signed.revocation_status, RevocationStatus::Checked);
    assert_eq!(signed.display_name.as_deref(), Some("alice Display"));

    // Each attempt starts from Bob's valid approval and shares one identity
    // with Alice's.
    let mut again = new_approval(&request, "bob", &bob);
    again.user_id = "alice".to_owned();
    assert_eq!(
        try_approval(&mut tx, &again).await,
        Ok(Err(SigningApprovalConflict::DuplicateUser)),
        "the same account signs again"
    );
    let mut same_certificate = new_approval(&request, "bob", &bob);
    same_certificate.fingerprint_sha256 = alice.fingerprint_sha256.clone();
    assert_eq!(
        try_approval(&mut tx, &same_certificate).await,
        Ok(Err(SigningApprovalConflict::DuplicateCertificate)),
        "Alice's certificate under Bob's account"
    );
    let mut same_key = new_approval(&request, "bob", &bob);
    same_key.spki_sha256 = alice.spki_sha256.clone();
    assert_eq!(
        try_approval(&mut tx, &same_key).await,
        Ok(Err(SigningApprovalConflict::DuplicateKey)),
        "a reissued certificate of Alice's key"
    );
    let mut same_holder = new_approval(&request, "bob", &bob);
    same_holder.holder_sha256 = alice.holder_sha256.clone();
    assert_eq!(
        try_approval(&mut tx, &same_holder).await,
        Ok(Err(SigningApprovalConflict::DuplicateHolder)),
        "a second certificate issued to Alice"
    );
    let mut uppercase = new_approval(&request, "bob", &bob);
    uppercase.fingerprint_sha256 = alice.fingerprint_sha256.to_uppercase();
    assert_eq!(
        insert_approval(&mut tx, &uppercase).await,
        Err("signing_approval_fingerprint_sha256_hex".to_owned()),
        "Alice's fingerprint written another way"
    );

    insert_approval(&mut tx, &new_approval(&request, "bob", &bob))
        .await
        .unwrap();
    // Signing times come from the clock, so one transaction keeps its order.
    let signers = list_signing_approvals(&tx, s.tenant, s.event, request.id)
        .await
        .unwrap()
        .into_iter()
        .map(|approval| approval.user_id)
        .collect::<Vec<_>>();
    assert_eq!(signers, ["alice", "bob"]);
    assert_eq!(
        count_signing_approvals(&tx, s.tenant, s.event, request.id)
            .await
            .unwrap(),
        2
    );

    // An approval stays in its request's event.
    let other = event(&tx, s.tenant, id(line!(), 3)).await;
    let carol = insert_certificate(&mut tx, &new_certificate(other, "carol", "c", "c", "c"))
        .await
        .unwrap();
    let mut elsewhere = new_approval(&request, "carol", &carol);
    elsewhere.election_event_id = other.event;
    assert_eq!(
        insert_approval(&mut tx, &elsewhere).await,
        Err("signing_approval_of_its_request".to_owned())
    );
}

async fn election(tx: &Transaction<'_>, s: Scope, election: Uuid) {
    tx.execute(
        "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id)
         VALUES ($1, $2, $3)",
        &[&election, &s.tenant, &s.event],
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn a_post_takes_its_requests_along_and_leaves_its_registrations() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let seed = line!();
    let s = scope(&tx, seed).await;
    let other = event(&tx, s.tenant, id(seed, 3)).await;
    let post = id(seed, 10);
    election(&tx, s, post).await;
    election(&tx, other, id(seed, 11)).await;

    let mut request = new_request(s, SigningAction::OpenVoting, "post", 1);
    request.election_id = Some(post);
    let request = insert_request(&mut tx, &request).await.unwrap();
    let mut certificate = new_certificate(s, "alice", "a", "a", "a");
    certificate.election_id = Some(post);
    let certificate = insert_certificate(&mut tx, &certificate).await.unwrap();
    insert_approval(&mut tx, &new_approval(&request, "alice", &certificate))
        .await
        .unwrap();

    // A Post of another event, or none at all.
    for (election_id, why) in [
        (id(seed, 11), "another event's Post"),
        (id(seed, 12), "no Post"),
    ] {
        let mut stray = new_request(s, SigningAction::OpenVoting, "stray", 2);
        stray.election_id = Some(election_id);
        assert_eq!(
            insert_request(&mut tx, &stray).await,
            Err("signing_request_of_its_election".to_owned()),
            "{why}"
        );
        let mut stray = new_certificate(s, "bob", "b", "b", "b");
        stray.election_id = Some(election_id);
        assert_eq!(
            insert_certificate(&mut tx, &stray).await,
            Err("staff_certificate_for_its_election".to_owned()),
            "{why}"
        );
    }
    let mut stray = new_request(s, SigningAction::GenerateElectionReturns, "stray", 3);
    stray.area_id = Some(id(seed, 13));
    assert_eq!(
        insert_request(&mut tx, &stray).await,
        Err("signing_request_of_its_area".to_owned())
    );

    tx.execute(
        "DELETE FROM sequent_backend.election WHERE id = $1",
        &[&post],
    )
    .await
    .unwrap();
    assert_eq!(
        get_signing_request(&tx, s.tenant, s.event, request.id)
            .await
            .unwrap(),
        None,
        "the request goes with its Post"
    );
    assert_eq!(
        count_signing_approvals(&tx, s.tenant, s.event, request.id)
            .await
            .unwrap(),
        0
    );
    let kept = find_active_staff_certificates_by_spki(&tx, s.tenant, &certificate.spki_sha256)
        .await
        .unwrap();
    assert_eq!(kept.len(), 1, "the registration stays");
    assert_eq!(kept[0].election_id, None, "for no Post");
}

#[tokio::test]
async fn one_request_waits_per_scope_until_it_is_cancelled() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let first = insert_request(
        &mut tx,
        &new_request(s, SigningAction::CloseVoting, "post-1", 1),
    )
    .await
    .unwrap();
    assert_eq!(first.status, SigningRequestStatus::Waiting);
    assert_eq!(first.cancel_reason, None);
    assert_eq!(first.expires_at, None);
    assert_eq!(first.requested_by_name.as_deref(), Some("Requester Name"));

    // Not an error: the caller reads the waiting request and supersedes it.
    assert_eq!(
        try_request(
            &mut tx,
            &new_request(s, SigningAction::CloseVoting, "post-1", 2)
        )
        .await,
        Ok(None),
        "a second waiting request for the same scope"
    );
    // Controls: another scope, and another action on the same scope.
    insert_request(
        &mut tx,
        &new_request(s, SigningAction::CloseVoting, "post-2", 3),
    )
    .await
    .unwrap();
    insert_request(
        &mut tx,
        &new_request(s, SigningAction::OpenVoting, "post-1", 4),
    )
    .await
    .unwrap();

    let waiting = list_waiting_signing_requests(&tx, s.tenant, s.event, SigningAction::CloseVoting)
        .await
        .unwrap();
    assert_eq!(
        waiting
            .iter()
            .map(|r| r.scope_key.as_str())
            .collect::<Vec<_>>(),
        ["post-1", "post-2"]
    );

    // A cancellation says why.
    assert_eq!(
        attempt(
            &mut tx,
            "UPDATE sequent_backend.signing_request SET status = 'cancelled' WHERE id = $1",
            &[&first.id],
        )
        .await,
        Err("signing_request_cancelled_with_reason".to_owned())
    );
    let cancelled = update_signing_request_status(
        &tx,
        s.tenant,
        s.event,
        first.id,
        &SigningRequestTransition::Cancel {
            reason: CancelReason::Superseded,
            by: Some("operator".to_owned()),
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(cancelled.status, SigningRequestStatus::Cancelled);
    assert_eq!(cancelled.cancel_reason, Some(CancelReason::Superseded));
    assert_eq!(cancelled.cancelled_by.as_deref(), Some("operator"));
    // Only a waiting request is cancelled or completed.
    for transition in [
        SigningRequestTransition::Complete,
        SigningRequestTransition::Expire,
    ] {
        assert_eq!(
            update_signing_request_status(&tx, s.tenant, s.event, first.id, &transition)
                .await
                .unwrap(),
            None,
            "{transition:?} after the cancellation"
        );
    }

    let next = insert_request(
        &mut tx,
        &new_request(s, SigningAction::CloseVoting, "post-1", 5),
    )
    .await
    .unwrap();
    let locked = lock_signing_request(&tx, s.tenant, s.event, next.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(locked, next);
    assert_eq!(
        get_signing_request(&tx, s.tenant, s.event, first.id)
            .await
            .unwrap(),
        Some(cancelled)
    );
    assert_eq!(
        get_signing_request(&tx, s.tenant, id(line!(), 7), first.id)
            .await
            .unwrap(),
        None,
        "a request of another event"
    );

    let completed = update_signing_request_status(
        &tx,
        s.tenant,
        s.event,
        next.id,
        &SigningRequestTransition::Complete,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(completed.status, SigningRequestStatus::Completed);
    assert!(completed.completed_at.is_some());
    assert_eq!(completed.executed_at, None);
    let executed = update_signing_request_status(
        &tx,
        s.tenant,
        s.event,
        next.id,
        &SigningRequestTransition::Execute {
            result: Some(json!({ "ok": true })),
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(executed.status, SigningRequestStatus::Executed);
    assert!(executed.executed_at.is_some());
    assert_eq!(executed.execution_result, Some(json!({ "ok": true })));
    assert_eq!(
        update_signing_request_status(
            &tx,
            s.tenant,
            s.event,
            next.id,
            &SigningRequestTransition::Fail { result: None },
        )
        .await
        .unwrap(),
        None,
        "an executed request does not fail afterwards"
    );
    // A completed request doesn't block the next one either.
    insert_request(
        &mut tx,
        &new_request(s, SigningAction::CloseVoting, "post-1", 6),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn stage_queues_a_user_and_a_system_entry_of_one_step() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let log_scope = LogScope {
        tenant_id: s.tenant,
        election_event_id: s.event,
        election_id: Some(id(line!(), 9)),
        area_id: None,
    };
    let refused = LogStep {
        kind: SigningStatementKind::SigningSignatureRefused,
        user: Actor {
            user_id: "alice".to_owned(),
            username: "alice-name".to_owned(),
        },
        system: SystemOutcome::Error,
        scope: log_scope,
        description: "Signature refused on 7F3A-91C2".to_owned(),
        details: json!({ "check": "already-signed" }),
    };
    let step_id = stage(&tx, &refused).await.unwrap();

    let rows = fetch_unposted_signing_log_outbox(&tx, s.tenant, s.event, 10)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    let (user, system) = (&rows[0], &rows[1]);
    assert!(user.id < system.id, "the USER entry is posted first");
    for row in &rows {
        assert_eq!(row.step_id, step_id);
        assert_eq!(
            row.statement_kind,
            SigningStatementKind::SigningSignatureRefused
        );
        assert_eq!(row.election_id, log_scope.election_id);
        assert_eq!(
            row.body,
            json!({
                "description": "Signature refused on 7F3A-91C2",
                "details": { "check": "already-signed" },
            })
        );
        assert_eq!(row.attempts, 0);
        assert_eq!(row.posted_at, None);
    }
    assert!(matches!(user.event_type, StatementEventType::USER));
    assert!(matches!(user.log_type, StatementLogType::INFO));
    assert_eq!(user.user_id.as_deref(), Some("alice"));
    assert_eq!(user.username.as_deref(), Some("alice-name"));
    assert!(matches!(system.event_type, StatementEventType::SYSTEM));
    assert!(matches!(system.log_type, StatementLogType::ERROR));
    assert_eq!(system.user_id, None);
    assert_eq!(system.username, None);
    // The stored texts, as the worker and the CHECK constraints read them.
    let stored = tx
        .query(
            "SELECT entry, statement_kind, event_type, log_type
             FROM sequent_backend.signing_log_outbox WHERE step_id = $1 ORDER BY id",
            &[&step_id],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| {
            (
                row.get::<_, i16>(0),
                row.get::<_, String>(1),
                row.get::<_, String>(2),
                row.get::<_, String>(3),
            )
        })
        .collect::<Vec<_>>();
    let text = |entry: i16, event_type: &str, log_type: &str| {
        (
            entry,
            "SigningSignatureRefused".to_owned(),
            event_type.to_owned(),
            log_type.to_owned(),
        )
    };
    assert_eq!(
        stored,
        [text(0, "USER", "INFO"), text(1, "SYSTEM", "ERROR")]
    );

    let signed = LogStep {
        kind: SigningStatementKind::SigningRequestSigned,
        system: SystemOutcome::Info,
        ..refused.clone()
    };
    let second = stage(&tx, &signed).await.unwrap();
    assert_ne!(second, step_id);
    let rows = fetch_unposted_signing_log_outbox(&tx, s.tenant, s.event, 10)
        .await
        .unwrap();
    assert_eq!(rows.len(), 4);
    assert!(matches!(rows[3].log_type, StatementLogType::INFO));

    // Posting and failures.
    mark_signing_log_outbox_failed(&tx, rows[0].id, "board down")
        .await
        .unwrap();
    mark_signing_log_outbox_posted(&tx, rows[1].id)
        .await
        .unwrap();
    let rows = fetch_unposted_signing_log_outbox(&tx, s.tenant, s.event, 2)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2, "the limit");
    assert_eq!(rows[0].attempts, 1);
    assert_eq!(rows[0].last_error.as_deref(), Some("board down"));
    assert_eq!(rows[1].step_id, second, "the posted entry is skipped");
}

#[tokio::test]
async fn an_outbox_entry_is_a_user_or_a_system_entry() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let sql = "INSERT INTO sequent_backend.signing_log_outbox
                   (tenant_id, election_event_id, step_id, entry, statement_kind,
                    event_type, log_type, user_id, body)
               VALUES ($1, $2, $3, $4, 'SigningRequestCreated', $5, $6, $7, '{}')";
    let step = Uuid::new_v4();
    let user = Some("alice");
    let none: Option<&str> = None;
    // (same step as the first two, entry, event type, log type, user, outcome)
    let cases: [(bool, i16, &str, &str, Option<&str>, Result<u64, &str>); 7] = [
        (true, 0, "USER", "INFO", user, Ok(1)),
        (true, 1, "SYSTEM", "ERROR", none, Ok(1)),
        (
            true,
            0,
            "USER",
            "INFO",
            user,
            Err("signing_log_outbox_one_entry_per_step"),
        ),
        (
            false,
            0,
            "SYSTEM",
            "INFO",
            none,
            Err("signing_log_outbox_entry_is_its_event_type"),
        ),
        (
            false,
            1,
            "SYSTEM",
            "INFO",
            user,
            Err("signing_log_outbox_user_on_user_entries"),
        ),
        (
            false,
            1,
            "system",
            "INFO",
            none,
            Err("signing_log_outbox_event_type_known"),
        ),
        (
            false,
            1,
            "SYSTEM",
            "WARN",
            none,
            Err("signing_log_outbox_log_type_known"),
        ),
    ];
    for (same_step, entry, event_type, log_type, user_id, expected) in cases {
        let step = if same_step { step } else { Uuid::new_v4() };
        assert_eq!(
            attempt(
                &mut tx,
                sql,
                &[
                    &s.tenant,
                    &s.event,
                    &step,
                    &entry,
                    &event_type,
                    &log_type,
                    &user_id
                ],
            )
            .await,
            expected.map_err(str::to_owned),
            "entry {entry} {event_type} {log_type} {user_id:?}"
        );
    }
}

#[tokio::test]
async fn rules_and_checks_are_saved_from_the_revision_read() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let action = SigningAction::TransmitResults;
    assert_eq!(
        get_signing_rule(&tx, s.tenant, s.event, action)
            .await
            .unwrap(),
        None
    );
    let mut rule = SigningRule {
        action,
        requirement: SigningRequirement::Required,
        signatures: 3,
        requester_signing: RequesterSigning::NotAllowed,
        expires_minutes: None,
        revision: 0,
    };
    assert_eq!(
        upsert_signing_rule(&tx, s.tenant, s.event, &rule, 1, "officer", None)
            .await
            .unwrap(),
        None,
        "a first save that read a revision nobody wrote"
    );
    let saved = upsert_signing_rule(
        &tx,
        s.tenant,
        s.event,
        &rule,
        0,
        "officer",
        Some("Officer Name"),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        saved.rule,
        SigningRule {
            revision: 1,
            ..rule.clone()
        }
    );
    assert_eq!(saved.updated_by, "officer");
    assert_eq!(saved.updated_by_name.as_deref(), Some("Officer Name"));

    rule.signatures = 2;
    rule.expires_minutes = Some(120);
    rule.requester_signing = RequesterSigning::Allowed;
    assert_eq!(
        upsert_signing_rule(&tx, s.tenant, s.event, &rule, 0, "other", None)
            .await
            .unwrap(),
        None,
        "a save from a stale revision"
    );
    let resaved = upsert_signing_rule(&tx, s.tenant, s.event, &rule, 1, "other", None)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        resaved.rule,
        SigningRule {
            revision: 2,
            ..rule.clone()
        }
    );
    assert_eq!(resaved.id, saved.id);
    assert_eq!(resaved.updated_by_name, None, "the name of the last save");
    assert_eq!(
        get_signing_rule(&tx, s.tenant, s.event, action)
            .await
            .unwrap(),
        Some(resaved)
    );
    assert_eq!(
        get_signing_rule(&tx, s.tenant, s.event, SigningAction::OpenVoting)
            .await
            .unwrap(),
        None,
        "another action keeps its default"
    );

    assert_eq!(
        get_signing_checks(&tx, s.tenant, s.event).await.unwrap(),
        None
    );
    let checks = SigningChecks {
        revocation_check: RevocationCheck::DontCheck,
        crl_unavailable: CrlUnavailablePolicy::AcceptUnchecked,
        registration: CertificateRegistration::SecurityOfficerOnly,
        post_binding: CertificatePostBinding::AnyPost,
        revision: 0,
    };
    let saved = upsert_signing_checks(
        &tx,
        s.tenant,
        s.event,
        &checks,
        0,
        "officer",
        Some("Officer Name"),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(saved.updated_by_name.as_deref(), Some("Officer Name"));
    assert_eq!(
        saved.checks,
        SigningChecks {
            revision: 1,
            ..checks.clone()
        }
    );
    assert_eq!(
        upsert_signing_checks(&tx, s.tenant, s.event, &checks, 0, "officer", None)
            .await
            .unwrap(),
        None,
        "a save from a stale revision"
    );
    let defaults = upsert_signing_checks(
        &tx,
        s.tenant,
        s.event,
        &SigningChecks::default(),
        1,
        "x",
        None,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        defaults.checks,
        SigningChecks {
            revision: 2,
            ..SigningChecks::default()
        }
    );
    assert_eq!(
        get_signing_checks(&tx, s.tenant, s.event).await.unwrap(),
        Some(defaults)
    );
}

#[tokio::test]
async fn staff_certificates_are_found_by_certificate_key_and_holder() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let seed = line!();
    let s = scope(&tx, seed).await;
    let second_event = event(&tx, s.tenant, id(seed, 3)).await;
    let other_tenant = scope(&tx, seed + 1).await;

    let alice = insert_certificate(&mut tx, &new_certificate(s, "alice", "a", "a", "a"))
        .await
        .unwrap();
    assert_eq!(alice.status, StaffCertificateStatus::Active);
    assert_eq!(alice.registration, StaffCertificateRegistration::FirstUse);
    assert_eq!(alice.user_display_name.as_deref(), Some("alice Display"));
    assert_eq!(alice.registered_by_name.as_deref(), Some("alice Display"));
    assert_eq!(alice.revoked_by_name, None);
    assert_eq!(
        insert_certificate(&mut tx, &new_certificate(s, "bob", "a", "b", "b")).await,
        Err("staff_certificate_one_active".to_owned()),
        "Alice's certificate registered on first use to Bob"
    );
    // A Security Officer links it to Alice's second account, once.
    let mut link = new_certificate(s, "alice-trustee", "a", "a", "a");
    link.registration = StaffCertificateRegistration::SecurityOfficer;
    link.linked_to = Some("alice".to_owned());
    let linked = insert_certificate(&mut tx, &link).await.unwrap();
    assert_eq!(
        insert_certificate(&mut tx, &link).await,
        Err("staff_certificate_one_active_per_user".to_owned()),
        "the same link twice"
    );
    // Only a Security Officer links, and to another account.
    let mut first_use_link = new_certificate(s, "alice-auditor", "a", "a", "a");
    first_use_link.linked_to = Some("alice".to_owned());
    let mut self_link = link.clone();
    self_link.user_id = "alice".to_owned();
    for (refused, why) in [
        (first_use_link, "a link on first use"),
        (self_link, "a link to itself"),
    ] {
        assert_eq!(
            insert_certificate(&mut tx, &refused).await,
            Err("staff_certificate_linked_by_security_officer".to_owned()),
            "{why}"
        );
    }
    // Controls in another event and another tenant.
    let reissued = insert_certificate(
        &mut tx,
        &new_certificate(second_event, "alice", "a2", "a", "a"),
    )
    .await
    .unwrap();
    insert_certificate(
        &mut tx,
        &new_certificate(other_tenant, "mallory", "a", "a", "a"),
    )
    .await
    .unwrap();

    // Registration times come from the clock: the order they were made in.
    let ids =
        |rows: Vec<StaffCertificateRow>| rows.into_iter().map(|row| row.id).collect::<Vec<_>>();
    assert_eq!(
        ids(find_active_staff_certificates_by_fingerprint(
            &tx,
            s.tenant,
            s.event,
            &alice.fingerprint_sha256
        )
        .await
        .unwrap()),
        vec![alice.id, linked.id]
    );
    assert_eq!(
        ids(find_active_staff_certificates_by_fingerprint(
            &tx,
            s.tenant,
            second_event.event,
            &alice.fingerprint_sha256
        )
        .await
        .unwrap()),
        Vec::<Uuid>::new(),
        "the fingerprint lookup stays in its event"
    );
    // Key and holder lookups cover the tenant, not other tenants.
    assert_eq!(
        ids(
            find_active_staff_certificates_by_spki(&tx, s.tenant, &alice.spki_sha256)
                .await
                .unwrap()
        ),
        vec![alice.id, linked.id, reissued.id]
    );
    assert_eq!(
        ids(
            find_active_staff_certificates_by_holder(&tx, s.tenant, &alice.holder_sha256)
                .await
                .unwrap()
        ),
        vec![alice.id, linked.id, reissued.id]
    );

    // A revoked registration says when, and is no longer found.
    assert_eq!(
        attempt(
            &mut tx,
            "UPDATE sequent_backend.staff_certificate SET status = 'revoked' WHERE id = $1",
            &[&reissued.id],
        )
        .await,
        Err("staff_certificate_revoked_when".to_owned())
    );
    attempt(
        &mut tx,
        "UPDATE sequent_backend.staff_certificate
         SET status = 'revoked', revoked_at = now(), revoked_by = 'officer',
             revoked_by_name = 'Officer Name'
         WHERE id = $1",
        &[&reissued.id],
    )
    .await
    .unwrap();
    assert_eq!(
        ids(
            find_active_staff_certificates_by_spki(&tx, s.tenant, &alice.spki_sha256)
                .await
                .unwrap()
        ),
        vec![alice.id, linked.id]
    );
    // A revocation sticks across the tenant: the revoked certificate and key
    // are found, so first use can refuse them. The holder is not: a person
    // may get a new key after a revocation.
    let revoked =
        find_revoked_staff_certificates_by_fingerprint(&tx, s.tenant, &reissued.fingerprint_sha256)
            .await
            .unwrap();
    assert_eq!(ids(revoked.clone()), vec![reissued.id]);
    assert_eq!(revoked[0].status, StaffCertificateStatus::Revoked);
    assert_eq!(revoked[0].revoked_by.as_deref(), Some("officer"));
    assert_eq!(revoked[0].revoked_by_name.as_deref(), Some("Officer Name"));
    assert_eq!(
        ids(
            find_revoked_staff_certificates_by_spki(&tx, s.tenant, &alice.spki_sha256)
                .await
                .unwrap()
        ),
        vec![reissued.id]
    );
    assert_eq!(
        ids(
            find_revoked_staff_certificates_by_spki(&tx, s.tenant, &alice.fingerprint_sha256)
                .await
                .unwrap()
        ),
        Vec::<Uuid>::new(),
        "a fingerprint is not a key"
    );
    assert_eq!(
        ids(find_revoked_staff_certificates_by_fingerprint(
            &tx,
            other_tenant.tenant,
            &reissued.fingerprint_sha256
        )
        .await
        .unwrap()),
        Vec::<Uuid>::new(),
        "another tenant"
    );
}

#[tokio::test]
async fn the_tables_accept_exactly_the_enum_texts() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;

    let rule = upsert_signing_rule(
        &tx,
        s.tenant,
        s.event,
        &SigningRule::default_for(SigningAction::OpenVoting),
        0,
        "officer",
        None,
    )
    .await
    .unwrap()
    .unwrap();
    let checks = upsert_signing_checks(
        &tx,
        s.tenant,
        s.event,
        &SigningChecks::default(),
        0,
        "x",
        None,
    )
    .await
    .unwrap()
    .unwrap();
    let request = insert_request(
        &mut tx,
        &new_request(s, SigningAction::OpenVoting, "post-1", 1),
    )
    .await
    .unwrap();
    let certificate = insert_certificate(&mut tx, &new_certificate(s, "alice", "a", "a", "a"))
        .await
        .unwrap();
    let approval = insert_approval(&mut tx, &new_approval(&request, "alice", &certificate))
        .await
        .unwrap();
    let revision: Uuid = tx
        .query_one(
            "INSERT INTO sequent_backend.signing_document_revision
                 (tenant_id, election_event_id, request_id, revision, document_id, sha256, state)
             VALUES ($1, $2, $3, 0, $4, $5, 'base') RETURNING id",
            &[
                &s.tenant,
                &s.event,
                &request.id,
                &Uuid::new_v4(),
                &sha("pdf"),
            ],
        )
        .await
        .unwrap()
        .get(0);
    let issuer = staff_issuer(&tx, s, "Staff CA").await;
    let crl = staff_crl(&tx, s, issuer, "unavailable").await;
    stage(&tx, &created_step(s)).await.unwrap();
    let outbox: i64 = tx
        .query_one(
            "SELECT id FROM sequent_backend.signing_log_outbox
             WHERE election_event_id = $1 ORDER BY id LIMIT 1",
            &[&s.event],
        )
        .await
        .unwrap()
        .get(0);

    accepts_exactly(
        &mut tx,
        "signing_rule",
        "action",
        &rule.id,
        SigningAction::iter(),
        "signing_rule_action_known",
        "",
    )
    .await;
    accepts_exactly(
        &mut tx,
        "signing_rule",
        "requirement",
        &rule.id,
        SigningRequirement::iter(),
        "signing_rule_requirement_known",
        "",
    )
    .await;
    accepts_exactly(
        &mut tx,
        "signing_rule",
        "requester_signing",
        &rule.id,
        RequesterSigning::iter(),
        "signing_rule_requester_signing_known",
        "",
    )
    .await;
    accepts_exactly(
        &mut tx,
        "signing_checks",
        "revocation_check",
        &checks.id,
        RevocationCheck::iter(),
        "signing_checks_revocation_check_known",
        "",
    )
    .await;
    accepts_exactly(
        &mut tx,
        "signing_checks",
        "crl_unavailable",
        &checks.id,
        CrlUnavailablePolicy::iter(),
        "signing_checks_crl_unavailable_known",
        "",
    )
    .await;
    accepts_exactly(
        &mut tx,
        "signing_checks",
        "registration",
        &checks.id,
        CertificateRegistration::iter(),
        "signing_checks_registration_known",
        "",
    )
    .await;
    accepts_exactly(
        &mut tx,
        "signing_checks",
        "post_binding",
        &checks.id,
        CertificatePostBinding::iter(),
        "signing_checks_post_binding_known",
        "",
    )
    .await;
    accepts_exactly(
        &mut tx,
        "signing_request",
        "action",
        &request.id,
        SigningAction::iter(),
        "signing_request_action_known",
        "",
    )
    .await;
    // A status changes with its reason or its times.
    accepts_exactly(
        &mut tx,
        "signing_request",
        "status",
        &request.id,
        SigningRequestStatus::iter(),
        "signing_request_status_known",
        ", cancel_reason = CASE WHEN $2 = 'cancelled' THEN 'by-operator' END,
           completed_at = CASE WHEN $2 IN ('completed', 'executed', 'failed')
               THEN clock_timestamp() END,
           executed_at = CASE WHEN $2 = 'executed' THEN clock_timestamp() END",
    )
    .await;
    // Without their times.
    for (status, constraint) in [
        ("completed", "signing_request_completed_when"),
        ("failed", "signing_request_completed_when"),
        ("executed", "signing_request_executed_when"),
    ] {
        let also = if status == "executed" {
            ", completed_at = clock_timestamp(), executed_at = NULL"
        } else {
            ", completed_at = NULL"
        };
        assert_eq!(
            attempt(
                &mut tx,
                &format!(
                    "UPDATE sequent_backend.signing_request
                     SET status = $2, cancel_reason = NULL{also} WHERE id = $1"
                ),
                &[&request.id, &status],
            )
            .await,
            Err(constraint.to_owned()),
            "{status}"
        );
    }
    accepts_exactly(
        &mut tx,
        "signing_log_outbox",
        "statement_kind",
        &outbox,
        STATEMENT_KINDS,
        "signing_log_outbox_statement_kind_known",
        "",
    )
    .await;
    accepts_exactly(
        &mut tx,
        "signing_request",
        "cancel_reason",
        &request.id,
        CancelReason::iter(),
        "signing_request_cancel_reason_known",
        ", status = 'cancelled'",
    )
    .await;
    accepts_exactly(
        &mut tx,
        "signing_approval",
        "algorithm",
        &approval.id,
        SignatureAlgorithm::iter(),
        "signing_approval_algorithm_known",
        "",
    )
    .await;
    accepts_exactly(
        &mut tx,
        "signing_approval",
        "revocation_status",
        &approval.id,
        RevocationStatus::iter(),
        "signing_approval_revocation_status_known",
        "",
    )
    .await;
    accepts_exactly(
        &mut tx,
        "staff_certificate",
        "status",
        &certificate.id,
        StaffCertificateStatus::iter(),
        "staff_certificate_status_known",
        ", revoked_at = CASE WHEN $2 = 'revoked' THEN now() END",
    )
    .await;
    accepts_exactly(
        &mut tx,
        "staff_certificate",
        "registration",
        &certificate.id,
        StaffCertificateRegistration::iter(),
        "staff_certificate_registration_known",
        "",
    )
    .await;
    accepts_exactly(
        &mut tx,
        "signing_document_revision",
        "state",
        &revision,
        DocumentRevisionState::iter(),
        "signing_document_revision_state_known",
        "",
    )
    .await;
    accepts_exactly(
        &mut tx,
        "staff_crl",
        "status",
        &crl,
        CrlStatus::iter(),
        "staff_crl_status_known",
        "",
    )
    .await;

    // Rows read back as the enums they were written as.
    let request = get_signing_request(&tx, s.tenant, s.event, request.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(request.status, SigningRequestStatus::Cancelled);
    assert_eq!(request.cancel_reason, CancelReason::iter().last());
}

#[tokio::test]
async fn certificate_authorities_sign_voters_in_unless_they_are_staff_issuers() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    // Written as before staff signatures, without a purpose.
    let voter: Uuid = tx
        .query_one(
            "INSERT INTO sequent_backend.certificate_authority
                 (tenant_id, election_event_id, common_name, subject, issuer_common_name,
                  issuer, not_before, not_after, fingerprint_sha256, serial_number, pem)
             VALUES ($1, $2, 'CA', 'CN=CA', 'CA', 'CN=CA', now(), now(), $3, '01', 'pem')
             RETURNING id",
            &[&s.tenant, &s.event, &sha("voter ca")],
        )
        .await
        .unwrap()
        .get(0);
    let purpose: String = tx
        .query_one(
            "SELECT purpose FROM sequent_backend.certificate_authority WHERE id = $1",
            &[&voter],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(purpose, "voter-sign-in");
    assert_eq!(
        purpose,
        CertificateAuthorityPurpose::default().to_string(),
        "the column default is the Rust default"
    );
    accepts_exactly(
        &mut tx,
        "certificate_authority",
        "purpose",
        &voter,
        CertificateAuthorityPurpose::iter(),
        "certificate_authority_purpose_known",
        "",
    )
    .await;

    // The row is now a staff issuer. The same certificate may also be the
    // event's voter CA, once: the voter import skips a second copy.
    let record = || CertificateAuthorityRecord {
        id: Uuid::new_v4(),
        tenant_id: s.tenant,
        election_event_id: s.event,
        common_name: "CA".to_owned(),
        subject: "CN=CA".to_owned(),
        issuer_common_name: "CA".to_owned(),
        issuer: "CN=CA".to_owned(),
        not_before: Utc::now(),
        not_after: Utc::now(),
        fingerprint_sha256: sha("voter ca"),
        serial_number: "01".to_owned(),
        pem: "pem".to_owned(),
    };
    assert!(insert_certificate_authority(&tx, record()).await.unwrap());
    assert!(!insert_certificate_authority(&tx, record()).await.unwrap());
    assert_eq!(
        attempt(
            &mut tx,
            "UPDATE sequent_backend.certificate_authority SET purpose = 'voter-sign-in'
             WHERE id = $1",
            &[&voter],
        )
        .await,
        Err("certificate_authority_one_per_purpose".to_owned()),
        "two voter CAs of one certificate"
    );

    // A staff issuer's CRLs go with it.
    let issuer = staff_issuer(&tx, s, "Staff CA").await;
    let crl = staff_crl(&tx, s, issuer, "ok").await;
    let other = event(&tx, s.tenant, id(line!(), 3)).await;
    assert_eq!(
        attempt(
            &mut tx,
            "INSERT INTO sequent_backend.staff_crl
                 (tenant_id, election_event_id, issuer_id, issuer_fingerprint, url, status)
             VALUES ($1, $2, $3, 'x', 'http://crl.example/other.crl', 'ok')",
            &[&other.tenant, &other.event, &issuer],
        )
        .await,
        Err("staff_crl_of_its_issuer".to_owned()),
        "an issuer of another event"
    );
    tx.execute(
        "DELETE FROM sequent_backend.certificate_authority WHERE id = $1",
        &[&issuer],
    )
    .await
    .unwrap();
    assert_eq!(
        count(
            &tx,
            "SELECT count(*) FROM sequent_backend.staff_crl WHERE id = $1",
            &[&crl],
        )
        .await,
        0
    );
}

#[tokio::test]
async fn the_signing_rows_go_with_their_election_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let seed = line!();
    let s = scope(&tx, seed).await;
    let kept = event(&tx, s.tenant, id(seed, 3)).await;
    for scope in [s, kept] {
        upsert_signing_rule(
            &tx,
            scope.tenant,
            scope.event,
            &SigningRule::default_for(SigningAction::OpenVoting),
            0,
            "officer",
            None,
        )
        .await
        .unwrap()
        .unwrap();
        upsert_signing_checks(
            &tx,
            scope.tenant,
            scope.event,
            &SigningChecks::default(),
            0,
            "x",
            None,
        )
        .await
        .unwrap()
        .unwrap();
        let request = insert_request(
            &mut tx,
            &new_request(scope, SigningAction::OpenVoting, "post-1", 1),
        )
        .await
        .unwrap();
        let certificate =
            insert_certificate(&mut tx, &new_certificate(scope, "alice", "a", "a", "a"))
                .await
                .unwrap();
        insert_approval(&mut tx, &new_approval(&request, "alice", &certificate))
            .await
            .unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.signing_document_revision
                 (tenant_id, election_event_id, request_id, revision, document_id, sha256, state)
             VALUES ($1, $2, $3, 0, $4, $5, 'base')",
            &[
                &scope.tenant,
                &scope.event,
                &request.id,
                &Uuid::new_v4(),
                &sha("pdf"),
            ],
        )
        .await
        .unwrap();
        let issuer = staff_issuer(&tx, scope, "Staff CA").await;
        staff_crl(&tx, scope, issuer, "ok").await;
        stage(&tx, &created_step(scope)).await.unwrap();
        // A request of a Post, which the deletion removes before the event.
        let post = Uuid::new_v4();
        election(&tx, scope, post).await;
        let mut of_post = new_request(scope, SigningAction::CloseVoting, "post-2", 2);
        of_post.election_id = Some(post);
        insert_request(&mut tx, &of_post).await.unwrap();
    }
    let tables = [
        "signing_rule",
        "signing_checks",
        "signing_request",
        "signing_approval",
        "signing_document_revision",
        "staff_certificate",
        "staff_crl",
        "signing_log_outbox",
    ];
    for table in tables {
        let sql =
            format!("SELECT count(*) FROM sequent_backend.{table} WHERE election_event_id = $1");
        assert!(count(&tx, &sql, &[&s.event]).await > 0, "{table} is empty");
    }

    delete_election_event(&tx, &s.tenant.to_string(), &s.event.to_string())
        .await
        .unwrap();
    for table in tables {
        let sql =
            format!("SELECT count(*) FROM sequent_backend.{table} WHERE election_event_id = $1");
        assert_eq!(count(&tx, &sql, &[&s.event]).await, 0, "{table} kept rows");
        assert!(
            count(&tx, &sql, &[&kept.event]).await > 0,
            "{table} lost the other event's rows"
        );
    }
}

async fn expire(tx: &Transaction<'_>, s: Scope, request: Uuid) -> Option<SigningRequestRow> {
    update_signing_request_status(
        tx,
        s.tenant,
        s.event,
        request,
        &SigningRequestTransition::Expire,
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn a_request_can_wait_without_a_time_limit_or_expire() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    // No time limit: it never expires.
    let unlimited = insert_request(
        &mut tx,
        &new_request(s, SigningAction::ApproveVoter, "application-0", 1),
    )
    .await
    .unwrap();
    assert_eq!(expire(&tx, s, unlimited.id).await, None);
    // Its time has not come.
    let mut later = new_request(s, SigningAction::ApproveVoter, "application-1", 1);
    later.expires_at = Some(Utc::now() + Duration::minutes(30));
    let later = insert_request(&mut tx, &later).await.unwrap();
    assert_eq!(expire(&tx, s, later.id).await, None);
    // Past its time.
    let mut past = new_request(s, SigningAction::ApproveVoter, "application-2", 1);
    past.expires_at = Some(Utc::now() - Duration::seconds(1));
    let past = insert_request(&mut tx, &past).await.unwrap();
    let expired = expire(&tx, s, past.id).await.unwrap();
    assert_eq!(expired.status, SigningRequestStatus::Expired);
    assert_eq!(expired.cancel_reason, None);
    assert_eq!(expire(&tx, s, past.id).await, None, "expired once");

    let mut zero = new_request(s, SigningAction::ApproveVoter, "application-3", 2);
    zero.required = 0;
    assert_eq!(
        insert_request(&mut tx, &zero).await,
        Err("signing_request_needs_a_signature".to_owned())
    );
    let mut rule = SigningRule::default_for(SigningAction::ApproveVoter);
    rule.expires_minutes = Some(0);
    let savepoint = tx.savepoint("rule").await.unwrap();
    let result =
        upsert_signing_rule(&savepoint, s.tenant, s.event, &rule, 0, "officer", None).await;
    assert_eq!(
        settle(savepoint, result).await.map(|_| ()),
        Err("signing_rule_expires_later".to_owned())
    );
}

/// A tenant with an election event, committed, for tests with several
/// connections. [`forget`] deletes the event.
async fn committed_scope(pool: &Pool, seed: u32) -> Scope {
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let s = scope(&tx, seed).await;
    tx.commit().await.unwrap();
    s
}

async fn forget(pool: &Pool, s: Scope) {
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    delete_election_event(&tx, &s.tenant.to_string(), &s.event.to_string())
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

async fn backend_pid(tx: &Transaction<'_>) -> i32 {
    tx.query_one("SELECT pg_backend_pid()", &[])
        .await
        .unwrap()
        .get(0)
}

/// Returns once the backend `pid` waits for a lock another one holds.
async fn until_blocked(observer: &deadpool_postgres::Client, pid: i32) {
    for _ in 0..1000 {
        let blocked: bool = observer
            .query_one("SELECT cardinality(pg_blocking_pids($1)) > 0", &[&pid])
            .await
            .unwrap()
            .get(0);
        if blocked {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("backend {pid} never waited for a lock");
}

#[tokio::test]
async fn of_two_saves_from_one_revision_exactly_one_wins() {
    let pool = schema::pool().await;
    let s = committed_scope(&pool, line!()).await;
    let observer = pool.get().await.unwrap();
    let mut a = pool.get().await.unwrap();
    let mut b = pool.get().await.unwrap();
    let rule = SigningRule {
        requirement: SigningRequirement::Required,
        signatures: 2,
        ..SigningRule::default_for(SigningAction::CloseVoting)
    };
    // The first save of the action, then a change of revision 1.
    for expected in [0, 1] {
        let tx_a = a.transaction().await.unwrap();
        let tx_b = b.transaction().await.unwrap();
        let pid_b = backend_pid(&tx_b).await;
        let first = upsert_signing_rule(&tx_a, s.tenant, s.event, &rule, expected, "a", None)
            .await
            .unwrap();
        assert_eq!(first.map(|row| row.rule.revision), Some(expected + 1));
        let (second, ()) = tokio::join!(
            upsert_signing_rule(&tx_b, s.tenant, s.event, &rule, expected, "b", None),
            async {
                until_blocked(&observer, pid_b).await;
                tx_a.commit().await.unwrap();
            }
        );
        assert_eq!(
            second.unwrap(),
            None,
            "the second save from revision {expected}"
        );
        tx_b.commit().await.unwrap();
    }
    let tx = a.transaction().await.unwrap();
    let saved = get_signing_rule(&tx, s.tenant, s.event, SigningAction::CloseVoting)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved.rule.revision, 2);
    assert_eq!(saved.updated_by, "a");
    tx.commit().await.unwrap();
    forget(&pool, s).await;
}

#[tokio::test]
async fn a_waiting_request_completes_or_expires_not_both() {
    let pool = schema::pool().await;
    let s = committed_scope(&pool, line!()).await;
    let observer = pool.get().await.unwrap();
    let mut a = pool.get().await.unwrap();
    let mut b = pool.get().await.unwrap();
    let setup = a.transaction().await.unwrap();
    let mut due = new_request(s, SigningAction::OpenVoting, "post-1", 1);
    due.expires_at = Some(Utc::now() - Duration::seconds(1));
    let due = insert_signing_request(&setup, &due).await.unwrap().unwrap();
    setup.commit().await.unwrap();

    let tx_a = a.transaction().await.unwrap();
    let tx_b = b.transaction().await.unwrap();
    let pid_b = backend_pid(&tx_b).await;
    let completed = update_signing_request_status(
        &tx_a,
        s.tenant,
        s.event,
        due.id,
        &SigningRequestTransition::Complete,
    )
    .await
    .unwrap();
    assert!(completed.is_some());
    let (expired, ()) = tokio::join!(
        update_signing_request_status(
            &tx_b,
            s.tenant,
            s.event,
            due.id,
            &SigningRequestTransition::Expire,
        ),
        async {
            until_blocked(&observer, pid_b).await;
            tx_a.commit().await.unwrap();
        }
    );
    assert_eq!(expired.unwrap(), None, "the expiry after the completion");
    tx_b.commit().await.unwrap();

    let tx = a.transaction().await.unwrap();
    let request = get_signing_request(&tx, s.tenant, s.event, due.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(request.status, SigningRequestStatus::Completed);
    tx.commit().await.unwrap();
    forget(&pool, s).await;
}

#[tokio::test]
async fn an_approval_holds_off_the_revocation_of_its_certificate() {
    let pool = schema::pool().await;
    let s = committed_scope(&pool, line!()).await;
    let mut a = pool.get().await.unwrap();
    let mut b = pool.get().await.unwrap();
    let mut c = pool.get().await.unwrap();
    let setup = a.transaction().await.unwrap();
    let certificate = insert_staff_certificate(&setup, &new_certificate(s, "alice", "a", "a", "a"))
        .await
        .unwrap();
    setup.commit().await.unwrap();

    // Two approvals sign with it at once.
    let tx_a = a.transaction().await.unwrap();
    let tx_b = b.transaction().await.unwrap();
    for tx in [&tx_a, &tx_b] {
        let locked = lock_staff_certificate_for_share(tx, s.tenant, s.event, certificate.id)
            .await
            .unwrap();
        assert_eq!(locked.map(|row| row.id), Some(certificate.id));
    }
    // A revocation waits for them.
    let tx_c = c.transaction().await.unwrap();
    tx_c.batch_execute("SET LOCAL lock_timeout = '100ms'")
        .await
        .unwrap();
    let error = lock_staff_certificate_for_update(&tx_c, s.tenant, s.event, certificate.id)
        .await
        .unwrap_err();
    let database = error
        .downcast_ref::<tokio_postgres::Error>()
        .and_then(tokio_postgres::Error::as_db_error)
        .unwrap_or_else(|| panic!("not a database error: {error:?}"));
    assert_eq!(database.code().code(), "55P03", "lock_not_available");
    tx_c.rollback().await.unwrap();
    tx_a.commit().await.unwrap();
    tx_b.commit().await.unwrap();

    let tx_c = c.transaction().await.unwrap();
    let locked = lock_staff_certificate_for_update(&tx_c, s.tenant, s.event, certificate.id)
        .await
        .unwrap();
    assert_eq!(locked.map(|row| row.id), Some(certificate.id));
    tx_c.commit().await.unwrap();
    forget(&pool, s).await;
}

#[tokio::test]
async fn a_step_is_staged_after_the_signing_writes_of_its_event_commit() {
    let pool = schema::pool().await;
    let s = committed_scope(&pool, line!()).await;
    let elsewhere = committed_scope(&pool, line!()).await;
    let observer = pool.get().await.unwrap();
    let mut a = pool.get().await.unwrap();
    let mut b = pool.get().await.unwrap();

    let tx_a = a.transaction().await.unwrap();
    lock_signing_event(&tx_a, s.tenant, s.event).await.unwrap();
    // Taking it again in the same transaction doesn't wait.
    lock_signing_event(&tx_a, s.tenant, s.event).await.unwrap();
    let tx_b = b.transaction().await.unwrap();
    // Another event's steps don't wait for it.
    stage(&tx_b, &created_step(elsewhere)).await.unwrap();
    let pid_b = backend_pid(&tx_b).await;
    let waiting_step = created_step(s);
    let (staged, ()) = tokio::join!(stage(&tx_b, &waiting_step), async {
        until_blocked(&observer, pid_b).await;
        stage(&tx_a, &created_step(s)).await.unwrap();
        tx_a.commit().await.unwrap();
    });
    let second = staged.unwrap();
    tx_b.commit().await.unwrap();

    // Queued in commit order: the step that held the lock first.
    let tx = a.transaction().await.unwrap();
    let steps = fetch_unposted_signing_log_outbox(&tx, s.tenant, s.event, 10)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.step_id)
        .collect::<Vec<_>>();
    assert_eq!(steps.len(), 4);
    assert_ne!(steps[0], second);
    assert_eq!(steps[0], steps[1]);
    assert_eq!(&steps[2..], [second, second]);
    tx.commit().await.unwrap();
    forget(&pool, s).await;
    forget(&pool, elsewhere).await;
}
