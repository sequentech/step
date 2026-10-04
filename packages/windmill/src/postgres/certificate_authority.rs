// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::{Client, Transaction};
use tracing::instrument;
use uuid::Uuid;

pub struct CertificateAuthorityRecord {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub common_name: String,
    pub subject: String,
    pub issuer_common_name: String,
    pub issuer: String,
    pub not_before: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
    pub fingerprint_sha256: String,
    pub serial_number: String,
    pub pem: String,
}

/// Inserts a certificate authority record into the database.
/// Returns `true` if the record was inserted, `false` if it was skipped
/// due to a duplicate fingerprint for the same election event and purpose
/// (the record's purpose is the column default, voter sign-in).
#[instrument(skip(hasura_transaction, record), err)]
pub async fn insert_certificate_authority(
    hasura_transaction: &Transaction<'_>,
    record: CertificateAuthorityRecord,
) -> Result<bool> {
    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO sequent_backend.certificate_authority
                    (id, tenant_id, election_event_id, common_name, subject,
                     issuer_common_name, issuer, not_before, not_after,
                     fingerprint_sha256, serial_number, pem)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                ON CONFLICT (tenant_id, election_event_id, purpose, fingerprint_sha256) DO NOTHING
            "#,
        )
        .await?;

    let rows_affected = hasura_transaction
        .execute(
            &statement,
            &[
                &record.id,
                &record.tenant_id,
                &record.election_event_id,
                &record.common_name,
                &record.subject,
                &record.issuer_common_name,
                &record.issuer,
                &record.not_before,
                &record.not_after,
                &record.fingerprint_sha256,
                &record.serial_number,
                &record.pem,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error inserting certificate authority: {err}"))?;

    Ok(rows_affected > 0)
}

/// Deletes the voter sign-in certificate authorities matching the given ids,
/// scoped to the given tenant and election event; staff issuers are removed
/// with [`delete_staff_issuer`].
/// Returns the subjects of all deleted rows.
#[instrument(skip(hasura_transaction), err)]
pub async fn delete_certificate_authorities(
    hasura_transaction: &Transaction<'_>,
    ids: &[Uuid],
    election_event_id: Uuid,
    tenant_id: Uuid,
) -> Result<Vec<String>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                DELETE FROM sequent_backend.certificate_authority
                WHERE id = ANY($1) AND tenant_id = $2 AND election_event_id = $3
                  AND purpose = 'voter-sign-in'
                RETURNING subject
            "#,
        )
        .await?;

    let rows = hasura_transaction
        .query(&statement, &[&ids, &tenant_id, &election_event_id])
        .await
        .map_err(|err| anyhow!("Error deleting certificate authorities: {err}"))?;

    Ok(rows.into_iter().map(|row| row.get(0)).collect())
}

/// Returns the PEM strings for all voter sign-in certificate authorities
/// belonging to the given election event, ordered by creation time. Staff
/// issuers never appear: this feeds the voters' sign-in truststore.
#[instrument(skip(transaction), err)]
pub async fn get_certificate_authorities_pem(
    transaction: &Transaction<'_>,
    election_event_id: Uuid,
) -> Result<Vec<String>> {
    let statement = transaction
        .prepare(
            r#"
                SELECT pem
                FROM sequent_backend.certificate_authority
                WHERE election_event_id = $1 AND purpose = 'voter-sign-in'
                ORDER BY created_at ASC
            "#,
        )
        .await?;

    let rows = transaction
        .query(&statement, &[&election_event_id])
        .await
        .map_err(|err| anyhow!("Error fetching certificate authority PEMs: {err}"))?;

    Ok(rows
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect())
}

/// Returns the PEM strings for the specified voter sign-in certificate
/// authorities (by id),
/// scoped to the given election event. If `ids` is empty, returns PEMs for all
/// CAs in the election event (same behaviour as `get_certificate_authorities_pem`).
#[instrument(skip(transaction), err)]
pub async fn get_certificate_authorities_pem_by_ids(
    transaction: &Transaction<'_>,
    election_event_id: Uuid,
    ids: &[Uuid],
) -> Result<Vec<String>> {
    if ids.is_empty() {
        return get_certificate_authorities_pem(transaction, election_event_id).await;
    }

    let statement = transaction
        .prepare(
            r#"
                SELECT pem
                FROM sequent_backend.certificate_authority
                WHERE election_event_id = $1
                  AND purpose = 'voter-sign-in'
                  AND id = ANY($2)
                ORDER BY created_at ASC
            "#,
        )
        .await?;

    let rows = transaction
        .query(&statement, &[&election_event_id, &ids])
        .await
        .map_err(|err| anyhow!("Error fetching certificate authority PEMs by ids: {err}"))?;

    Ok(rows
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect())
}

/// A staff issuer: a certificate authority of purpose staff signatures.
#[derive(Debug, Clone, PartialEq)]
pub struct StaffIssuerRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub common_name: String,
    pub subject: String,
    pub issuer_common_name: String,
    pub issuer: String,
    pub not_before: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
    /// Uppercase hex with colons, like every certificate authority's.
    pub fingerprint_sha256: String,
    pub serial_number: String,
    pub pem: String,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<tokio_postgres::Row> for StaffIssuerRow {
    type Error = anyhow::Error;

    fn try_from(row: tokio_postgres::Row) -> Result<Self> {
        Ok(StaffIssuerRow {
            id: row.try_get("id")?,
            tenant_id: row.try_get("tenant_id")?,
            election_event_id: row.try_get("election_event_id")?,
            common_name: row.try_get("common_name")?,
            subject: row.try_get("subject")?,
            issuer_common_name: row.try_get("issuer_common_name")?,
            issuer: row.try_get("issuer")?,
            not_before: row.try_get("not_before")?,
            not_after: row.try_get("not_after")?,
            fingerprint_sha256: row.try_get("fingerprint_sha256")?,
            serial_number: row.try_get("serial_number")?,
            pem: row.try_get("pem")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

const STAFF_ISSUER_COLUMNS: &str = "id, tenant_id, election_event_id, common_name, subject, \
     issuer_common_name, issuer, not_before, not_after, fingerprint_sha256, serial_number, pem, \
     created_at";

/// Inserts a staff issuer. `None` when the event already trusts the same
/// certificate as a staff issuer.
#[instrument(skip(hasura_transaction, record), err)]
pub async fn insert_staff_issuer(
    hasura_transaction: &Transaction<'_>,
    record: CertificateAuthorityRecord,
) -> Result<Option<StaffIssuerRow>> {
    hasura_transaction
        .query_opt(
            &format!(
                "INSERT INTO sequent_backend.certificate_authority
                     (id, tenant_id, election_event_id, common_name, subject,
                      issuer_common_name, issuer, not_before, not_after,
                      fingerprint_sha256, serial_number, pem, purpose)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, 'staff-signatures')
                 ON CONFLICT (tenant_id, election_event_id, purpose, fingerprint_sha256) DO NOTHING
                 RETURNING {STAFF_ISSUER_COLUMNS}"
            ),
            &[
                &record.id,
                &record.tenant_id,
                &record.election_event_id,
                &record.common_name,
                &record.subject,
                &record.issuer_common_name,
                &record.issuer,
                &record.not_before,
                &record.not_after,
                &record.fingerprint_sha256,
                &record.serial_number,
                &record.pem,
            ],
        )
        .await
        .context("Error inserting the staff issuer")?
        .map(StaffIssuerRow::try_from)
        .transpose()
}

/// The event's staff issuers, oldest first.
#[instrument(skip(hasura_transaction), err)]
pub async fn list_staff_issuers(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<Vec<StaffIssuerRow>> {
    hasura_transaction
        .query(
            &format!(
                "SELECT {STAFF_ISSUER_COLUMNS} FROM sequent_backend.certificate_authority
                 WHERE tenant_id = $1 AND election_event_id = $2 AND purpose = 'staff-signatures'
                 ORDER BY created_at, id"
            ),
            &[&tenant_id, &election_event_id],
        )
        .await
        .context("Error listing the staff issuers")?
        .into_iter()
        .map(StaffIssuerRow::try_from)
        .collect()
}

/// Removes a staff issuer (and, by cascade, its revocation lists). `None`
/// when the event has no staff issuer with that id; a voter sign-in
/// authority is never removed here.
#[instrument(skip(hasura_transaction), err)]
pub async fn delete_staff_issuer(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    id: Uuid,
) -> Result<Option<StaffIssuerRow>> {
    hasura_transaction
        .query_opt(
            &format!(
                "DELETE FROM sequent_backend.certificate_authority
                 WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3
                   AND purpose = 'staff-signatures'
                 RETURNING {STAFF_ISSUER_COLUMNS}"
            ),
            &[&tenant_id, &election_event_id, &id],
        )
        .await
        .context("Error removing the staff issuer")?
        .map(StaffIssuerRow::try_from)
        .transpose()
}
