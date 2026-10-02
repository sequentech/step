// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Queries of staff certificates, their revocation lists and the requests
//! they sign that the certificate services need besides
//! [`crate::postgres::signing`]. Every query is scoped to its tenant.

use crate::postgres::signing::{SigningRequestRow, StaffCertificateRow};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use sequent_core::signing::{CrlStatus, SigningRequestStatus, StaffCertificateStatus};
use std::str::FromStr;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

/// A request by id within the tenant: the signing routes name a request
/// without its event.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_signing_request_in_tenant(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<SigningRequestRow>> {
    hasura_transaction
        .query_opt(
            "SELECT * FROM sequent_backend.signing_request WHERE tenant_id = $1 AND id = $2",
            &[&tenant_id, &id],
        )
        .await
        .context("Error reading the signing request")?
        .map(SigningRequestRow::try_from)
        .transpose()
}

/// Whether the election (Post) belongs to the event.
#[instrument(skip(hasura_transaction), err)]
pub async fn election_in_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    election_id: Uuid,
) -> Result<bool> {
    Ok(hasura_transaction
        .query_opt(
            "SELECT 1 FROM sequent_backend.election
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
            &[&tenant_id, &election_event_id, &election_id],
        )
        .await
        .context("Error reading the election")?
        .is_some())
}

/// The active registrations of the event, for the revocation lists of
/// their issuers.
#[instrument(skip(hasura_transaction), err)]
pub async fn list_active_staff_certificates(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<Vec<StaffCertificateRow>> {
    hasura_transaction
        .query(
            "SELECT * FROM sequent_backend.staff_certificate
             WHERE tenant_id = $1 AND election_event_id = $2 AND status = $3
             ORDER BY registered_at, id",
            &[
                &tenant_id,
                &election_event_id,
                &StaffCertificateStatus::Active.to_string(),
            ],
        )
        .await
        .context("Error listing the active staff certificates")?
        .into_iter()
        .map(StaffCertificateRow::try_from)
        .collect()
}

/// A registration by id in the event, without locking it.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_staff_certificate(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    id: Uuid,
) -> Result<Option<StaffCertificateRow>> {
    hasura_transaction
        .query_opt(
            "SELECT * FROM sequent_backend.staff_certificate
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
            &[&tenant_id, &election_event_id, &id],
        )
        .await
        .context("Error reading the staff certificate")?
        .map(StaffCertificateRow::try_from)
        .transpose()
}

/// Revokes every active registration of the key (`spki_sha256`) in the
/// tenant: a revocation is of the key, in every event and account.
#[instrument(skip(hasura_transaction, reason), err)]
pub async fn revoke_staff_certificates_of_key(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    spki_sha256: &str,
    revoked_by: &str,
    revoked_by_name: Option<&str>,
    reason: &str,
) -> Result<Vec<StaffCertificateRow>> {
    hasura_transaction
        .query(
            "UPDATE sequent_backend.staff_certificate SET
                 status = $3, revoked_by = $4, revoked_at = clock_timestamp(),
                 revoke_reason = $5, revoked_by_name = $6
             WHERE tenant_id = $1 AND spki_sha256 = $2 AND status = $7
             RETURNING *",
            &[
                &tenant_id,
                &spki_sha256,
                &StaffCertificateStatus::Revoked.to_string(),
                &revoked_by,
                &reason,
                &revoked_by_name,
                &StaffCertificateStatus::Active.to_string(),
            ],
        )
        .await
        .context("Error revoking the staff certificates of the key")?
        .into_iter()
        .map(StaffCertificateRow::try_from)
        .collect()
}

/// The Posts (elections) a key signed requests for in the event, through
/// any account, counting only `post_actions` (the Post-scoped actions).
#[instrument(skip(hasura_transaction), err)]
pub async fn posts_signed_by_key(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    spki_sha256: &str,
    post_actions: &[String],
) -> Result<Vec<Uuid>> {
    Ok(hasura_transaction
        .query(
            "SELECT DISTINCT r.election_id FROM sequent_backend.signing_approval a
             JOIN sequent_backend.signing_request r
               ON r.tenant_id = a.tenant_id AND r.election_event_id = a.election_event_id
              AND r.id = a.request_id
             WHERE a.tenant_id = $1 AND a.election_event_id = $2 AND a.spki_sha256 = $3
               AND r.election_id IS NOT NULL AND r.action = ANY($4)
             ORDER BY r.election_id",
            &[&tenant_id, &election_event_id, &spki_sha256, &post_actions],
        )
        .await
        .context("Error listing the Posts the key signed for")?
        .iter()
        .map(|row| row.get(0))
        .collect())
}

/// The waiting requests of the event that a key already signed, in id
/// order.
#[instrument(skip(hasura_transaction), err)]
pub async fn waiting_requests_signed_by_key(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    spki_sha256: &str,
) -> Result<Vec<Uuid>> {
    Ok(hasura_transaction
        .query(
            "SELECT DISTINCT r.id FROM sequent_backend.signing_approval a
             JOIN sequent_backend.signing_request r
               ON r.tenant_id = a.tenant_id AND r.election_event_id = a.election_event_id
              AND r.id = a.request_id
             WHERE a.tenant_id = $1 AND a.election_event_id = $2 AND a.spki_sha256 = $3
               AND r.status = $4
             ORDER BY r.id",
            &[
                &tenant_id,
                &election_event_id,
                &spki_sha256,
                &SigningRequestStatus::Waiting.to_string(),
            ],
        )
        .await
        .context("Error listing the waiting requests the key signed")?
        .iter()
        .map(|row| row.get(0))
        .collect())
}

// Revocation lists

#[derive(Debug, Clone, PartialEq)]
pub struct StaffCrlRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub issuer_id: Uuid,
    pub issuer_fingerprint: String,
    pub url: String,
    pub der: Option<Vec<u8>>,
    pub this_update: Option<DateTime<Utc>>,
    pub next_update: Option<DateTime<Utc>>,
    /// The last download attempt.
    pub fetched_at: DateTime<Utc>,
    /// The last download that was accepted (`der` is from it).
    pub last_ok_at: Option<DateTime<Utc>>,
    pub status: CrlStatus,
    pub last_error: Option<String>,
}

impl TryFrom<Row> for StaffCrlRow {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self> {
        let status: String = row.try_get("status")?;
        Ok(StaffCrlRow {
            id: row.try_get("id")?,
            tenant_id: row.try_get("tenant_id")?,
            election_event_id: row.try_get("election_event_id")?,
            issuer_id: row.try_get("issuer_id")?,
            issuer_fingerprint: row.try_get("issuer_fingerprint")?,
            url: row.try_get("url")?,
            der: row.try_get("der")?,
            this_update: row.try_get("this_update")?,
            next_update: row.try_get("next_update")?,
            fetched_at: row.try_get("fetched_at")?,
            last_ok_at: row.try_get("last_ok_at")?,
            status: CrlStatus::from_str(&status)
                .map_err(|err| anyhow!("status: unknown value {status:?}: {err}"))?,
            last_error: row.try_get("last_error")?,
        })
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn list_staff_crls(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<Vec<StaffCrlRow>> {
    hasura_transaction
        .query(
            "SELECT * FROM sequent_backend.staff_crl
             WHERE tenant_id = $1 AND election_event_id = $2
             ORDER BY url",
            &[&tenant_id, &election_event_id],
        )
        .await
        .context("Error listing the staff revocation lists")?
        .into_iter()
        .map(StaffCrlRow::try_from)
        .collect()
}

/// One download of a revocation list.
#[derive(Debug, Clone)]
pub enum CrlDownload {
    /// A list signed by the issuer.
    Ok {
        der: Vec<u8>,
        this_update: Option<DateTime<Utc>>,
        next_update: Option<DateTime<Utc>>,
    },
    /// The download or the list failed; the last good list is kept.
    Unavailable { error: String },
}

/// Records a download of `url`: a good list replaces the stored one; a
/// failure keeps it and records the error.
#[instrument(skip(hasura_transaction, download), err)]
pub async fn upsert_staff_crl(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    issuer_id: Uuid,
    issuer_fingerprint: &str,
    url: &str,
    download: &CrlDownload,
) -> Result<StaffCrlRow> {
    let (status, der, this_update, next_update, error) = match download {
        CrlDownload::Ok {
            der,
            this_update,
            next_update,
        } => (
            CrlStatus::Ok,
            Some(der.clone()),
            *this_update,
            *next_update,
            None,
        ),
        CrlDownload::Unavailable { error } => (
            CrlStatus::Unavailable,
            None,
            None,
            None,
            Some(error.clone()),
        ),
    };
    // One reading of the clock: a good list's fetch is its last good time.
    let row = hasura_transaction
        .query_one(
            "WITH clock AS (SELECT clock_timestamp() AS now)
             INSERT INTO sequent_backend.staff_crl AS crl
                 (tenant_id, election_event_id, issuer_id, issuer_fingerprint, url, der,
                  this_update, next_update, status, last_error, fetched_at, last_ok_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, (SELECT now FROM clock),
                     CASE WHEN $6::bytea IS NULL THEN NULL ELSE (SELECT now FROM clock) END)
             ON CONFLICT (tenant_id, election_event_id, url) DO UPDATE SET
                 issuer_id = EXCLUDED.issuer_id,
                 issuer_fingerprint = EXCLUDED.issuer_fingerprint,
                 der = COALESCE(EXCLUDED.der, crl.der),
                 this_update = CASE WHEN EXCLUDED.der IS NULL
                     THEN crl.this_update ELSE EXCLUDED.this_update END,
                 next_update = CASE WHEN EXCLUDED.der IS NULL
                     THEN crl.next_update ELSE EXCLUDED.next_update END,
                 status = EXCLUDED.status,
                 last_error = EXCLUDED.last_error,
                 fetched_at = EXCLUDED.fetched_at,
                 last_ok_at = COALESCE(EXCLUDED.last_ok_at, crl.last_ok_at)
             RETURNING *",
            &[
                &tenant_id,
                &election_event_id,
                &issuer_id,
                &issuer_fingerprint,
                &url,
                &der,
                &this_update,
                &next_update,
                &status.to_string(),
                &error,
            ],
        )
        .await
        .context("Error storing the staff revocation list")?;
    StaffCrlRow::try_from(row)
}

/// The events with staff issuers: the ones whose lists the job refreshes.
#[instrument(skip(hasura_transaction), err)]
pub async fn list_events_with_staff_issuers(
    hasura_transaction: &Transaction<'_>,
) -> Result<Vec<(Uuid, Uuid)>> {
    Ok(hasura_transaction
        .query(
            "SELECT DISTINCT tenant_id, election_event_id
             FROM sequent_backend.certificate_authority
             WHERE purpose = 'staff-signatures'
             ORDER BY tenant_id, election_event_id",
            &[],
        )
        .await
        .context("Error listing the events with staff issuers")?
        .iter()
        .map(|row| (row.get(0), row.get(1)))
        .collect())
}
