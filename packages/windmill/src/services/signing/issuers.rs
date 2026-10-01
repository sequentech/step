// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The Certificates settings of an event: its trusted staff issuers and its
//! certificate checks. Every change takes the event's signing lock first and
//! stages its two log entries in the same transaction.

use crate::postgres::certificate_authority::{
    delete_staff_issuer, insert_staff_issuer, list_staff_issuers, CertificateAuthorityRecord,
    StaffIssuerRow,
};
use crate::postgres::signing::{
    get_signing_checks, lock_signing_event, upsert_signing_checks, SigningChecksRow,
};
use crate::services::certificate_authority::parse_certificate_pem;
use crate::services::signing::certificates::is_ca;
use crate::services::signing::log::{stage, Actor, LogScope, LogStep, SystemOutcome};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::SigningStatementKind;
use openssl::x509::X509;
use sequent_core::signing::SigningChecks;
use serde_json::{json, Value};
use tracing::instrument;
use uuid::Uuid;

/// The user id and username log entries name when no person made the
/// change (an election event import).
pub const SYSTEM_ACTOR: &str = "system";

fn event_scope(tenant_id: Uuid, election_event_id: Uuid) -> LogScope {
    LogScope {
        tenant_id,
        election_event_id,
        election_id: None,
        area_id: None,
    }
}

/// What an issuer import did.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IssuerImport {
    pub imported: Vec<StaffIssuerRow>,
    /// Subjects of issuers the event already trusts.
    pub skipped: Vec<String>,
    /// One line per certificate that was refused.
    pub errors: Vec<String>,
}

fn issuer_details(row: &StaffIssuerRow) -> Value {
    // The staff tables write hashes as lowercase hex; the authority's own
    // fingerprint is uppercase with colons.
    json!({
        "id": row.id,
        "subject": row.subject,
        "fingerprint": row.fingerprint_sha256.replace(':', "").to_lowercase(),
        "not_before": row.not_before,
        "not_after": row.not_after,
    })
}

fn names(rows: &[StaffIssuerRow]) -> String {
    rows.iter()
        .map(|row| row.common_name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Trusts `certificates` as staff issuers of the event. Each must be a
/// certificate authority that is not expired at `now`; one already trusted
/// is skipped.
#[instrument(skip(hasura_transaction, certificates), err)]
pub async fn import_staff_issuers(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    certificates: &[X509],
    actor: &Actor,
    now: DateTime<Utc>,
) -> Result<IssuerImport> {
    lock_signing_event(hasura_transaction, tenant_id, election_event_id).await?;
    let mut import = IssuerImport::default();
    for (index, certificate) in certificates.iter().enumerate() {
        let number = index + 1;
        let pem = String::from_utf8(certificate.to_pem()?)?;
        let parsed = match parse_certificate_pem(&pem) {
            Ok(parsed) => parsed,
            Err(err) => {
                import.errors.push(format!("Certificate {number}: {err}"));
                continue;
            }
        };
        if !is_ca(certificate)? {
            import.errors.push(format!(
                "Certificate {number}: {} is not a certificate authority",
                parsed.common_name
            ));
            continue;
        }
        if parsed.not_after < now {
            import.errors.push(format!(
                "Certificate {number}: expired on {}",
                parsed.not_after.format("%Y-%m-%d")
            ));
            continue;
        }
        let subject = parsed.subject.clone();
        let record = CertificateAuthorityRecord {
            id: Uuid::new_v4(),
            tenant_id,
            election_event_id,
            common_name: parsed.common_name,
            subject: parsed.subject,
            issuer_common_name: parsed.issuer_common_name,
            issuer: parsed.issuer,
            not_before: parsed.not_before,
            not_after: parsed.not_after,
            fingerprint_sha256: parsed.fingerprint_sha256,
            serial_number: parsed.serial_number,
            pem: parsed.pem,
        };
        match insert_staff_issuer(hasura_transaction, record).await? {
            Some(row) => import.imported.push(row),
            None => import.skipped.push(subject),
        }
    }
    if !import.imported.is_empty() {
        let description = match import.imported.as_slice() {
            [one] => format!("Imported trusted issuer {}", one.common_name),
            many => format!("Imported {} trusted issuers: {}", many.len(), names(many)),
        };
        stage(
            hasura_transaction,
            &LogStep {
                kind: SigningStatementKind::SigningIssuerChanged,
                user: actor.clone(),
                system: SystemOutcome::Info,
                scope: event_scope(tenant_id, election_event_id),
                description,
                details: json!({
                    "change": "imported",
                    "issuers": import.imported.iter().map(issuer_details).collect::<Vec<_>>(),
                }),
            },
        )
        .await?;
    }
    Ok(import)
}

/// Stops trusting a staff issuer; its revocation lists go with it. `None`
/// when the event has no staff issuer with that id.
#[instrument(skip(hasura_transaction), err)]
pub async fn remove_staff_issuer(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    id: Uuid,
    actor: &Actor,
) -> Result<Option<StaffIssuerRow>> {
    lock_signing_event(hasura_transaction, tenant_id, election_event_id).await?;
    let Some(row) =
        delete_staff_issuer(hasura_transaction, tenant_id, election_event_id, id).await?
    else {
        return Ok(None);
    };
    stage(
        hasura_transaction,
        &LogStep {
            kind: SigningStatementKind::SigningIssuerChanged,
            user: actor.clone(),
            system: SystemOutcome::Info,
            scope: event_scope(tenant_id, election_event_id),
            description: format!("Removed trusted issuer {}", row.common_name),
            details: json!({
                "change": "removed",
                "issuers": [issuer_details(&row)],
            }),
        },
    )
    .await?;
    Ok(Some(row))
}

/// The event's staff issuers as one PEM bundle, for an election event
/// export; `None` without any. The ids are any UUIDs.
pub async fn staff_issuers_pem_bundle(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Option<String>> {
    let tenant_id = Uuid::parse_str(tenant_id).context("Invalid tenant id")?;
    let election_event_id =
        Uuid::parse_str(election_event_id).context("Invalid election event id")?;
    let issuers = list_staff_issuers(hasura_transaction, tenant_id, election_event_id).await?;
    Ok((!issuers.is_empty()).then(|| {
        issuers
            .iter()
            .map(|issuer| issuer.pem.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }))
}

/// The changed settings, as "setting: before → after".
fn check_changes(before: &SigningChecks, after: &SigningChecks) -> Vec<String> {
    let mut changes = vec![];
    let mut compare = |name: &str, before: String, after: String| {
        if before != after {
            changes.push(format!("{name}: {before} → {after}"));
        }
    };
    compare(
        "revocation check",
        before.revocation_check.to_string(),
        after.revocation_check.to_string(),
    );
    compare(
        "when a list can't be downloaded",
        before.crl_unavailable.to_string(),
        after.crl_unavailable.to_string(),
    );
    compare(
        "registration",
        before.registration.to_string(),
        after.registration.to_string(),
    );
    compare(
        "post binding",
        before.post_binding.to_string(),
        after.post_binding.to_string(),
    );
    changes
}

/// Saves the event's checks if they are still at `expected_revision` (0
/// before the first save). `None` when someone saved them first.
#[instrument(skip(hasura_transaction), err)]
pub async fn update_signing_checks(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    checks: &SigningChecks,
    expected_revision: i64,
    actor: &Actor,
    actor_name: Option<&str>,
) -> Result<Option<SigningChecksRow>> {
    lock_signing_event(hasura_transaction, tenant_id, election_event_id).await?;
    let before = get_signing_checks(hasura_transaction, tenant_id, election_event_id)
        .await?
        .map(|row| row.checks)
        .unwrap_or_default();
    let Some(row) = upsert_signing_checks(
        hasura_transaction,
        tenant_id,
        election_event_id,
        checks,
        expected_revision,
        &actor.user_id,
        actor_name,
    )
    .await?
    else {
        return Ok(None);
    };
    let changes = check_changes(&before, &row.checks);
    let description = if changes.is_empty() {
        format!(
            "Saved the certificate checks unchanged (revision {})",
            row.checks.revision
        )
    } else {
        format!("Changed the certificate checks: {}", changes.join("; "))
    };
    stage(
        hasura_transaction,
        &LogStep {
            kind: SigningStatementKind::SigningChecksChanged,
            user: actor.clone(),
            system: SystemOutcome::Info,
            scope: event_scope(tenant_id, election_event_id),
            description,
            details: json!({
                "before": before,
                "after": row.checks,
                "revision": row.checks.revision,
            }),
        },
    )
    .await?;
    Ok(Some(row))
}
