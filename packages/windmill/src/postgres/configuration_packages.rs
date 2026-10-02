// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Imported configuration packages and the revocation lists a tenant has
//! seen.

use anyhow::{Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::election_config::manifest::Manifest;
use sequent_core::election_config::package_verify::{LastImport, VerifiedPackage};
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sha2::{Digest, Sha256};
use tracing::instrument;

/// The newest package imported for a configuration.
#[instrument(skip(hasura_transaction), err)]
pub async fn last_import(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    external_id: &str,
) -> Result<Option<LastImport>> {
    let row = hasura_transaction
        .query_opt(
            "SELECT revision, manifest_sha256 FROM sequent_backend.configuration_package
             WHERE tenant_id = $1 AND external_id = $2
             ORDER BY revision DESC LIMIT 1",
            &[&parse_uuid_v4(tenant_id)?, &external_id],
        )
        .await?;
    Ok(row.map(|row| LastImport {
        revision: row.get::<_, i64>(0) as u64,
        manifest_sha256: row.get(1),
    }))
}

/// Records a verified package as imported into `election_event_id`.
#[instrument(skip(hasura_transaction, package), err)]
pub async fn record_import(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    package: &VerifiedPackage,
) -> Result<()> {
    let revision = i64::try_from(package.manifest.configuration.revision)
        .context("the revision is out of range")?;
    hasura_transaction
        .execute(
            "INSERT INTO sequent_backend.configuration_package
                (tenant_id, election_event_id, external_id, revision, manifest_sha256, manifest,
                 signer_subject, signer_serial, signer_fingerprint, approvers)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &package.manifest.configuration.external_id,
                &revision,
                &package.manifest_sha256,
                &serde_json::to_value(&package.manifest)?,
                &package.signer.subject,
                &package.signer.serial,
                &package.signer.fingerprint_sha256,
                &serde_json::to_value(&package.approvers)?,
            ],
        )
        .await
        .context("could not record the imported configuration package")?;
    Ok(())
}

/// The manifest of the package an election event was imported from, and
/// its digest, if it was imported from one.
#[instrument(skip(hasura_transaction), err)]
pub async fn manifest_of_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Option<(Manifest, String)>> {
    let row = hasura_transaction
        .query_opt(
            "SELECT manifest, manifest_sha256 FROM sequent_backend.configuration_package
             WHERE tenant_id = $1 AND election_event_id = $2
             ORDER BY revision DESC LIMIT 1",
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
            ],
        )
        .await?;
    row.map(|row| -> Result<(Manifest, String)> {
        Ok((serde_json::from_value(row.get(0))?, row.get(1)))
    })
    .transpose()
}

/// Every revocation list the tenant has seen, DER.
#[instrument(skip(hasura_transaction), err)]
pub async fn revocation_lists(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
) -> Result<Vec<Vec<u8>>> {
    Ok(hasura_transaction
        .query(
            "SELECT der FROM sequent_backend.configuration_revocation_list WHERE tenant_id = $1",
            &[&parse_uuid_v4(tenant_id)?],
        )
        .await?
        .into_iter()
        .map(|row| row.get(0))
        .collect())
}

/// Keeps revocation lists (DER) so they apply to every later import.
#[instrument(skip(hasura_transaction, lists), err)]
pub async fn remember_revocation_lists(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    lists: &[Vec<u8>],
) -> Result<()> {
    let tenant = parse_uuid_v4(tenant_id)?;
    for der in lists {
        let sha256 = hex::encode(Sha256::digest(der));
        hasura_transaction
            .execute(
                "INSERT INTO sequent_backend.configuration_revocation_list (tenant_id, sha256, der)
                 VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
                &[&tenant, &sha256, der],
            )
            .await?;
    }
    Ok(())
}
