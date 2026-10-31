// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Registered staff certificates (design §5a): a Security Officer registers
//! a certificate to a person, or links the same person's certificate to
//! their second account, and revokes keys; a signer reads their own
//! registrations; the signing dialog dry-runs every check before signing.

use crate::postgres::signing::{
    find_active_staff_certificates_by_fingerprint, find_active_staff_certificates_by_holder,
    find_active_staff_certificates_by_spki, find_revoked_staff_certificates_by_fingerprint,
    find_revoked_staff_certificates_by_spki, get_signing_checks, insert_staff_certificate,
    list_signing_approvals, lock_signing_event, lock_signing_request,
    lock_staff_certificate_for_update, update_signing_request_status, NewStaffCertificate,
    SigningRequestRow, SigningRequestTransition, StaffCertificateRow,
};
use crate::postgres::signing_certificates::{
    election_in_event, get_staff_certificate, revoke_staff_certificates_of_key,
    waiting_requests_signed_by_key,
};
use crate::services::signing::allowed_by_permission;
use crate::services::signing::certificates::{
    conflicting_registrations, key_usage_check, load_staff_anchors, lock_certificate_identities,
    lock_identity_keys, parse_chain, root_account, validity_check, verified_path,
    CertificateIdentity, CertificateVerification, CertificateVerificationInput,
    CertificateVerifier, OtherHolder,
};
use crate::services::signing::crl::{refresh_chain_crls, CrlFetcher};
use crate::services::signing::log::{stage, Actor, LogScope, LogStep, SystemOutcome};
use crate::services::signing::requests::voided_approvals;
use anyhow::Result;
use chrono::{DateTime, Utc};
use deadpool_postgres::{Client, Transaction};
use electoral_log::messages::newtypes::SigningStatementKind;
use openssl::x509::X509;
use sequent_core::signing::{CancelReason, RevocationCheck, StaffCertificateRegistration};
use sequent_core::types::permissions::Permissions;
use serde::Serialize;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use strum_macros::{Display, EnumString};
use tracing::instrument;
use uuid::Uuid;

/// The longest revocation reason kept.
pub const MAX_REVOKE_REASON_CHARS: usize = 500;

/// Why a Security Officer's registration is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Display, EnumString)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum RegistrationRefusalReason {
    Unreadable,
    UntrustedIssuer,
    NotValidNow,
    NotForSigning,
    UnknownPost,
    AlreadyRegistered,
    /// The certificate or its key was revoked: it never signs again.
    Revoked,
    /// The key or holder is registered to another account, and the request
    /// doesn't link to exactly that account.
    RegisteredToOther,
    /// A link was asked for, but nobody else holds the certificate.
    NothingToLink,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationRefused {
    pub reason: RegistrationRefusalReason,
    pub detail: String,
    /// The account holding the key or holder, for `RegisteredToOther`.
    pub other_holder: Option<OtherHolder>,
}

type Registration = std::result::Result<StaffCertificateRow, RegistrationRefused>;

fn refused(reason: RegistrationRefusalReason, detail: impl Into<String>) -> Result<Registration> {
    Ok(Err(RegistrationRefused {
        reason,
        detail: detail.into(),
        other_holder: None,
    }))
}

/// A Security Officer's registration of a certificate to a person.
#[derive(Debug, Clone)]
pub struct StaffCertificateRegistrationInput {
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    /// The person the certificate is registered to.
    pub holder: Actor,
    /// Their display name.
    pub holder_name: Option<String>,
    /// The Post it signs for; `None`: every Post and the event.
    pub election_id: Option<Uuid>,
    /// The certificate first, then any intermediates.
    pub chain_pem: Vec<String>,
    /// The account the certificate is already registered to, to link it to
    /// the same person's second account (`holder`).
    pub linked_to: Option<String>,
}

/// Whom a registration is for: their display name, or the username.
fn person_name(row: &StaffCertificateRow) -> String {
    row.user_display_name
        .clone()
        .unwrap_or_else(|| row.username.clone())
}

fn event_scope(tenant_id: Uuid, election_event_id: Uuid, election_id: Option<Uuid>) -> LogScope {
    LogScope {
        tenant_id,
        election_event_id,
        election_id,
        area_id: None,
    }
}

/// The checks of a certificate a Security Officer registers that don't
/// depend on registrations; `Err` refuses it.
fn check_certificate_itself(
    chain: &[X509],
    anchors: &[X509],
    now: DateTime<Utc>,
) -> Result<std::result::Result<CertificateIdentity, (RegistrationRefusalReason, String)>> {
    use RegistrationRefusalReason as Reason;
    let (leaf, intermediates) = (&chain[0], &chain[1..]);
    let identity = match CertificateIdentity::of(leaf, &[]) {
        Ok(identity) => identity,
        Err(err) => return Ok(Err((Reason::Unreadable, format!("{err:#}")))),
    };
    if let Err(reason) = verified_path(leaf, intermediates, anchors)? {
        return Ok(Err((Reason::UntrustedIssuer, reason)));
    }
    let validity = validity_check(std::slice::from_ref(leaf), now)?;
    if !validity.ok {
        return Ok(Err((
            Reason::NotValidNow,
            validity.detail.unwrap_or_default(),
        )));
    }
    let usage = key_usage_check(leaf, &identity)?;
    if !usage.ok {
        return Ok(Err((
            Reason::NotForSigning,
            usage.detail.unwrap_or_default(),
        )));
    }
    Ok(Ok(identity))
}

/// Registers a certificate to a person, as a Security Officer. The
/// certificate must chain to a staff issuer, be valid at `now`, be made for
/// signing and not be revoked (nor its key). Its key and holder may be
/// registered to another account only when `linked_to` names exactly that
/// account (the same person's accounts, decided O1).
#[instrument(skip(hasura_transaction, input), err)]
pub async fn register_staff_certificate(
    hasura_transaction: &Transaction<'_>,
    input: &StaffCertificateRegistrationInput,
    officer: &Actor,
    officer_name: Option<&str>,
    now: DateTime<Utc>,
) -> Result<Registration> {
    use RegistrationRefusalReason as Reason;
    let (tenant_id, election_event_id) = (input.tenant_id, input.election_event_id);
    lock_signing_event(hasura_transaction, tenant_id, election_event_id).await?;
    let chain = match parse_chain(&input.chain_pem) {
        Ok(chain) => chain,
        Err(err) => return refused(Reason::Unreadable, format!("{err}")),
    };
    let anchors: Vec<X509> = load_staff_anchors(hasura_transaction, tenant_id, election_event_id)
        .await?
        .into_iter()
        .map(|(_, anchor)| anchor)
        .collect();
    // The parsers' errors refuse the certificate as unreadable.
    let identity = match check_certificate_itself(&chain, &anchors, now) {
        Ok(Ok(identity)) => identity,
        Ok(Err((reason, detail))) => return refused(reason, detail),
        Err(err) => return refused(Reason::Unreadable, format!("{err:#}")),
    };
    if let Some(election_id) = input.election_id {
        if !election_in_event(
            hasura_transaction,
            tenant_id,
            election_event_id,
            election_id,
        )
        .await?
        {
            return refused(
                Reason::UnknownPost,
                "the Post is not in this election event",
            );
        }
    }

    lock_certificate_identities(hasura_transaction, tenant_id, &identity).await?;
    let revoked = !find_revoked_staff_certificates_by_fingerprint(
        hasura_transaction,
        tenant_id,
        &identity.fingerprint_sha256,
    )
    .await?
    .is_empty()
        || !find_revoked_staff_certificates_by_spki(
            hasura_transaction,
            tenant_id,
            &identity.spki_sha256,
        )
        .await?
        .is_empty();
    if revoked {
        return refused(Reason::Revoked, "this certificate or its key was revoked");
    }
    let mut registrations = find_active_staff_certificates_by_fingerprint(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &identity.fingerprint_sha256,
    )
    .await?;
    registrations.extend(
        find_active_staff_certificates_by_spki(
            hasura_transaction,
            tenant_id,
            &identity.spki_sha256,
        )
        .await?,
    );
    registrations.extend(
        find_active_staff_certificates_by_holder(
            hasura_transaction,
            tenant_id,
            &identity.holder_sha256,
        )
        .await?,
    );
    let holder = &input.holder;
    if registrations.iter().any(|row| {
        row.user_id == holder.user_id
            && row.election_event_id == election_event_id
            && row.fingerprint_sha256 == identity.fingerprint_sha256
    }) {
        return refused(
            Reason::AlreadyRegistered,
            format!("already registered to {}", holder.username),
        );
    }
    let conflicts = conflicting_registrations(&registrations, &identity, &holder.user_id);
    let roots: BTreeSet<&str> = conflicts.iter().map(|row| root_account(row)).collect();
    let linked = match (conflicts.first(), &input.linked_to) {
        (None, None) => None,
        (None, Some(_)) => {
            return refused(
                Reason::NothingToLink,
                "the certificate is not registered to anyone else",
            )
        }
        (Some(_), Some(linked_to)) if roots == BTreeSet::from([linked_to.as_str()]) => {
            let name = registrations
                .iter()
                .find(|row| row.user_id == *linked_to)
                .map_or_else(|| linked_to.clone(), person_name);
            Some((linked_to.clone(), name))
        }
        (Some(row), _) => {
            return Ok(Err(RegistrationRefused {
                reason: Reason::RegisteredToOther,
                detail: person_name(row),
                other_holder: Some(OtherHolder {
                    user_id: row.user_id.clone(),
                    display_name: person_name(row),
                }),
            }))
        }
    };

    let row = insert_staff_certificate(
        hasura_transaction,
        &NewStaffCertificate {
            tenant_id,
            election_event_id,
            user_id: holder.user_id.clone(),
            username: holder.username.clone(),
            user_display_name: input.holder_name.clone(),
            election_id: input.election_id,
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
            linked_to: linked.as_ref().map(|(user_id, _)| user_id.clone()),
            registered_by: officer.user_id.clone(),
            registered_by_name: officer_name.map(str::to_owned),
        },
    )
    .await?;
    let holder_name = person_name(&row);
    let description = match &linked {
        Some((_, linked_name)) => format!(
            "Linked certificate {} ({}) of {} to {}",
            identity.common_name,
            identity.short_fingerprint(),
            linked_name,
            holder_name
        ),
        None => format!(
            "Registered certificate {} ({}) to {}",
            identity.common_name,
            identity.short_fingerprint(),
            holder_name
        ),
    };
    stage(
        hasura_transaction,
        &LogStep {
            kind: SigningStatementKind::SigningCertificateRegistered,
            user: officer.clone(),
            system: SystemOutcome::Info,
            scope: event_scope(tenant_id, election_event_id, input.election_id),
            description,
            details: json!({
                "election_id": input.election_id,
                "certificate": identity.log_details(),
                "certificate_id": row.id,
                "user_id": row.user_id,
                "username": row.username,
                "registration": row.registration,
                "linked_to": row.linked_to,
                "allowed_by": allowed_by_permission(Permissions::SIGNING_CERTIFICATES_REGISTER),
            }),
        },
    )
    .await?;
    Ok(Ok(row))
}

/// What a revocation did.
#[derive(Debug, Clone, PartialEq)]
pub enum RevokeOutcome {
    /// Revoked: the registration, every registration of its key in the
    /// tenant (it included), and the waiting requests the key had signed,
    /// in any of their events, which are cancelled.
    Revoked {
        certificate: StaffCertificateRow,
        revoked: Vec<StaffCertificateRow>,
        cancelled_requests: Vec<SigningRequestRow>,
    },
    NotFound,
    AlreadyRevoked(StaffCertificateRow),
}

fn common_name_of(row: &StaffCertificateRow) -> String {
    X509::from_pem(row.pem.as_bytes())
        .ok()
        .and_then(|certificate| CertificateIdentity::of(&certificate, &[]).ok())
        .map_or_else(|| row.subject.clone(), |identity| identity.common_name)
}

/// Revokes a registration's key, as a Security Officer, for `reason`
/// (trimmed, required): every active registration of the key in the tenant,
/// in every event and account, is revoked, and each event logs it. Every
/// waiting request the key signed is cancelled (`certificate-revoked`):
/// its signatures no longer count.
///
/// Locks: the signing locks of every event with a registration of the key,
/// in id order, then the key and holder, then the registration's row (it
/// waits for approvals signing with it), then the requests, in id order.
#[instrument(skip(hasura_transaction, reason), err)]
pub async fn revoke_registration(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    id: Uuid,
    reason: &str,
    officer: &Actor,
    officer_name: Option<&str>,
) -> Result<RevokeOutcome> {
    let reason = reason.trim();
    if reason.is_empty() || reason.chars().count() > MAX_REVOKE_REASON_CHARS {
        anyhow::bail!(
            "a revocation needs a reason of at most {MAX_REVOKE_REASON_CHARS} characters"
        );
    }
    let Some(target) =
        get_staff_certificate(hasura_transaction, tenant_id, election_event_id, id).await?
    else {
        return Ok(RevokeOutcome::NotFound);
    };
    let mut events: BTreeSet<Uuid> = BTreeSet::from([election_event_id]);
    events.extend(
        find_active_staff_certificates_by_spki(hasura_transaction, tenant_id, &target.spki_sha256)
            .await?
            .iter()
            .map(|row| row.election_event_id),
    );
    for event in &events {
        lock_signing_event(hasura_transaction, tenant_id, *event).await?;
    }
    lock_identity_keys(
        hasura_transaction,
        tenant_id,
        &target.spki_sha256,
        &target.holder_sha256,
    )
    .await?;
    let Some(current) =
        lock_staff_certificate_for_update(hasura_transaction, tenant_id, election_event_id, id)
            .await?
    else {
        return Ok(RevokeOutcome::NotFound);
    };
    if current.status != sequent_core::signing::StaffCertificateStatus::Active {
        return Ok(RevokeOutcome::AlreadyRevoked(current));
    }
    // A registration of the key made in another event before the locks
    // (none can be made after them) still has its event locked here, in
    // id order after the ones above.
    for row in
        find_active_staff_certificates_by_spki(hasura_transaction, tenant_id, &target.spki_sha256)
            .await?
    {
        if events.insert(row.election_event_id) {
            lock_signing_event(hasura_transaction, tenant_id, row.election_event_id).await?;
        }
    }
    let revoked = revoke_staff_certificates_of_key(
        hasura_transaction,
        tenant_id,
        &target.spki_sha256,
        &officer.user_id,
        officer_name,
        reason,
    )
    .await?;
    let mut by_event: BTreeMap<Uuid, Vec<&StaffCertificateRow>> = BTreeMap::new();
    for row in &revoked {
        by_event.entry(row.election_event_id).or_default().push(row);
    }
    let mut cancelled_requests = vec![];
    for (event, rows) in by_event {
        let first = rows[0];
        let names: BTreeSet<String> = rows.iter().map(|row| person_name(row)).collect();
        let name = common_name_of(first);
        let mut cancelled = vec![];
        for request_id in waiting_requests_signed_by_key(
            hasura_transaction,
            tenant_id,
            event,
            &target.spki_sha256,
        )
        .await?
        {
            if let Some(request) = cancel_for_revocation(
                hasura_transaction,
                tenant_id,
                event,
                request_id,
                officer,
                &name,
            )
            .await?
            {
                cancelled.push(request);
            }
        }
        stage(
            hasura_transaction,
            &LogStep {
                kind: SigningStatementKind::SigningCertificateRevoked,
                user: officer.clone(),
                system: SystemOutcome::Info,
                scope: event_scope(
                    tenant_id,
                    event,
                    (rows.len() == 1).then_some(first.election_id).flatten(),
                ),
                description: format!(
                    "Revoked certificate {name} of {}: {reason}",
                    names.into_iter().collect::<Vec<_>>().join(", ")
                ),
                details: json!({
                    "certificates": rows.iter().map(|row| json!({
                        "certificate_id": row.id,
                        "subject": row.subject,
                        "issuer": row.issuer,
                        "serial": row.serial,
                        "fingerprint": row.fingerprint_sha256,
                        "user_id": row.user_id,
                        "username": row.username,
                        "election_id": row.election_id,
                    })).collect::<Vec<_>>(),
                    "key": target.spki_sha256,
                    "reason": reason,
                    "cancelled_requests": cancelled.iter().map(|request| request.id).collect::<Vec<_>>(),
                    "allowed_by": allowed_by_permission(Permissions::SIGNING_CERTIFICATES_REVOKE),
                }),
            },
        )
        .await?;
        cancelled_requests.extend(cancelled);
    }
    let certificate = revoked
        .iter()
        .find(|row| row.id == id)
        .cloned()
        .unwrap_or(current);
    Ok(RevokeOutcome::Revoked {
        certificate,
        revoked,
        cancelled_requests,
    })
}

/// Cancels a waiting request the revoked key signed, and stages its
/// SigningRequestCancelled entries (USER: the Security Officer). `None`
/// when it is no longer waiting.
async fn cancel_for_revocation(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    request_id: Uuid,
    officer: &Actor,
    certificate_name: &str,
) -> Result<Option<SigningRequestRow>> {
    let Some(request) =
        lock_signing_request(hasura_transaction, tenant_id, election_event_id, request_id).await?
    else {
        return Ok(None);
    };
    let Some(cancelled) = update_signing_request_status(
        hasura_transaction,
        tenant_id,
        election_event_id,
        request.id,
        &SigningRequestTransition::Cancel {
            reason: CancelReason::CertificateRevoked,
            by: Some(officer.user_id.clone()),
        },
    )
    .await?
    else {
        return Ok(None);
    };
    let approvals = list_signing_approvals(
        hasura_transaction,
        tenant_id,
        election_event_id,
        cancelled.id,
    )
    .await?;
    stage(
        hasura_transaction,
        &LogStep {
            kind: SigningStatementKind::SigningRequestCancelled,
            user: officer.clone(),
            system: SystemOutcome::Info,
            scope: LogScope {
                tenant_id,
                election_event_id,
                election_id: cancelled.election_id,
                area_id: cancelled.area_id,
            },
            description: format!(
                "Cancelled signing request {}: the certificate {certificate_name} that signed it was revoked",
                cancelled.code
            ),
            details: json!({
                "election_id": cancelled.election_id,
                "area_id": cancelled.area_id,
                "action": cancelled.action,
                "request_id": cancelled.id,
                "code": cancelled.code,
                "reason": CancelReason::CertificateRevoked,
                "voided_approvals": voided_approvals(&approvals),
                "allowed_by": allowed_by_permission(Permissions::SIGNING_CERTIFICATES_REVOKE),
            }),
        },
    )
    .await?;
    Ok(Some(cancelled))
}

/// The dry run the signing dialog shows before signing: downloads the
/// chain's missing revocation lists (when the event checks revocation)
/// outside any transaction, then runs every check but the signature in a
/// transaction of its own, which it rolls back: nothing is registered.
#[instrument(skip(client, verifier, fetcher, request, chain_pem), err)]
pub async fn check_certificate(
    client: &mut Client,
    verifier: &dyn CertificateVerifier,
    fetcher: &dyn CrlFetcher,
    request: &SigningRequestRow,
    signer: Actor,
    chain_pem: Vec<String>,
    now: DateTime<Utc>,
) -> Result<CertificateVerification> {
    let checks = {
        let transaction = client.transaction().await?;
        get_signing_checks(&transaction, request.tenant_id, request.election_event_id)
            .await?
            .map(|row| row.checks)
            .unwrap_or_default()
    };
    if checks.revocation_check == RevocationCheck::Check {
        refresh_chain_crls(
            client,
            request.tenant_id,
            request.election_event_id,
            &chain_pem,
            fetcher,
            now,
        )
        .await?;
    }
    let transaction = client.transaction().await?;
    let verification = verifier
        .verify(
            &transaction,
            &CertificateVerificationInput {
                request,
                signer,
                chain_pem,
                signatures: None,
                now,
            },
        )
        .await?;
    transaction.rollback().await?;
    Ok(verification)
}
