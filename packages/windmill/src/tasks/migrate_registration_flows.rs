// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Adds the per-Post enrollment check (VOTE-LIFECYCLE §8) to the
//! registration form of the event realms made before it. New event realms
//! get it from their template (the default event realm and the janitor's);
//! an existing realm gets a REQUIRED `enrollment-window-check` execution
//! appended to the form flow of its registration flow. A realm that already
//! has it is left alone, so every run is safe.

use crate::postgres::election_event::get_all_tenant_election_events;
use crate::postgres::tenant::get_tenant_ids;
use crate::services::database::get_hasura_pool;
use crate::tasks::migrate_realm_permissions::retry_countdown;
use anyhow::{anyhow, Context, Result};
use celery::task::{Task, TaskResult};
use deadpool_postgres::Client as DbClient;
use keycloak::types::AuthenticationExecutionInfoRepresentation;
use keycloak::{KeycloakError, KeycloakTokenSupplier};
use sequent_core::services::keycloak::{get_event_realm, KeycloakAdminClient, PubKeycloakAdmin};
use tracing::{error, info, instrument};

/// The form action the registration form needs.
pub const ENROLLMENT_WINDOW_CHECK: &str = "enrollment-window-check";

/// The authenticator of the execution that runs a registration form flow.
const REGISTRATION_PAGE_FORM: &str = "registration-page-form";

/// What a realm's registration flow needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowPlan {
    /// The check is already in the flow.
    Present,
    /// Add the check to this form flow.
    AddTo(String),
    /// The flow has no registration form: nothing to add the check to.
    NoRegistrationForm,
}

/// What happened to one realm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowOutcome {
    Added {
        form_flow: String,
    },
    AlreadyPresent,
    NoRegistrationForm,
    /// The event has no realm (yet).
    NoRealm,
}

/// Decides from the flattened executions of the realm's registration flow.
pub fn plan(executions: &[AuthenticationExecutionInfoRepresentation]) -> FlowPlan {
    if executions
        .iter()
        .any(|execution| execution.provider_id.as_deref() == Some(ENROLLMENT_WINDOW_CHECK))
    {
        return FlowPlan::Present;
    }
    executions
        .iter()
        .find(|execution| {
            execution.authentication_flow == Some(true)
                && execution.provider_id.as_deref() == Some(REGISTRATION_PAGE_FORM)
        })
        .and_then(|execution| execution.display_name.clone())
        .map(FlowPlan::AddTo)
        .unwrap_or(FlowPlan::NoRegistrationForm)
}

/// Adds the check to one event realm when it lacks it.
#[instrument(skip(admin, pub_admin), err)]
pub async fn migrate_realm(
    admin: &KeycloakAdminClient,
    pub_admin: &PubKeycloakAdmin,
    realm: &str,
) -> Result<FlowOutcome> {
    let representation = match admin.client.realm_get(realm).await {
        Ok(representation) => representation,
        Err(KeycloakError::HttpFailure { status: 404, .. }) => return Ok(FlowOutcome::NoRealm),
        Err(err) => return Err(anyhow!("Error reading realm {realm}: {err:?}")),
    };
    let flow = representation
        .registration_flow
        .clone()
        .unwrap_or_else(|| "registration".to_string());
    let executions = admin
        .get_flow_executions(pub_admin, realm, &flow)
        .await
        .map_err(|err| anyhow!("Error reading flow {flow} of {realm}: {err:?}"))?;
    let form_flow = match plan(&executions) {
        FlowPlan::Present => return Ok(FlowOutcome::AlreadyPresent),
        FlowPlan::NoRegistrationForm => return Ok(FlowOutcome::NoRegistrationForm),
        FlowPlan::AddTo(form_flow) => form_flow,
    };

    // Keycloak appends the new execution, DISABLED.
    let url = format!(
        "{}/admin/realms/{realm}/authentication/flows/{form_flow}/executions/execution",
        pub_admin.url
    );
    let token = pub_admin
        .token_supplier
        .get(&pub_admin.url)
        .await
        .map_err(|err| anyhow!("Error getting a Keycloak token: {err:?}"))?;
    let response = pub_admin
        .client
        .post(&url)
        .bearer_auth(token)
        .json(&serde_json::json!({ "provider": ENROLLMENT_WINDOW_CHECK }))
        .send()
        .await
        .with_context(|| format!("Error adding {ENROLLMENT_WINDOW_CHECK} to {realm}"))?;
    if !response.status().is_success() {
        return Err(anyhow!(
            "Keycloak answered {} adding {ENROLLMENT_WINDOW_CHECK} to {form_flow} of {realm}",
            response.status()
        ));
    }

    let mut added = admin
        .get_flow_executions(pub_admin, realm, &flow)
        .await
        .map_err(|err| anyhow!("Error reading flow {flow} of {realm}: {err:?}"))?
        .into_iter()
        .find(|execution| execution.provider_id.as_deref() == Some(ENROLLMENT_WINDOW_CHECK))
        .ok_or_else(|| anyhow!("{ENROLLMENT_WINDOW_CHECK} missing from {realm} after adding it"))?;
    added.requirement = Some("REQUIRED".into());
    admin
        .upsert_flow_execution(
            pub_admin,
            realm,
            &form_flow,
            &serde_json::to_string(&added)?,
        )
        .await
        .with_context(|| format!("Error requiring {ENROLLMENT_WINDOW_CHECK} in {realm}"))?;
    Ok(FlowOutcome::Added { form_flow })
}

/// [`migrate_realm`] for the realm of every election event. Every realm is
/// tried; if any failed the result is an error naming them, after the
/// others were migrated.
#[instrument(err)]
pub async fn migrate_registration_flows_now() -> Result<Vec<(String, FlowOutcome)>> {
    let realms = {
        let mut hasura_db_client: DbClient = get_hasura_pool()
            .await
            .get()
            .await
            .context("Error getting hasura client")?;
        let hasura_transaction = hasura_db_client
            .transaction()
            .await
            .context("Error starting hasura transaction")?;
        let mut realms = vec![];
        for tenant_id in get_tenant_ids(&hasura_transaction).await? {
            for event in get_all_tenant_election_events(&hasura_transaction, &tenant_id).await? {
                realms.push(get_event_realm(&tenant_id, &event.0.id));
            }
        }
        realms
    };
    let admin = KeycloakAdminClient::new().await?;
    let pub_admin = KeycloakAdminClient::pub_new().await?;
    let mut migrated = vec![];
    let mut failed = vec![];
    for realm in realms {
        match migrate_realm(&admin, &pub_admin, &realm).await {
            Ok(outcome) => {
                info!(realm, ?outcome, "Registration flow migrated");
                migrated.push((realm, outcome));
            }
            Err(err) => {
                error!(realm, "Registration flow not migrated: {err:?}");
                failed.push(realm);
            }
        }
    }
    if failed.is_empty() {
        Ok(migrated)
    } else {
        Err(anyhow!(
            "Registration flow not migrated in {}",
            failed.join(", ")
        ))
    }
}

/// Retried with a growing wait when the migration fails, as when Keycloak
/// is still starting with beat; every run skips the realms that have it.
#[instrument(skip(task))]
#[celery::task(bind = true, max_retries = 5)]
pub async fn migrate_registration_flows(task: &Self) -> TaskResult<()> {
    match migrate_registration_flows_now().await {
        Ok(_) => Ok(()),
        Err(err) => {
            let retries = task.request().retries;
            error!(retries, "Registration flow migration failed: {err:?}");
            task.retry_with_countdown(retry_countdown(retries))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn execution(
        provider: &str,
        flow: Option<&str>,
        level: i32,
    ) -> AuthenticationExecutionInfoRepresentation {
        AuthenticationExecutionInfoRepresentation {
            provider_id: Some(provider.into()),
            authentication_flow: Some(flow.is_some()),
            display_name: Some(flow.unwrap_or(provider).into()),
            level: Some(level),
            ..Default::default()
        }
    }

    #[test]
    fn the_default_event_realm_gets_the_check_in_its_registration_form() {
        let executions = vec![
            execution("registration-page-form", Some("registration form"), 0),
            execution("registration-user-creation", None, 1),
            execution("registration-password-action", None, 1),
        ];
        assert_eq!(
            plan(&executions),
            FlowPlan::AddTo("registration form".to_string())
        );
    }

    #[test]
    fn a_deferred_registration_realm_gets_it_in_its_own_form_flow() {
        let executions = vec![
            execution(
                "registration-page-form",
                Some("comelec-registration registration form"),
                0,
            ),
            execution("deferred-registration-user-creation", None, 1),
            execution("message-otp-authenticator", None, 0),
            execution("lookup-and-update-user", None, 0),
        ];
        assert_eq!(
            plan(&executions),
            FlowPlan::AddTo("comelec-registration registration form".to_string())
        );
    }

    #[test]
    fn a_realm_that_has_it_or_has_no_form_is_left_alone() {
        let migrated = vec![
            execution("registration-page-form", Some("registration form"), 0),
            execution(ENROLLMENT_WINDOW_CHECK, None, 1),
            execution("registration-user-creation", None, 1),
        ];
        assert_eq!(plan(&migrated), FlowPlan::Present);
        assert_eq!(
            plan(&[execution("auth-cookie", None, 0)]),
            FlowPlan::NoRegistrationForm
        );
    }
}
