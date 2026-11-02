// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Preview or apply a whole configuration delivery to the caller's tenant.
//! The task does the work; this checks the caller holds every section's
//! permission and stamps the rights the event import uses from their token.

use crate::routes::insert_election_event::stamp_initiators;
use crate::services::audit;
use crate::services::authorization::authorize;
use crate::services::tenant_activity::{TenantAction, TenantActivityRecorder};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::election_config::package_contract::Requirements;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::instrument;
use windmill::services::celery_app::get_celery_app;
use windmill::services::import::configuration_package_apply::ApplyMode;
use windmill::services::tasks_execution::{post, update_fail};
use windmill::tasks::apply_configuration_package::{
    self, ApplyConfigurationPackageBody,
};
use windmill::tasks::import_election_event::ImportElectionEventBody;
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplyConfigurationPackageInput {
    tenant_id: String,
    document_id: String,
    mode: ApplyMode,
    /// The action declares it nullable: absent and null both mean none.
    #[serde(default)]
    requirements: Option<Requirements>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplyConfigurationPackageOutput {
    task_execution: TasksExecution,
}

/// Every permission a package's sections need, for preview as for apply: a
/// preview reads the same tenant state an application changes.
pub const PACKAGE_PERMISSIONS: [Permissions; 6] = [
    Permissions::TENANT_WRITE,
    Permissions::USER_WRITE,
    Permissions::ROLE_WRITE,
    Permissions::TRUSTEE_CREATE,
    Permissions::TRUSTEE_WRITE,
    Permissions::ELECTION_EVENT_CREATE,
];

fn authorize_apply(
    claims: &JwtClaims,
    tenant_id: &str,
) -> Result<(), (Status, String)> {
    authorize(
        claims,
        true,
        Some(tenant_id.to_string()),
        PACKAGE_PERMISSIONS.to_vec(),
    )
}

#[instrument(skip(claims, activity))]
#[post("/apply-configuration-package", format = "json", data = "<input>")]
pub async fn apply_configuration_package_route(
    claims: JwtClaims,
    input: Json<ApplyConfigurationPackageInput>,
    activity: TenantActivityRecorder<'_>,
) -> Result<Json<ApplyConfigurationPackageOutput>, (Status, String)> {
    let input = input.into_inner();
    activity.record(
        &claims,
        TenantAction::ApplyConfigurationPackage,
        json!({
            "tenant_id": input.tenant_id,
            "document_id": input.document_id,
            "mode": input.mode,
        }),
    );
    authorize_apply(&claims, &input.tenant_id)?;
    let mut event_import = ImportElectionEventBody {
        tenant_id: input.tenant_id.clone(),
        document_id: input.document_id.clone(),
        password: None,
        check_only: Some(false),
        sha256: None,
        may_write_secret_attributes: false,
        secret_write_initiator: None,
        importer: None,
        messaging_config_permission: Default::default(),
        privileges: Default::default(),
    };
    stamp_initiators(&mut event_import, &claims);
    let executed_by = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());
    let task_execution = post(
        &input.tenant_id,
        None,
        ETasksExecution::APPLY_CONFIGURATION_PACKAGE,
        &executed_by,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {error:?}"),
        )
    })?;
    let body = ApplyConfigurationPackageBody {
        tenant_id: input.tenant_id.clone(),
        document_id: input.document_id.clone(),
        mode: input.mode,
        requirements: input.requirements.unwrap_or_default(),
        event_import,
        audit_actor: Some(audit::actor(&claims)),
        executed_by,
    };
    let sent = get_celery_app()
        .await
        .send_task(
            apply_configuration_package::apply_configuration_package::new(
                body,
                task_execution.clone(),
            ),
        )
        .await;
    if let Err(error) = sent {
        let message = format!(
            "Error sending apply_configuration_package task: {error:?}"
        );
        let _ = update_fail(&task_execution, &message).await;
        return Err((Status::InternalServerError, message));
    }
    Ok(Json(ApplyConfigurationPackageOutput { task_execution }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::authorization::test_claims::{
        admin_claims, CALLER_TENANT_ID, OTHER_TENANT_ID,
    };

    fn claims_holding(held: &[&Permissions]) -> JwtClaims {
        let roles: Vec<String> = held.iter().map(|p| p.to_string()).collect();
        let roles: Vec<&str> = roles.iter().map(String::as_str).collect();
        admin_claims(CALLER_TENANT_ID, &roles)
    }

    #[test]
    fn requirements_may_be_absent_or_null() {
        for body in [
            json!({"tenant_id": "t", "document_id": "d", "mode": "preview"}),
            json!({"tenant_id": "t", "document_id": "d", "mode": "preview", "requirements": null}),
        ] {
            let input: ApplyConfigurationPackageInput =
                serde_json::from_value(body).unwrap();
            assert_eq!(
                input.requirements.unwrap_or_default(),
                Requirements::default()
            );
        }
    }

    #[test]
    fn applying_needs_every_sections_permission_on_the_callers_tenant() {
        let all: Vec<&Permissions> = PACKAGE_PERMISSIONS.iter().collect();
        let claims = claims_holding(&all);
        assert!(authorize_apply(&claims, CALLER_TENANT_ID).is_ok());
        assert_eq!(
            authorize_apply(&claims, OTHER_TENANT_ID).unwrap_err().0,
            Status::Unauthorized
        );
        // Control above; now drop one permission at a time.
        for missing in &PACKAGE_PERMISSIONS {
            let held: Vec<&Permissions> =
                all.iter().copied().filter(|p| *p != missing).collect();
            assert!(
                authorize_apply(&claims_holding(&held), CALLER_TENANT_ID)
                    .is_err(),
                "{missing:?} must be required"
            );
        }
    }
}
