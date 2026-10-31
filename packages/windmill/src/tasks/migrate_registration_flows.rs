// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Adds the per-Post enrollment check (VOTE-LIFECYCLE §8) to the
//! registration forms of existing event realms and verifies every newly
//! created or imported realm. The gate is REQUIRED before account creation,
//! including nested and disabled form branches. A failed repair leaves signup
//! disabled. Already enforced forms are unchanged, so every run is safe.

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

/// Private realm workflow state: signup stays closed while a gate repair is
/// unfinished, with the latest ordinary operator/scheduler intent retained.
pub const REGISTRATION_RESTORE_ATTRIBUTE: &str = "enrollment_registration_restore";

#[derive(Clone, Copy, strum_macros::Display, strum_macros::EnumString)]
#[strum(serialize_all = "kebab-case")]
enum RegistrationDesire {
    Enabled,
    Disabled,
    ImportEnabled,
    ImportDisabled,
}

impl RegistrationDesire {
    fn of(enabled: bool) -> Self {
        if enabled {
            Self::Enabled
        } else {
            Self::Disabled
        }
    }
    fn for_import(enabled: bool) -> Self {
        if enabled {
            Self::ImportEnabled
        } else {
            Self::ImportDisabled
        }
    }
    fn enabled(self) -> bool {
        matches!(self, Self::Enabled | Self::ImportEnabled)
    }
    fn importing(self) -> bool {
        matches!(self, Self::ImportEnabled | Self::ImportDisabled)
    }
    fn with_desire(self, enabled: bool) -> Self {
        if self.importing() {
            Self::for_import(enabled)
        } else {
            Self::of(enabled)
        }
    }
}

fn pending_registration(
    realm: &keycloak::types::RealmRepresentation,
) -> Result<Option<RegistrationDesire>> {
    realm
        .attributes
        .as_ref()
        .and_then(|attributes| attributes.get(REGISTRATION_RESTORE_ATTRIBUTE))
        .map(|value| {
            value
                .parse::<RegistrationDesire>()
                .context("Invalid pending enrollment registration state")
        })
        .transpose()
}

/// The latest signup intent, including a failed repair or import's recovery state.
pub fn registration_desire(realm: &keycloak::types::RealmRepresentation) -> Result<bool> {
    Ok(pending_registration(realm)?
        .map(RegistrationDesire::enabled)
        .unwrap_or(realm.registration_allowed.unwrap_or(false)))
}

/// Ordinary START/END retains the repair origin and newest intent, while signup
/// stays closed. Unknown recovery states are refused without clearing the pause.
pub fn apply_registration_desire(
    realm: &mut keycloak::types::RealmRepresentation,
    enabled: bool,
) -> Result<()> {
    if let Some(attributes) = realm.attributes.as_mut() {
        if let Some(value) = attributes.get(REGISTRATION_RESTORE_ATTRIBUTE) {
            realm.registration_allowed = Some(false);
            let pending = value
                .parse::<RegistrationDesire>()
                .context("Invalid pending enrollment registration state")?;
            attributes.insert(
                REGISTRATION_RESTORE_ATTRIBUTE.into(),
                pending.with_desire(enabled).to_string(),
            );
            return Ok(());
        }
    }
    realm.registration_allowed = Some(enabled);
    Ok(())
}

fn set_registration_pending(
    realm: &mut keycloak::types::RealmRepresentation,
    pending: RegistrationDesire,
) {
    realm
        .attributes
        .get_or_insert_with(Default::default)
        .insert(REGISTRATION_RESTORE_ATTRIBUTE.into(), pending.to_string());
    realm.registration_allowed = Some(false);
}

/// Import intent must survive until the import itself finishes; guard-only
/// startup repair cannot complete its remaining external setup.
pub fn mark_import_registration_pending(
    realm: &mut keycloak::types::RealmRepresentation,
    desired: bool,
) {
    set_registration_pending(realm, RegistrationDesire::for_import(desired));
}

fn pause_registration(realm: &mut keycloak::types::RealmRepresentation) -> Result<()> {
    let pending = pending_registration(realm)?
        .unwrap_or_else(|| RegistrationDesire::of(realm.registration_allowed.unwrap_or(false)));
    set_registration_pending(realm, pending);
    Ok(())
}

#[derive(Clone, Copy)]
enum RegistrationCompletion {
    RestoreSignup,
    KeepPaused,
    FinishImport,
}

async fn finish_registration_with_completion(
    admin: &KeycloakAdminClient,
    realm: &str,
    completion: RegistrationCompletion,
) -> Result<()> {
    let mut latest = admin.client.realm_get(realm).await?;
    if let Some(pending) = pending_registration(&latest)? {
        if pending.importing() && !matches!(completion, RegistrationCompletion::FinishImport) {
            latest.registration_allowed = Some(false);
        } else {
            if let Some(attributes) = latest.attributes.as_mut() {
                attributes.remove(REGISTRATION_RESTORE_ATTRIBUTE);
            }
            latest.registration_allowed = Some(pending.enabled());
        }
        admin.client.realm_put(realm, latest).await?;
    }
    Ok(())
}

/// Only call after guard verification AND the import's remaining setup have
/// completed, while holding the event scheduling writer lock.
pub async fn finish_registration_setup(admin: &KeycloakAdminClient, realm: &str) -> Result<()> {
    finish_registration_with_completion(admin, realm, RegistrationCompletion::FinishImport).await
}

/// The authenticator of the execution that runs a registration form flow.
const REGISTRATION_PAGE_FORM: &str = "registration-page-form";

/// What a realm's registration flow needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowPlan {
    /// The check is already in the flow.
    Present,
    /// Add the check to this form flow.
    AddTo(String),
    /// Require or move the existing gate in this form.
    RequireIn(String),
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

/// The flattened children of a form, excluding its next sibling's actions.
fn form_children(
    executions: &[AuthenticationExecutionInfoRepresentation],
    index: usize,
) -> &[AuthenticationExecutionInfoRepresentation] {
    let level = executions[index].level.unwrap_or(0);
    let end = ((index + 1)..executions.len())
        .find(|next| executions[*next].level.unwrap_or(0) <= level)
        .unwrap_or(executions.len());
    &executions[index + 1..end]
}

/// Checks every form branch, including disabled branches, independently.
pub fn plan(executions: &[AuthenticationExecutionInfoRepresentation]) -> FlowPlan {
    let mut forms = false;
    for (index, execution) in executions.iter().enumerate() {
        if execution.authentication_flow != Some(true)
            || execution.provider_id.as_deref() != Some(REGISTRATION_PAGE_FORM)
        {
            continue;
        }
        forms = true;
        let Some(alias) = execution.display_name.clone() else {
            return FlowPlan::NoRegistrationForm;
        };
        let children = form_children(executions, index);
        match children
            .iter()
            .position(|child| child.provider_id.as_deref() == Some(ENROLLMENT_WINDOW_CHECK))
        {
            None => return FlowPlan::AddTo(alias),
            Some(position)
                if position != 0
                    || children[position].requirement.as_deref() != Some("REQUIRED") =>
            {
                return FlowPlan::RequireIn(alias)
            }
            Some(_) => {}
        }
    }
    if forms {
        FlowPlan::Present
    } else {
        FlowPlan::NoRegistrationForm
    }
}

/// Verifies imported registration forms before the remaining import setup.
pub async fn migrate_realm_for_import(
    admin: &KeycloakAdminClient,
    public: &PubKeycloakAdmin,
    realm: &str,
) -> Result<FlowOutcome> {
    migrate_realm_with_completion(admin, public, realm, RegistrationCompletion::KeepPaused).await
}

/// Requires the enrollment gate in every installed registration form. A failed
/// repair leaves registration disabled, rather than allowing an unchecked signup.
#[instrument(skip(admin, pub_admin), err)]
pub async fn migrate_realm(
    admin: &KeycloakAdminClient,
    pub_admin: &PubKeycloakAdmin,
    realm: &str,
) -> Result<FlowOutcome> {
    migrate_realm_with_completion(
        admin,
        pub_admin,
        realm,
        RegistrationCompletion::RestoreSignup,
    )
    .await
}

async fn migrate_realm_with_completion(
    admin: &KeycloakAdminClient,
    pub_admin: &PubKeycloakAdmin,
    realm: &str,
    completion: RegistrationCompletion,
) -> Result<FlowOutcome> {
    let mut representation = match admin.client.realm_get(realm).await {
        Ok(representation) => representation,
        Err(KeycloakError::HttpFailure { status: 404, .. }) => return Ok(FlowOutcome::NoRealm),
        Err(err) => return Err(anyhow!("Error reading realm {realm}: {err:?}")),
    };
    let flow = representation
        .registration_flow
        .clone()
        .unwrap_or_else(|| "registration".to_string());
    let mut executions = match admin.get_flow_executions(pub_admin, realm, &flow).await {
        Ok(executions) => executions,
        Err(error) => {
            pause_registration(&mut representation)?;
            admin
                .client
                .realm_put(realm, representation.clone())
                .await?;
            return Err(error.into());
        }
    };
    let mut changed_form = None;
    let limit = executions.len() + 1;
    for _ in 0..limit {
        let (form_flow, add) = match plan(&executions) {
            FlowPlan::Present => {
                match completion {
                    RegistrationCompletion::RestoreSignup
                    | RegistrationCompletion::FinishImport => {
                        finish_registration_with_completion(admin, realm, completion).await?
                    }
                    RegistrationCompletion::KeepPaused => {
                        let mut latest = admin.client.realm_get(realm).await?;
                        pause_registration(&mut latest)?;
                        admin.client.realm_put(realm, latest).await?;
                    }
                }
                return Ok(changed_form
                    .map(|form_flow| FlowOutcome::Added { form_flow })
                    .unwrap_or(FlowOutcome::AlreadyPresent));
            }
            FlowPlan::NoRegistrationForm => {
                if executions.iter().any(|execution| {
                    matches!(
                        execution.provider_id.as_deref(),
                        Some("registration-user-creation" | "deferred-registration-user-creation")
                    )
                }) {
                    pause_registration(&mut representation)?;
                    admin
                        .client
                        .realm_put(realm, representation.clone())
                        .await?;
                    return Err(anyhow!(
                        "Account creation in {realm} has no registration form to guard"
                    ));
                }
                return Ok(FlowOutcome::NoRegistrationForm);
            }
            FlowPlan::AddTo(alias) => (alias, true),
            FlowPlan::RequireIn(alias) => (alias, false),
        };
        if changed_form.is_none() {
            pause_registration(&mut representation)?;
            admin
                .client
                .realm_put(realm, representation.clone())
                .await?;
        }
        let form_id = executions
            .iter()
            .find(|execution| {
                execution.authentication_flow == Some(true)
                    && execution.provider_id.as_deref() == Some(REGISTRATION_PAGE_FORM)
                    && execution.display_name.as_deref() == Some(&form_flow)
            })
            .and_then(|execution| execution.flow_id.as_deref())
            .context("Registration form execution has no flow id")?;
        let original_form = admin
            .client
            .realm_authentication_flows_with_id_get(realm, form_id)
            .await?;
        // Built-in forms reject adding actions. Make only this same form
        // editable while repairing it, preserving its alias and semantics.
        if original_form.built_in == Some(true) {
            let mut editable = original_form.clone();
            editable.built_in = Some(false);
            admin
                .client
                .realm_authentication_flows_with_id_put(realm, form_id, editable)
                .await?;
            if admin
                .client
                .realm_authentication_flows_with_id_get(realm, form_id)
                .await?
                .built_in
                != Some(false)
            {
                return Err(anyhow!(
                    "Registration form {form_flow} of {realm} is not editable for enforcement"
                ));
            }
        }
        if add {
            let url = format!(
                "{}/admin/realms/{realm}/authentication/flows/{form_flow}/executions/execution",
                pub_admin.url
            );
            let token = pub_admin.token_supplier.get(&pub_admin.url).await?;
            let response = pub_admin
                .client
                .post(&url)
                .bearer_auth(token)
                .json(&serde_json::json!({"provider": ENROLLMENT_WINDOW_CHECK}))
                .send()
                .await?;
            if !response.status().is_success() {
                return Err(anyhow!("Keycloak answered {} adding {ENROLLMENT_WINDOW_CHECK} to {form_flow} of {realm}", response.status()));
            }
        }
        let children = admin
            .get_flow_executions(pub_admin, realm, &form_flow)
            .await?;
        let mut check = children
            .iter()
            .find(|execution| execution.provider_id.as_deref() == Some(ENROLLMENT_WINDOW_CHECK))
            .cloned()
            .ok_or_else(|| {
                anyhow!("{ENROLLMENT_WINDOW_CHECK} missing from {form_flow} of {realm}")
            })?;
        check.requirement = Some("REQUIRED".into());
        admin
            .upsert_flow_execution(
                pub_admin,
                realm,
                &form_flow,
                &serde_json::to_string(&check)?,
            )
            .await?;
        let id = check
            .id
            .as_deref()
            .context("Enrollment gate execution has no id")?;
        // Execution PUT changes requirement; priority uses Keycloak's dedicated
        // endpoint. Moving only this execution preserves the other actions' order.
        let position = children
            .iter()
            .position(|execution| execution.id == check.id)
            .context("Enrollment gate execution is missing")?;
        for _ in 0..position {
            admin
                .client
                .realm_authentication_executions_with_execution_id_raise_priority_post(realm, id)
                .await?;
        }
        let verified = admin
            .get_flow_executions(pub_admin, realm, &form_flow)
            .await?;
        if verified.first().is_none_or(|execution| {
            execution.provider_id.as_deref() != Some(ENROLLMENT_WINDOW_CHECK)
                || execution.requirement.as_deref() != Some("REQUIRED")
        }) {
            return Err(anyhow!(
                "Enrollment gate in {form_flow} of {realm} is not required before account creation"
            ));
        }
        if original_form.built_in == Some(true) {
            admin
                .client
                .realm_authentication_flows_with_id_put(realm, form_id, original_form)
                .await?;
            if admin
                .client
                .realm_authentication_flows_with_id_get(realm, form_id)
                .await?
                .built_in
                != Some(true)
            {
                return Err(anyhow!(
                    "Registration form {form_flow} of {realm} did not retain its built-in metadata"
                ));
            }
        }
        changed_form = Some(form_flow);
        executions = admin.get_flow_executions(pub_admin, realm, &flow).await?;
    }
    Err(anyhow!(
        "Registration forms of {realm} did not reach an enforced enrollment gate"
    ))
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
                realms.push((
                    tenant_id.clone(),
                    event.0.id.clone(),
                    get_event_realm(&tenant_id, &event.0.id),
                ));
            }
        }
        realms
    };
    let admin = KeycloakAdminClient::new().await?;
    let pub_admin = KeycloakAdminClient::pub_new().await?;
    let mut migrated = vec![];
    let mut failed = vec![];
    for (tenant, event, realm) in realms {
        // Same writer order as schedule saves, ordinary enrollment changes and
        // window refresh. Never take a separate lock inside migrate_realm,
        // which create/import calls with its existing transaction locked.
        let result = async {
            let mut client = get_hasura_pool().await.get().await?;
            let tx = client.transaction().await?;
            crate::postgres::scheduled_event::lock_scheduling_event(&tx, &tenant, &event).await?;
            let outcome = migrate_realm(&admin, &pub_admin, &realm).await?;
            tx.commit().await?;
            Ok::<_, anyhow::Error>(outcome)
        }
        .await;
        match result {
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
            requirement: Some("REQUIRED".into()),
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
    #[test]
    fn a_disabled_enrollment_check_is_not_an_enforced_registration_gate() {
        let mut check = execution(ENROLLMENT_WINDOW_CHECK, None, 1);
        check.requirement = Some("DISABLED".into());
        let executions = vec![
            execution("registration-page-form", Some("registration form"), 0),
            check,
        ];
        assert_ne!(plan(&executions), FlowPlan::Present);
    }
    #[test]
    fn a_sibling_forms_required_gate_does_not_cover_an_unguarded_form() {
        let executions = vec![
            execution("registration-page-form", Some("first form"), 0),
            execution(ENROLLMENT_WINDOW_CHECK, None, 1),
            execution("registration-user-creation", None, 1),
            execution("registration-page-form", Some("second form"), 0),
            execution("deferred-registration-user-creation", None, 1),
        ];
        assert_eq!(plan(&executions), FlowPlan::AddTo("second form".into()));
    }

    #[test]
    fn a_late_or_alternative_gate_must_be_required_before_account_creation() {
        let mut alternative = execution(ENROLLMENT_WINDOW_CHECK, None, 1);
        alternative.requirement = Some("ALTERNATIVE".into());
        assert_eq!(
            plan(&[
                execution("registration-page-form", Some("form"), 0),
                alternative
            ]),
            FlowPlan::RequireIn("form".into())
        );
        assert_eq!(
            plan(&[
                execution("registration-page-form", Some("form"), 0),
                execution("registration-user-creation", None, 1),
                execution(ENROLLMENT_WINDOW_CHECK, None, 1)
            ]),
            FlowPlan::RequireIn("form".into())
        );
    }
    #[test]
    fn enrollment_start_and_end_keep_signup_closed_until_pending_guard_verification() {
        for (initial, enabled, disabled) in [
            ("enabled", "enabled", "disabled"),
            ("import-enabled", "import-enabled", "import-disabled"),
        ] {
            for (desired, expected) in [(true, enabled), (false, disabled)] {
                let mut realm: keycloak::types::RealmRepresentation =
                    serde_json::from_value(serde_json::json!({
                        "registrationAllowed":false,
                        "attributes":{"enrollment_registration_restore":initial,"other":"kept"}
                    }))
                    .unwrap();
                apply_registration_desire(&mut realm, desired).unwrap();
                assert_eq!(realm.registration_allowed, Some(false));
                assert_eq!(
                    realm.attributes.as_ref().unwrap()["enrollment_registration_restore"],
                    expected
                );
                assert_eq!(realm.attributes.as_ref().unwrap()["other"], "kept");
            }
        }
    }

    #[test]
    fn ordinary_signup_updates_without_pending_repair_preserve_their_existing_behavior() {
        for desired in [true, false] {
            let mut realm = keycloak::types::RealmRepresentation::default();
            apply_registration_desire(&mut realm, desired).unwrap();
            assert_eq!(realm.registration_allowed, Some(desired));
            assert!(realm.attributes.is_none());
        }
    }
    #[test]
    fn unreadable_pending_signup_state_is_refused_and_kept_closed() {
        let mut realm: keycloak::types::RealmRepresentation =
            serde_json::from_value(serde_json::json!({
                "registrationAllowed":true,
                "attributes":{"enrollment_registration_restore":"unknown-phase","other":"kept"}
            }))
            .unwrap();
        assert!(apply_registration_desire(&mut realm, true).is_err());
        assert_eq!(realm.registration_allowed, Some(false));
        assert_eq!(
            realm.attributes.as_ref().unwrap()[REGISTRATION_RESTORE_ATTRIBUTE],
            "unknown-phase"
        );
        assert_eq!(realm.attributes.as_ref().unwrap()["other"], "kept");
    }
}
