// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::audit_log::{diff_values, stage, AdminAction, AuditActor, AuditObject};
use super::delete_election_event::delete_keycloak_realm;
use super::task_audit;
use crate::postgres::tenant::get_tenant_by_id_if_exist;
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use futures::try_join;
use sequent_core::services::s3;
use serde_json::json;
use tracing::instrument;

/// Queues the entry of a tenant's deletion in the transaction that deletes
/// it, with what the tenant was. It goes to the log of `log_tenant_id`, the
/// tenant of the admin who deletes it: the deleted tenant's own log goes
/// with it. A tenant already gone (a retried deletion) adds no entry.
#[instrument(skip(hasura_transaction), err)]
pub async fn stage_tenant_deletion(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    log_tenant_id: &str,
    deleted_by: Option<AuditActor>,
    task_id: Option<&str>,
) -> Result<()> {
    let target = task_audit::target(log_tenant_id, None)?;
    let Some(tenant) = get_tenant_by_id_if_exist(hasura_transaction, tenant_id).await? else {
        return Ok(());
    };
    let fields = json!({
        "slug": tenant.slug,
        "is_active": tenant.is_active,
        "voting_channels": tenant.voting_channels,
        "settings": tenant.settings,
    });
    let change = task_audit::change(
        deleted_by,
        target,
        AuditObject::new(task_audit::TENANT, tenant_id, Some(&tenant.slug)),
        AdminAction::Delete,
    )
    .with_changes(diff_values("", Some(&fields), None))
    .with_extra(json!({ "task_id": task_id }));
    stage(hasura_transaction, &change).await?;
    Ok(())
}

#[instrument(err)]
pub async fn delete_tenant_related_documents(tenant_id: &str) -> Result<()> {
    let documents_prefix = format!("tenant-{}/", tenant_id);

    let bucket = s3::get_private_bucket()?;
    s3::delete_files_from_s3(bucket, documents_prefix.clone(), s3::S3Endpoint::Server)
        .await
        .map_err(|err| anyhow!("Error deleting private files from S3: {err:?}"))?;

    let public_bucket = s3::get_public_bucket()?;
    s3::delete_files_from_s3(public_bucket, documents_prefix, s3::S3Endpoint::Server)
        .await
        .map_err(|err| anyhow!("Error deleting public files from S3: {err:?}"))?;

    Ok(())
}

#[instrument(err)]
pub async fn delete_tenant_related_data(tenant_id: &str, realm: &str) -> Result<()> {
    let documents_future = delete_tenant_related_documents(tenant_id);
    let keycloak_future = delete_keycloak_realm(realm);
    try_join!(documents_future, keycloak_future)?;

    Ok(())
}
