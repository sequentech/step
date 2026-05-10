// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! Insert a new tenant record and Keycloak realm.
use crate::postgres::tenant::{
    get_tenant_by_id_if_exist, get_tenant_by_slug_if_exist, insert_tenant as insert_tenant_row,
};
use crate::services::database::get_hasura_pool;
use crate::services::import::import_election_event::remove_keycloak_realm_secrets;
use crate::services::jwks::upsert_realm_jwks;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::types::error::Result;
use ::keycloak::types::RealmRepresentation;
use anyhow::{anyhow, Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use sequent_core;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::services::keycloak::get_tenant_realm;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::types::hasura::core::TasksExecution;
use std::{env, fs};
use tracing::{event, instrument, Level};

/// Reads the default Keycloak realm JSON from `KEYCLOAK_TENANT_REALM_CONFIG_PATH`.
///
/// # Errors
///
/// Returns an error if the file cannot be read or does not deserialize into a [`RealmRepresentation`].
///
/// # Panics
///
/// Panics when `KEYCLOAK_TENANT_REALM_CONFIG_PATH` is not set.
#[instrument(err)]
pub fn read_default_tenant_realm() -> AnyhowResult<RealmRepresentation> {
    let realm_config_path = env::var("KEYCLOAK_TENANT_REALM_CONFIG_PATH")
        .unwrap_or_else(|_| panic!("KEYCLOAK_TENANT_REALM_CONFIG_PATH must be set"));
    let realm_config = fs::read_to_string(&realm_config_path)
        .map_err(|err| anyhow!("Should have been able to read the configuration file in KEYCLOAK_TENANT_REALM_CONFIG_PATH={realm_config_path}. Error: {err}"))?;

    deserialize_str(&realm_config).map_err(|err| {
        anyhow!("Error parsing KEYCLOAK_TENANT_REALM_CONFIG_PATH into RealmRepresentation: {err}")
    })
}

/// Creates or updates the tenant Keycloak realm from the default template and refreshes JWKS.
///
/// # Errors
///
/// Propagates template load, secret stripping, JSON, Keycloak admin, or JWKS errors.
#[instrument(err)]
pub async fn upsert_keycloak_realm(tenant_id: &str, slug: &str) -> Result<()> {
    let mut default_tenant = read_default_tenant_realm()?;
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

/// Inserts a tenant row when no row with the same id exists.
///
/// # Errors
///
/// Propagates Hasura lookup or insert failures.
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

    insert_tenant_row(hasura_transaction, tenant_id, slug).await?;

    Ok(())
}

/// Returns whether a tenant with the given slug already exists.
///
/// # Errors
///
/// Propagates Hasura lookup failures.
#[instrument(skip(hasura_transaction), err)]
pub async fn check_tenant_exists(hasura_transaction: &Transaction<'_>, slug: &str) -> Result<bool> {
    // fetch tenant
    let found_tenant = get_tenant_by_slug_if_exist(hasura_transaction, slug).await?;

    Ok(found_tenant.is_some())
}

/// Full provisioning path: skip if slug taken, otherwise upsert Keycloak realm and insert the tenant, then commit.
///
/// # Errors
///
/// Propagates pool, transaction, Keycloak, insert, or commit failures (surfaced as strings).
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

mod insert_tenant_task {
    #![allow(missing_docs)]
    #![allow(clippy::missing_docs_in_private_items)]

    use super::*;

    /// Celery task: provisions tenant realm and DB row, optionally updating the linked task execution.
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
}

pub use insert_tenant_task::insert_tenant;
