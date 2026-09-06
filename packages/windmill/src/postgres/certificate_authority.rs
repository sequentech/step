// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
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
/// due to a duplicate fingerprint for the same election event.
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
                ON CONFLICT (tenant_id, election_event_id, fingerprint_sha256) DO NOTHING
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

/// Deletes the certificate authorities matching the given ids, scoped to the
/// given tenant and election event.
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

/// Returns the PEM strings for all certificate authorities belonging to the
/// given election event, ordered by creation time. Uses a transaction.
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
                WHERE election_event_id = $1
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

/// Returns the PEM strings for the specified certificate authorities (by id),
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
