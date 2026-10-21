// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Remove the built-in raw process logger from election-event realms. The
//! custom listener still publishes authentication outcomes to the signed log;
//! all other listeners and audit-store settings retain their configuration.
use crate::postgres::{election_event::get_all_tenant_election_events, tenant::get_tenant_ids};
use crate::services::database::get_hasura_pool;
use crate::tasks::migrate_realm_permissions::retry_countdown;
use crate::tasks::migrate_tenant_logs::EVENT_LISTENER;
use anyhow::{anyhow, Result};
use celery::task::{Task, TaskResult};
use keycloak::{types::RealmRepresentation, KeycloakTokenSupplier};
use sequent_core::services::keycloak::{get_event_realm, KeycloakAdminClient, PubKeycloakAdmin};
use serde_json::{json, Value};
use tracing::{error, info, instrument};

fn privacy_listeners(listeners: Option<&[String]>) -> Vec<String> {
    let mut kept: Vec<String> = listeners
        .unwrap_or_default()
        .iter()
        .filter(|name| name.as_str() != "jboss-logging")
        .cloned()
        .collect();
    if !kept.iter().any(|name| name == EVENT_LISTENER) {
        kept.push(EVENT_LISTENER.into());
    }
    kept
}

pub fn prepare_event_realm(realm: &mut RealmRepresentation) {
    realm.events_listeners = Some(privacy_listeners(realm.events_listeners.as_deref()));
}

fn privacy_settings(realm: &Value) -> Result<Option<Value>> {
    let current: Option<Vec<String>> =
        serde_json::from_value(realm.get("eventsListeners").cloned().unwrap_or(Value::Null))?;
    let wanted = privacy_listeners(current.as_deref());
    Ok((current.as_ref() != Some(&wanted)).then(|| json!({"eventsListeners":wanted})))
}

/// Missing realms have not been provisioned yet; their creation applies the
/// same listener normalization. A PUT changes only the listener collection.
async fn migrate_realm(client: &PubKeycloakAdmin, realm: &str) -> Result<bool> {
    let url = format!("{}/admin/realms/{realm}", client.url);
    let response = client
        .client
        .get(&url)
        .bearer_auth(client.token_supplier.get(&client.url).await?)
        .send()
        .await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(false);
    }
    if !response.status().is_success() {
        return Err(anyhow!(
            "Cannot read event realm listeners: HTTP {}",
            response.status()
        ));
    }
    let Some(settings) = privacy_settings(&response.json().await?)? else {
        return Ok(false);
    };
    let response = client
        .client
        .put(&url)
        .bearer_auth(client.token_supplier.get(&client.url).await?)
        .json(&settings)
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(anyhow!(
            "Cannot save event realm listeners: HTTP {}",
            response.status()
        ));
    }
    Ok(true)
}

pub async fn migrate_vote_privacy_realms_now() -> Result<()> {
    let realms = {
        let mut db = get_hasura_pool().await.get().await?;
        let tx = db.transaction().await?;
        let mut realms = Vec::new();
        for tenant in get_tenant_ids(&tx).await? {
            for event in get_all_tenant_election_events(&tx, &tenant).await? {
                realms.push(get_event_realm(&tenant, &event.0.id));
            }
        }
        realms
    };
    let client = KeycloakAdminClient::pub_new().await?;
    let mut failures = 0;
    for realm in realms {
        match migrate_realm(&client, &realm).await {
            Ok(changed) => info!(realm, changed, "Event realm privacy listener reconciled"),
            Err(_) => {
                failures += 1;
                error!(realm, "Event realm privacy listener reconciliation failed");
            }
        }
    }
    if failures > 0 {
        return Err(anyhow!(
            "Privacy listener could not be applied to {failures} event realms"
        ));
    }
    Ok(())
}

#[instrument(skip(task))]
#[celery::task(bind = true, max_retries = 5)]
pub async fn migrate_vote_privacy_realms(task: &Self) -> TaskResult<()> {
    match migrate_vote_privacy_realms_now().await {
        Ok(()) => Ok(()),
        Err(_) => {
            let retries = task.request().retries;
            error!(retries, "Event realm privacy listener migration failed");
            task.retry_with_countdown(retry_countdown(retries))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn existing_event_realms_keep_audit_settings_and_unrelated_listeners() {
        let existing = json!({"eventsListeners":["jboss-logging",EVENT_LISTENER,"metrics"],"eventsEnabled":true,"adminEventsEnabled":true,"adminEventsDetailsEnabled":true,"displayName":"Preserved"});
        let patch = privacy_settings(&existing).unwrap().unwrap();
        assert_eq!(patch, json!({"eventsListeners":[EVENT_LISTENER,"metrics"]}));
        assert!(privacy_settings(&patch).unwrap().is_none());
        let mut fresh: RealmRepresentation = serde_json::from_value(existing).unwrap();
        prepare_event_realm(&mut fresh);
        assert_eq!(fresh.events_enabled, Some(true));
        assert_eq!(fresh.admin_events_enabled, Some(true));
        assert_eq!(fresh.admin_events_details_enabled, Some(true));
        assert!(privacy_settings(&serde_json::to_value(fresh).unwrap())
            .unwrap()
            .is_none());
    }
}
