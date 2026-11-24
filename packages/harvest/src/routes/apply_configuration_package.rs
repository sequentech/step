// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Preview or apply a whole configuration delivery to the caller's tenant.
//! The task does the work; this checks the caller holds every section's
//! permission and stamps the rights the event import uses from their token.

use crate::routes::architect_signer_roster::is_tenant_portal_token;
use crate::routes::insert_election_event::stamp_initiators;
use crate::services::audit;
use crate::services::authorization::authorize;
use crate::services::tenant_activity::{TenantAction, TenantActivityRecorder};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::election_config::package_contract::Requirements;
use sequent_core::services::{jwt::JwtClaims, uuid_validation::parse_uuid_v4};
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
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
    /// Exact successful preview for portal-managed package application.
    #[serde(default)]
    preview_task_id: Option<String>,
    /// Digest of the authenticated inner ZIP; also requests signed-only admission.
    #[serde(default)]
    encrypted_payload_sha256: Option<String>,
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

/// Bind the selected Preview to the verified actor without returning their
/// OIDC subject or creating a stable cross-task identifier in task records.
fn managed_preview_operator_binding(
    claims: &JwtClaims,
    tenant_id: &str,
    mode: ApplyMode,
    current_task: &str,
    confirmed_preview: Option<&str>,
) -> Option<String> {
    let tenant = parse_uuid_v4(tenant_id).ok()?;
    if tenant.to_string() != tenant_id
        || claims.hasura_claims.tenant_id != tenant_id
        || !is_tenant_portal_token(claims, tenant_id)
        || claims.sub.is_empty()
        || claims.sub.len() > 255
    {
        return None;
    }
    let preview_task = match mode {
        ApplyMode::Preview => current_task,
        ApplyMode::Apply => confirmed_preview?,
    };
    let preview = parse_uuid_v4(preview_task).ok()?;
    if preview.to_string() != preview_task {
        return None;
    }
    let mut hash = Sha256::new();
    hash.update(b"sequent-package-preview-operator/1\0");
    hash.update(tenant.as_bytes());
    hash.update(preview.as_bytes());
    hash.update(claims.sub.as_bytes());
    Some(format!("{:x}", hash.finalize()))
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
    if input.encrypted_payload_sha256.as_deref().is_some_and(|sha| {
        sha.len() != 64 || !sha.bytes().all(|byte| byte.is_ascii_hexdigit())
    }) {
        return Err((Status::BadRequest, "Invalid encrypted package digest".into()));
    }
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
    let operator_binding = managed_preview_operator_binding(
        &claims,
        &input.tenant_id,
        input.mode,
        &task_execution.id,
        input.preview_task_id.as_deref(),
    );
    let body = ApplyConfigurationPackageBody {
        tenant_id: input.tenant_id.clone(),
        document_id: input.document_id.clone(),
        mode: input.mode,
        requirements: input.requirements.unwrap_or_default(),
        preview_task_id: input.preview_task_id,
        encrypted_payload_sha256: input.encrypted_payload_sha256,
        managed_preview_operator_binding: operator_binding,
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
    fn managed_operator_binding_uses_verified_subject_and_the_specific_preview()
    {
        let tenant = CALLER_TENANT_ID;
        let first = "550e8400-e29b-41d4-a716-446655440000";
        let second = "550e8400-e29b-41d4-a716-446655440001";
        let mut claims = admin_claims(tenant, &[]);
        claims.iss = format!("https://login.example/realms/tenant-{tenant}");
        claims.sub = "test-subject-1".into();
        let stamp = |claims: &JwtClaims, mode, current, preview| {
            managed_preview_operator_binding(
                claims, tenant, mode, current, preview,
            )
        };
        let preview = stamp(&claims, ApplyMode::Preview, first, None).unwrap();
        claims.preferred_username = Some("A different display name".into());
        assert_eq!(
            stamp(&claims, ApplyMode::Apply, second, Some(first)),
            Some(preview.clone())
        );
        assert_ne!(
            stamp(&claims, ApplyMode::Preview, second, None),
            Some(preview.clone())
        );
        claims.sub = "test-subject-2".into();
        assert_ne!(
            stamp(&claims, ApplyMode::Apply, second, Some(first)),
            Some(preview)
        );
        assert!(stamp(&claims, ApplyMode::Apply, second, None).is_none());
        claims.iss = format!(
            "https://login.example/realms/tenant-{tenant}-event-sample"
        );
        assert!(stamp(&claims, ApplyMode::Preview, first, None).is_none());
        assert!(managed_preview_operator_binding(
            &claims,
            OTHER_TENANT_ID,
            ApplyMode::Preview,
            first,
            None
        )
        .is_none());
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
