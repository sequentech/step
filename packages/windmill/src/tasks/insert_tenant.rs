// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::tenant::{
    get_tenant_by_id_if_exist, get_tenant_by_slug_if_exist, insert_tenant,
};
use crate::services::database::get_hasura_pool;
use crate::services::import::import_election_event::remove_keycloak_realm_secrets;
use crate::services::jwks::upsert_realm_jwks;
use crate::services::keycloak::read_realm_config_from_s3;
use crate::services::tasks_execution::{update_complete, update_fail};
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

    insert_tenant(hasura_transaction, tenant_id, slug).await?;

    Ok(())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn check_tenant_exists(hasura_transaction: &Transaction<'_>, slug: &str) -> Result<bool> {
    // fetch tenant
    let found_tenant = get_tenant_by_slug_if_exist(hasura_transaction, slug).await?;

    Ok(found_tenant.is_some())
}

#[instrument(err)]
pub async fn process_insert_tenant(tenant_id: String, slug: String) -> Result<()> {
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
        event!(Level::INFO, "Tenant with slug {} already exists", slug);
        return Ok(());
    }

    upsert_keycloak_realm(tenant_id.as_str(), slug.as_str()).await?;
    insert_tenant_db(&hasura_transaction, &tenant_id, &slug).await?;

    hasura_transaction
        .commit()
        .await
        .map_err(|err| format!("Error committing hasura transaction: {err}"))?;

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn insert_tenant(
    tenant_id: String,
    slug: String,
    task_execution: Option<TasksExecution>,
) -> Result<()> {
    let res = process_insert_tenant(tenant_id.clone(), slug.clone()).await;
    if let Some(task_execution) = task_execution {
        if let Err(err) = res {
            let err_str = format!("Error inserting tenant: {}", err);
            event!(Level::ERROR, err_str);
            update_fail(&task_execution, &err_str)
                .await
                .context("Failed to update task insert tenant to FAILED")?;
            return Err(err);
        }
        update_complete(&task_execution, None)
            .await
            .context("Failed to update task execution status to COMPLETED")?;
    }

    Ok(())
}
