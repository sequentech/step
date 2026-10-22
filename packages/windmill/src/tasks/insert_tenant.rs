// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::migrate_tenant_logs::prepare_realm_for_tenant_log;
use super::tenant_bootstrap_admin::TenantBootstrapAdminPolicy;
use crate::postgres::audit_log_outbox::AuditLogTarget;
use crate::postgres::tenant::{
    get_tenant_by_id_if_exist, get_tenant_by_slug_if_exist, insert_tenant,
};
use crate::services::audit_log::{
    self, AdminAction, AdminChange, AuditActor, AuditObject, AuditOrigin,
};
use crate::services::database::get_hasura_pool;
use crate::services::electoral_log_board::ensure_tenant_log;
use crate::services::import::import_election_event::{
    remove_keycloak_realm_secrets, AuthenticatorSecretsPolicy,
};
use crate::services::jwks::upsert_realm_jwks;
use crate::services::keycloak::read_realm_config_from_s3;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::tasks::audit_log_outbox::kick_audit_log_outbox;
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

/// Makes the default realm, without its secrets, a new tenant's: with its
/// bootstrap administrator, and sending its events and admin events to the
/// tenant log from the start, whatever template it comes from.
pub fn prepare_new_tenant_realm(realm: &mut RealmRepresentation) {
    TenantBootstrapAdminPolicy::apply_to_realm(realm);
    prepare_realm_for_tenant_log(realm);
}

#[instrument(err)]
pub async fn upsert_keycloak_realm(tenant_id: &str, slug: &str) -> Result<()> {
    let mut default_tenant = remove_keycloak_realm_secrets(
        &read_default_tenant_realm().await?,
        AuthenticatorSecretsPolicy::Clear,
    )?;
    prepare_new_tenant_realm(&mut default_tenant);
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

#[instrument(err)]
pub async fn process_insert_tenant(
    tenant_id: String,
    slug: String,
    audit_actor: Option<AuditActor>,
) -> Result<()> {
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
    // The tenant's log and its system key exist from the start, and its first
    // entry is the tenant's creation.
    ensure_tenant_log(&hasura_transaction, &tenant_id).await?;
    let created = AdminChange::new(
        AuditLogTarget {
            tenant_id: uuid::Uuid::parse_str(&tenant_id)
                .map_err(|err| format!("Invalid tenant id: {err}"))?,
            election_event_id: None,
        },
        AuditObject::new("tenant", &tenant_id, Some(&slug)),
        AdminAction::Create,
        audit_actor.unwrap_or_default(),
        AuditOrigin::Windmill,
    );
    audit_log::stage(&hasura_transaction, &created).await?;

    hasura_transaction
        .commit()
        .await
        .map_err(|err| format!("Error committing hasura transaction: {err}"))?;
    kick_audit_log_outbox();

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn insert_tenant(
    tenant_id: String,
    slug: String,
    task_execution: Option<TasksExecution>,
    audit_actor: Option<AuditActor>,
) -> Result<()> {
    let res = process_insert_tenant(tenant_id.clone(), slug.clone(), audit_actor).await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::migrate_tenant_logs::EVENT_LISTENER;

    /// A tenant created after the tenant log's one-off migration needs no
    /// migration: its realm feeds the log from the start.
    #[test]
    fn a_new_tenants_realm_feeds_the_tenant_log() {
        let mut realm = RealmRepresentation {
            events_listeners: Some(vec!["jboss-logging".into()]),
            ..Default::default()
        };
        prepare_new_tenant_realm(&mut realm);
        assert_eq!(
            realm.events_listeners.as_deref(),
            Some([EVENT_LISTENER.to_string(), "jboss-logging".to_string()].as_slice())
        );
        assert_eq!(realm.admin_events_enabled, Some(true));
        assert_eq!(realm.admin_events_details_enabled, Some(true));
        let roles = realm.roles.and_then(|roles| roles.realm).unwrap();
        assert!(roles
            .iter()
            .any(|role| role.name.as_deref() == Some("tenant-logs-read")));
    }
}
