// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::delete_election_event::delete_keycloak_realm;
use anyhow::{anyhow, Result};
use futures::try_join;
use sequent_core::services::s3;
use tracing::instrument;

#[instrument(err)]
pub async fn delete_tenant_related_documents(tenant_id: &str) -> Result<()> {
    let documents_prefix = format!("tenant-{}/", tenant_id);

    let bucket = s3::get_private_bucket()?;
    s3::delete_files_from_s3(bucket, documents_prefix.clone(), s3::S3Endpoint::Server)
        .await
        .map_err(|err| anyhow!("Error delete private files from s3: {err:?}"))?;

    let public_bucket = s3::get_public_bucket()?;
    s3::delete_files_from_s3(public_bucket, documents_prefix, s3::S3Endpoint::Server)
        .await
        .map_err(|err| anyhow!("Error delete public files from s3: {err:?}"))?;

    Ok(())
}

#[instrument(err)]
pub async fn delete_tenant_related_data(tenant_id: &str, realm: &str) -> Result<()> {
    let documents_future = delete_tenant_related_documents(tenant_id);
    let keycloak_future = delete_keycloak_realm(realm);
    try_join!(documents_future, keycloak_future)?;

    Ok(())
}
