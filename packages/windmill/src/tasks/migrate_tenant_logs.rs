// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Gives the tenants made before the tenant log what new tenants are created
//! with: the log, its system key, and a realm that sends its events and
//! admin events to the listener that feeds the log.
//!
//! Beat sends the task once, whenever it starts; a tenant that has its log
//! and whose realm is already set up is left as it is.

use crate::postgres::tenant::get_tenant_ids;
use crate::services::database::get_hasura_pool;
use crate::services::electoral_log_board::ensure_tenant_log;
use crate::tasks::migrate_realm_permissions::{admin_get, retry_countdown, top_level_group};
use anyhow::{anyhow, Context};
use celery::task::{Task, TaskResult};
use deadpool_postgres::Client as DbClient;
use keycloak::types::RoleRepresentation;
use keycloak::KeycloakTokenSupplier;
use sequent_core::services::keycloak::{
    get_tenant_realm, partial_import_realm_roles, IfResourceExists, KeycloakAdminClient,
    PubKeycloakAdmin,
};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use tracing::{error, info, instrument};

/// The Keycloak event listener that publishes a realm's events to Windmill.
pub const EVENT_LISTENER: &str = "custom-event-listener";

/// What a realm needs changed to feed the tenant log, or `None` when it
/// already does: the listener among its event listeners, and admin events
/// with their representation.
pub fn realm_log_settings(realm: &Value) -> Option<Value> {
    let mut listeners: Vec<Value> = realm["eventsListeners"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let listens = listeners.iter().any(|listener| listener == EVENT_LISTENER);
    let admin_events = realm["adminEventsEnabled"] == true;
    let admin_details = realm["adminEventsDetailsEnabled"] == true;
    if listens && admin_events && admin_details {
        return None;
    }
    if !listens {
        listeners.insert(0, json!(EVENT_LISTENER));
    }
    Some(json!({
        "eventsListeners": listeners,
        "adminEventsEnabled": true,
        "adminEventsDetailsEnabled": true,
    }))
}

/// Sets a realm up to feed the tenant log; `false` when it already was.
#[instrument(skip(client), err)]
pub async fn set_up_realm(client: &PubKeycloakAdmin, realm: &str) -> anyhow::Result<bool> {
    let url = format!("{}/admin/realms/{realm}", client.url);
    let response = client
        .client
        .get(&url)
        .bearer_auth(client.token_supplier.get(&client.url).await?)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(anyhow!("GET {realm}: HTTP {status}"));
    }
    let Some(settings) = realm_log_settings(&response.json().await?) else {
        return Ok(false);
    };
    let response = client
        .client
        .put(&url)
        .bearer_auth(client.token_supplier.get(&client.url).await?)
        .json(&settings)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(anyhow!("PUT {realm}: HTTP {status}"));
    }
    Ok(true)
}

/// The tenant log's permissions, with the English label Users and Roles
/// shows for each, which becomes the role's description.
pub const TENANT_LOG_PERMISSIONS: [(Permissions, &str); 2] = [
    (Permissions::TENANT_LOGS_READ, "Read the tenant log"),
    (Permissions::TENANT_LOGS_EXPORT, "Export the tenant log"),
];
/// The group that holds them in the realm templates.
pub const TENANT_LOG_GROUP: &str = "admin";

/// The realm roles of the tenant log's permissions, without ids, so Keycloak
/// gives each realm its own.
pub fn tenant_log_roles() -> Vec<RoleRepresentation> {
    TENANT_LOG_PERMISSIONS
        .iter()
        .map(|(permission, label)| RoleRepresentation {
            name: Some(permission.to_string()),
            description: Some(label.to_string()),
            composite: Some(false),
            client_role: Some(false),
            ..Default::default()
        })
        .collect()
}

/// The tenant log roles a group holding `held` still lacks.
pub fn missing_tenant_log_roles(held: &[RoleRepresentation]) -> Vec<String> {
    TENANT_LOG_PERMISSIONS
        .iter()
        .map(|(permission, _)| permission.to_string())
        .filter(|role| !held.iter().any(|h| h.name.as_deref() == Some(role)))
        .collect()
}

/// Adds the tenant log's roles to a realm that lacks them and grants them to
/// its administrators' group, as the realm templates do. Returns the roles
/// granted now.
#[instrument(skip(client), err)]
pub async fn grant_tenant_log_roles(
    client: &PubKeycloakAdmin,
    realm: &str,
) -> anyhow::Result<Vec<String>> {
    partial_import_realm_roles(client, realm, &tenant_log_roles(), IfResourceExists::Skip).await?;
    let Some(group) = top_level_group(client, realm, TENANT_LOG_GROUP).await? else {
        return Ok(vec![]);
    };
    let Some(group_id) = group.id else {
        return Ok(vec![]);
    };
    let held: Vec<RoleRepresentation> = admin_get(
        client,
        &format!("{realm}/groups/{group_id}/role-mappings/realm"),
        &[],
    )
    .await?;
    let missing = missing_tenant_log_roles(&held);
    if missing.is_empty() {
        return Ok(missing);
    }
    let mut mapping = vec![];
    for name in &missing {
        let role: RoleRepresentation =
            admin_get(client, &format!("{realm}/roles/{name}"), &[]).await?;
        mapping.push(json!({"id": role.id, "name": role.name}));
    }
    let response = client
        .client
        .post(format!(
            "{}/admin/realms/{realm}/groups/{group_id}/role-mappings/realm",
            client.url
        ))
        .bearer_auth(client.token_supplier.get(&client.url).await?)
        .json(&mapping)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(anyhow!(
            "granting {missing:?} to {TENANT_LOG_GROUP}: HTTP {status}"
        ));
    }
    Ok(missing)
}

/// Creates the log and key of every tenant that lacks them, each in a
/// transaction of its own. Returns the tenants, for their realms.
#[instrument(err)]
pub async fn create_tenant_logs() -> anyhow::Result<Vec<String>> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .context("Error getting hasura client")?;
    let tenant_ids = {
        let hasura_transaction = hasura_db_client.transaction().await?;
        let tenant_ids = get_tenant_ids(&hasura_transaction).await?;
        hasura_transaction.commit().await?;
        tenant_ids
    };
    for tenant_id in &tenant_ids {
        let hasura_transaction = hasura_db_client.transaction().await?;
        let log = ensure_tenant_log(&hasura_transaction, tenant_id).await?;
        hasura_transaction
            .commit()
            .await
            .with_context(|| format!("Error saving the log key of tenant {tenant_id}"))?;
        info!(tenant_id, board = log.board, "Tenant log ready");
    }
    Ok(tenant_ids)
}

/// Every tenant's log and realm. Every realm is tried; if any failed the
/// result is an error naming them, after the others were set up.
#[instrument(err)]
pub async fn migrate_tenant_logs_now() -> anyhow::Result<()> {
    let tenant_ids = create_tenant_logs().await?;
    let client = KeycloakAdminClient::pub_new().await?;
    let mut failed = vec![];
    for tenant_id in &tenant_ids {
        let realm = get_tenant_realm(tenant_id);
        let migrated = async {
            let changed = set_up_realm(&client, &realm).await?;
            let granted = grant_tenant_log_roles(&client, &realm).await?;
            anyhow::Ok((changed, granted))
        };
        match migrated.await {
            Ok((changed, granted)) => {
                info!(realm, changed, ?granted, "Realm feeds the tenant log")
            }
            Err(err) => {
                error!(realm, "Realm not set up for the tenant log: {err:?}");
                failed.push(realm);
            }
        }
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(anyhow!(
            "Realms not set up for the tenant log: {}",
            failed.join(", ")
        ))
    }
}

/// Retried with a growing wait when it fails, as when Keycloak is still
/// starting with beat.
#[instrument(skip(task))]
#[celery::task(bind = true, max_retries = 5)]
pub async fn migrate_tenant_logs(task: &Self) -> TaskResult<()> {
    match migrate_tenant_logs_now().await {
        Ok(()) => Ok(()),
        Err(err) => {
            let retries = task.request().retries;
            error!(retries, "Tenant log migration failed: {err:?}");
            task.retry_with_countdown(retry_countdown(retries))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_realm_that_feeds_the_log_is_left_alone() {
        let realm = json!({
            "eventsListeners": ["jboss-logging", EVENT_LISTENER],
            "adminEventsEnabled": true,
            "adminEventsDetailsEnabled": true,
        });
        assert_eq!(realm_log_settings(&realm), None);
    }

    #[test]
    fn a_realm_gets_the_listener_and_admin_events_it_lacks() {
        let realm = json!({
            "eventsListeners": ["jboss-logging"],
            "adminEventsEnabled": false,
        });
        assert_eq!(
            realm_log_settings(&realm),
            Some(json!({
                "eventsListeners": [EVENT_LISTENER, "jboss-logging"],
                "adminEventsEnabled": true,
                "adminEventsDetailsEnabled": true,
            }))
        );
        // Only the details are missing: the listeners stay as they are.
        let realm = json!({
            "eventsListeners": [EVENT_LISTENER],
            "adminEventsEnabled": true,
            "adminEventsDetailsEnabled": false,
        });
        assert_eq!(
            realm_log_settings(&realm).unwrap()["eventsListeners"],
            json!([EVENT_LISTENER])
        );
        assert_eq!(
            realm_log_settings(&json!({})).unwrap()["eventsListeners"],
            json!([EVENT_LISTENER])
        );
    }

    #[test]
    fn the_tenant_log_roles_are_the_permissions_with_their_labels() {
        let roles = tenant_log_roles();
        let names: Vec<_> = roles
            .iter()
            .map(|role| role.name.clone().unwrap())
            .collect();
        assert_eq!(names, ["tenant-logs-read", "tenant-logs-export"]);
        assert_eq!(roles[0].description.as_deref(), Some("Read the tenant log"));
        assert!(roles.iter().all(|role| role.id.is_none()));
        // A group that holds one is granted the other.
        assert_eq!(
            missing_tenant_log_roles(&roles[..1]),
            ["tenant-logs-export"]
        );
        assert!(missing_tenant_log_roles(&roles).is_empty());
        assert_eq!(missing_tenant_log_roles(&[]).len(), 2);
    }
}
