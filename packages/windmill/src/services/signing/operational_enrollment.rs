// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! First-use enrollment intent. No hardware, staff certificate authorization,
//! device grant, or signature is produced by this endpoint.

use super::approve::sign_refusal;
use super::operational_device::matches_canonical_request;
use super::operational_policy::{signing_mode_for_event, OperationalSigningMode};
use super::requests::{is_overdue, read_request};
use super::{SigningCaller, SigningError, SigningResult};
use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use deadpool_postgres::Transaction;
use sequent_core::signing::{SigningRequestStatus, SigningRule};
use sequent_core::types::permissions::Permissions;
use uuid::Uuid;

fn unavailable() -> SigningError {
    SigningError::Forbidden("Signing setup is unavailable for this request.".into())
}

fn fresh_gold(acr: &str, auth_time: Option<i64>, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let time = DateTime::from_timestamp(auth_time?, 0)?;
    (acr == Permissions::GOLD.to_string()
        && time <= now
        && now.signed_duration_since(time) < Duration::seconds(60))
        .then_some(time)
}

#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EnrollmentPhase {
    AwaitingIndependentOfficer,
    HardwareGenerationPending,
    CertificatePending,
    CertificateOnFile,
    ReconciliationRequired,
}

pub struct RequestedEnrollment {
    pub ticket: Uuid,
    pub official: Uuid,
    pub phase: EnrollmentPhase,
    pub event: Uuid,
    pub post: Option<Uuid>,
}

/// Only call with verified Portal OIDC claims and the authenticated tenant.
/// The client provides a request ID, never a holder, a device or a certificate.
pub async fn request_enrollment(
    tx: &Transaction<'_>,
    caller: &SigningCaller,
    tenant: Uuid,
    request_id: Uuid,
    reviewed_payload_sha256: &str,
    acr: &str,
    auth_time: Option<i64>,
) -> SigningResult<RequestedEnrollment> {
    let auth_time = fresh_gold(acr, auth_time, Utc::now()).ok_or_else(unavailable)?;
    let official = Uuid::parse_str(&caller.user_id)
        .ok()
        .filter(|id| !id.is_nil() && id.to_string() == caller.user_id)
        .ok_or_else(unavailable)?;
    let initial = read_request(tx, tenant, request_id).await.map_err(|error| match error {
        SigningError::NotFound(_) => unavailable(),
        other => other,
    })?;
    if signing_mode_for_event(tx, tenant, initial.election_event_id)
        .await
        .context("Read event signing mode")?
        != OperationalSigningMode::HsmRequired
    {
        return Err(unavailable());
    }
    // The narrow actor connection needs no signing-request UPDATE grants.
    // Same event lock as the approval guard; the SQL definer takes the
    // exact request row lock after identity, rule and canonical review.
    crate::postgres::signing::lock_signing_event(tx, tenant, initial.election_event_id)
        .await.context("Lock setup event")?;
    let request = read_request(tx, tenant, request_id).await?;
    if request.election_event_id != initial.election_event_id {
        return Err(unavailable());
    }
    let rule: SigningRule = serde_json::from_value(request.rule_snapshot.clone())
        .context("Read frozen signing rule for enrollment")?;
    if request.status != SigningRequestStatus::Waiting
        || is_overdue(tx, &request).await?
        || rule.action != request.action
        || rule.validate().is_err()
        || !rule.is_required()
        || i32::from(rule.required()) != request.required
        || request.payload_sha256 != reviewed_payload_sha256
        || !matches_canonical_request(&request)
        || sign_refusal(tx, caller, &request).await?.is_some()
    {
        return Err(unavailable());
    }
    let event = request.election_event_id;
    let post = request.election_id;
    let proposed = Uuid::new_v4();
    let ticket: Option<Uuid> = tx
        .query_one(
            "SELECT sequent_backend.operational_request_event_setup($1,$2,$3,$4,$5,$6,$7,$8)",
            &[&tenant, &event, &request_id, &official, &post, &request.payload_sha256, &auth_time, &proposed],
        )
        .await
        .context("Record actor-bound enrollment request")?
        .try_get(0)
        .context("Read enrollment request result")?;
    let ticket = ticket.ok_or_else(unavailable)?;
    let phase: Option<String> = tx.query_one(
        "SELECT sequent_backend.operational_setup_request_phase($1,$2,$3,$4,$5)",
        &[&tenant,&event,&request_id,&official,&ticket],
    ).await.context("Read durable setup state")?.try_get(0)
       .context("Read setup phase result")?;
    let phase = match phase.as_deref() {
        Some("awaiting-independent-officer") => EnrollmentPhase::AwaitingIndependentOfficer,
        Some("hardware-generation-pending") => EnrollmentPhase::HardwareGenerationPending,
        Some("certificate-pending") => EnrollmentPhase::CertificatePending,
        Some("certificate-on-file") => EnrollmentPhase::CertificateOnFile,
        Some("reconciliation-required") => EnrollmentPhase::ReconciliationRequired,
        _ => return Err(unavailable()),
    };
    Ok(RequestedEnrollment {
        ticket,
        official,
        event,
        post,
        phase,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_gold_and_nonfuture_timestamp_in_strict_window() {
        let now = DateTime::from_timestamp(1_000, 0).unwrap();
        let gold = Permissions::GOLD.to_string();
        assert!(fresh_gold(&gold, Some(941), now).is_some());
        assert!(fresh_gold(&gold, Some(1_000), now).is_some());
        for time in [None, Some(940), Some(1_001), Some(i64::MAX)] {
            assert!(fresh_gold(&gold, time, now).is_none());
        }
        assert!(fresh_gold("silver", Some(1_000), now).is_none());
    }
}

/// Public part of the exact attested key needed by the distinct staff CA
/// operator. This type is never supplied by a public browser.
pub struct SetupCertificateMaterial {
    pub official: Uuid,
    pub post: Option<Uuid>,
    pub initial_attempt: Option<Uuid>,
    pub enrollment: Uuid,
    pub spki: String,
    pub csr: String,
}

/// Only the operator's separately provisioned setup authority login can
/// obtain this public CSR, and then only for the live, exact consent.
pub async fn setup_certificate_material(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    ticket: Uuid,
    verified_officer: Uuid,
) -> anyhow::Result<Option<SetupCertificateMaterial>> {
    tx.query_opt(
        "SELECT official,post,initial_attempt,enrollment,spki,csr
           FROM sequent_backend.operational_setup_certificate_material($1,$2,$3,$4)",
        &[&tenant, &event, &ticket, &verified_officer],
    )
    .await?
    .map(|row| {
        Ok(SetupCertificateMaterial {
            official: row.try_get(0)?,
            post: row.try_get(1)?,
            initial_attempt: row.try_get(2)?,
            enrollment: row.try_get(3)?,
            spki: row.try_get(4)?,
            csr: row.try_get(5)?,
        })
    })
    .transpose()
}

/// Register and atomically bind a privately delivered, externally issued
/// staff CA certificate to the database-pinned attested YubiHSM key. The
/// caller authenticates and authorizes the independent officer before this
/// function, uses its dedicated setup authority database capability, and
/// commits only if this function returns Ok. Do not expose this as an
/// Admin Portal upload. No key generation or approval is possible here.
pub async fn attach_setup_certificate(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    ticket: Uuid,
    officer: &super::log::Actor,
    holder: super::log::Actor,
    chain_pem: Vec<String>,
) -> anyhow::Result<Uuid> {
    use super::certificates::{
        key_usage_check, load_staff_anchors, operational_revocation_until, parse_chain,
        validity_check, verified_path, CertificateIdentity, StoredCrl,
    };
    use super::staff_certificates::{register_staff_certificate, StaffCertificateRegistrationInput};
    use anyhow::{anyhow, ensure};
    use openssl::nid::Nid;
    use openssl::x509::X509NameRef;
    use sequent_core::signing::SignatureAlgorithm;

    let verified_officer = Uuid::parse_str(&officer.user_id)?;
    ensure!(!verified_officer.is_nil() && verified_officer.to_string() == officer.user_id);
    let material = setup_certificate_material(tx, tenant, event, ticket, verified_officer)
        .await?
        .context("No exact live operational setup certificate scope")?;
    ensure!(holder.user_id == material.official.to_string(), "certificate holder scope mismatch");
    let chain = parse_chain(&chain_pem)?;
    let leaf = chain.first().context("No issuer leaf")?;
    fn subject_matches(subject: &X509NameRef, nid: Nid, expected: &str) -> bool {
        let mut entries = subject.entries_by_nid(nid);
        entries.next().is_some_and(|entry| {
            entry.data().as_utf8().is_ok_and(|value| value.to_string() == expected)
        }) && entries.next().is_none()
    }
    ensure!(
        subject_matches(leaf.subject_name(), Nid::COMMONNAME, &holder.user_id)
            && subject_matches(leaf.subject_name(), Nid::ORGANIZATIONALUNITNAME, &tenant.to_string()),
        "the CA leaf must preserve the attested request's official and tenant subject"
    );
    let identity = CertificateIdentity::of(leaf, &chain)?;
    ensure!(identity.spki_sha256 == material.spki, "CA leaf does not match this attested key");
    ensure!(identity.key_algorithm == Some(SignatureAlgorithm::EcdsaP256Sha256));
    ensure!(key_usage_check(leaf, &identity)?.ok);
    let anchors = load_staff_anchors(tx, tenant, event)
        .await?.into_iter().map(|(_, anchor)| anchor).collect::<Vec<_>>();
    let path = verified_path(leaf, &chain[1..], &anchors)?
        .map_err(|_| anyhow!("certificate is not trusted by this event's staff issuers"))?;
    let now = Utc::now();
    ensure!(validity_check(&path, now)?.ok);
    let crls = crate::postgres::signing_certificates::list_staff_crls(tx, tenant, event)
        .await?.into_iter().filter_map(|row| {
            Some(StoredCrl { url: row.url, der: row.der, fetched_at: row.last_ok_at? })
        }).collect::<Vec<_>>();
    let usable_until = operational_revocation_until(&path, &crls, now)
        .context("No current positive staff CA revocation assessment")?;
    let encoded = chain.iter()
        .map(|cert| Ok(String::from_utf8(cert.to_pem()?)?))
        .collect::<anyhow::Result<Vec<String>>>()?;
    let row = register_staff_certificate(
        tx,
        &StaffCertificateRegistrationInput {
            tenant_id: tenant,
            election_event_id: event,
            holder,
            holder_name: None,
            election_id: material.post,
            chain_pem: encoded.clone(),
            linked_to: None,
        },
        officer,
        None,
        now,
    )
    .await?
    .map_err(|refused| anyhow!("staff CA registration refused: {}", refused.reason))?;
    ensure!(usable_until > Utc::now(), "certificate assessment expired before binding");
    let attached = tx.query_one(
        "SELECT sequent_backend.operational_attach_requested_setup_certificate($1,$2,$3,$4,$5,$6,$7)",
        &[&tenant,&event,&ticket,&verified_officer,&row.id,&identity.fingerprint_sha256,&encoded],
    ).await?.try_get::<_,bool>(0)?;
    ensure!(attached, "staff certificate was not attached to this hardware enrollment");
    Ok(row.id)
}

/// Exact device delivery, returned only after a verified independent officer
/// approved the ordinary actor's existing ticket. Commit before delivery.
#[derive(serde::Serialize)]
pub struct SetupDispatch {
    pub tenant: Uuid,
    pub event: Uuid,
    pub official: Uuid,
    pub attempt: Uuid,
}

/// The private dispatcher login is allowed this operation but cannot claim a
/// device, sign, register a staff CA certificate or override a signing rule.
pub async fn prepare_setup_dispatch(
    tx: &Transaction<'_>,
    officer: &SigningCaller,
    tenant: Uuid,
    event: Uuid,
    ticket: Uuid,
    acr: &str,
    auth_time: Option<i64>,
) -> SigningResult<SetupDispatch> {
    if !officer.has(Permissions::SIGNING_CERTIFICATES_REGISTER) {
        return Err(unavailable());
    }
    let auth_time = fresh_gold(acr, auth_time, Utc::now()).ok_or_else(unavailable)?;
    let officer_id = Uuid::parse_str(&officer.user_id)
        .ok()
        .filter(|id| !id.is_nil() && id.to_string() == officer.user_id)
        .ok_or_else(unavailable)?;
    let attempt = Uuid::new_v4();
    let official: Option<Uuid> = tx.query_one(
        "SELECT sequent_backend.operational_dispatch_event_setup($1,$2,$3,$4,$5,$6)",
        &[&tenant, &event, &ticket, &officer_id, &attempt, &auth_time],
    ).await.context("Record independent officer dispatch")?.try_get(0)
        .context("Read the authorized device delivery")?;
    Ok(SetupDispatch { tenant, event, official: official.ok_or_else(unavailable)?, attempt })
}

/// Recovery is an explicit independent Gold officer action. It releases an
/// expired dispatch only after SQL proves no key operation was claimed.
pub async fn reconcile_unclaimed_setup(
    tx: &Transaction<'_>, officer: &SigningCaller, tenant: Uuid, event: Uuid,
    ticket: Uuid, acr: &str, auth_time: Option<i64>,
) -> SigningResult<()> {
    if !officer.has(Permissions::SIGNING_CERTIFICATES_REGISTER) {
        return Err(unavailable());
    }
    let auth_time = fresh_gold(acr, auth_time, Utc::now()).ok_or_else(unavailable)?;
    let officer_id = Uuid::parse_str(&officer.user_id).ok()
        .filter(|id| !id.is_nil() && id.to_string() == officer.user_id).ok_or_else(unavailable)?;
    let released: bool = tx.query_one(
        "SELECT sequent_backend.operational_reconcile_unclaimed_setup($1,$2,$3,$4,$5)",
        &[&tenant,&event,&ticket,&officer_id,&auth_time],
    ).await.context("Check expired unclaimed setup")?.try_get(0)
       .context("Read setup reconciliation result")?;
    if !released { return Err(unavailable()); }
    Ok(())
}
