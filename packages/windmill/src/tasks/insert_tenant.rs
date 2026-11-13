// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::tenant_bootstrap_admin::TenantBootstrapAdminPolicy;
use crate::postgres::tenant::{
    get_tenant_by_id_if_exist, get_tenant_by_slug_if_exist, insert_tenant,
};
use crate::services::database::get_hasura_pool;
use crate::services::delete_election_event::delete_keycloak_realm;
use crate::services::import::import_election_event::remove_keycloak_realm_secrets;
use crate::services::jwks::upsert_realm_jwks;
use crate::services::keycloak::read_realm_config_from_s3;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::services::tenant_credentials::{
    provision_tenant_credentials, select_source_trustees, TenantCredentialsRequest,
};
use crate::types::error::Result;
use ::keycloak::types::RealmRepresentation;
use anyhow::{Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use sequent_core::services::keycloak::get_tenant_realm;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::types::hasura::core::TasksExecution;
use tracing::{event, instrument, Level};

const KEYCLOAK_TENANT_REALM_CONFIG_S3_KEY: &str = "KEYCLOAK_TENANT_REALM_CONFIG_S3_KEY";

#[instrument(err)]
pub async fn read_default_tenant_realm() -> AnyhowResult<RealmRepresentation> {
    read_realm_config_from_s3(KEYCLOAK_TENANT_REALM_CONFIG_S3_KEY).await
}

#[instrument(err)]
pub async fn upsert_keycloak_realm(tenant_id: &str, slug: &str) -> Result<()> {
    let mut default_tenant = read_default_tenant_realm().await?;
    default_tenant = remove_keycloak_realm_secrets(&default_tenant)?;
    TenantBootstrapAdminPolicy::apply_to_realm(&mut default_tenant);
    let realm_config = serde_json::to_string(&default_tenant)?;
    let client = KeycloakAdminClient::new().await?;
    let realm = get_tenant_realm(tenant_id);
    client
        .upsert_realm(
            realm.as_str(),
            &realm_config,
            tenant_id,
            true,
            Some(slug.to_string()),
            None,
        )
        .await?;
    upsert_realm_jwks(realm.as_str()).await?;
    Ok(())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn insert_tenant_db(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    slug: &str,
) -> Result<()> {
    // fetch tenant
    let found_tenant = get_tenant_by_id_if_exist(hasura_transaction, tenant_id).await?;

    if found_tenant.is_some() {
        event!(Level::INFO, "Tenant with id {} already exists", tenant_id);
        return Ok(());
    }

    let _ = insert_tenant(hasura_transaction, tenant_id, slug).await?;

    Ok(())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn check_tenant_exists(hasura_transaction: &Transaction<'_>, slug: &str) -> Result<bool> {
    // fetch tenant
    let found_tenant = get_tenant_by_slug_if_exist(hasura_transaction, slug).await?;

    Ok(found_tenant.is_some())
}

/// Creates the tenant. With `credentials` it also creates its trustees and logins,
/// and returns the id of the encrypted document that holds them.
#[instrument(err, skip(credentials))]
pub async fn process_insert_tenant(
    tenant_id: String,
    slug: String,
    credentials: Option<TenantCredentialsRequest>,
) -> Result<Option<String>> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| format!("Error getting hasura db pool: {err}"))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|err| format!("Error starting hasura transaction: {err}"))?;

    let tenant_exists = check_tenant_exists(&hasura_transaction, &slug).await?;
    if tenant_exists {
        // A request for credentials must not report success with no file to download
        if credentials.is_some() {
            return Err(format!("Tenant with slug {slug} already exists").into());
        }
        event!(Level::INFO, "Tenant with slug {} already exists", slug);
        return Ok(None);
    }

    // Check the request before anything is created
    let trustees = match credentials.as_ref() {
        Some(request) => select_source_trustees(&hasura_transaction, request.num_trustees).await?,
        None => vec![],
    };

    upsert_keycloak_realm(tenant_id.as_str(), slug.as_str()).await?;
    insert_tenant_db(&hasura_transaction, &tenant_id, &slug).await?;

    let document_id = match credentials.as_ref() {
        Some(request) => {
            match provision_tenant_credentials(
                &hasura_transaction,
                &tenant_id,
                &slug,
                &trustees,
                request,
            )
            .await
            {
                Ok(document_id) => Some(document_id),
                Err(err) => {
                    // The passwords are lost with the transaction: leave no realm behind
                    let realm = get_tenant_realm(&tenant_id);
                    if let Err(delete_err) = delete_keycloak_realm(&realm).await {
                        event!(
                            Level::ERROR,
                            "Failed to delete realm {realm}: {delete_err:?}"
                        );
                    }
                    return Err(err.into());
                }
            }
        }
        None => None,
    };

    hasura_transaction
        .commit()
        .await
        .map_err(|err| format!("Error committing hasura transaction: {err}"))?;

    Ok(document_id)
}

#[instrument(err, skip(credentials))]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn insert_tenant(
    tenant_id: String,
    slug: String,
    task_execution: Option<TasksExecution>,
    credentials: Option<TenantCredentialsRequest>,
) -> Result<()> {
    let res = process_insert_tenant(tenant_id.clone(), slug.clone(), credentials).await;
    if let Some(task_execution) = task_execution {
        let document_id = match res {
            Ok(document_id) => document_id,
            Err(err) => {
                let err_str = format!("Error inserting tenant: {}", err);
                event!(Level::ERROR, err_str);
                update_fail(&task_execution, &err_str)
                    .await
                    .context("Failed to update task insert tenant to FAILED")?;
                return Err(err);
            }
        };
        update_complete(&task_execution, document_id)
            .await
            .context("Failed to update task execution status to COMPLETED")?;
    }

    Ok(())
}
