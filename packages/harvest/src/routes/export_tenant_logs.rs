// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::authorization::authorize;
use sequent_core::services::jwt;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::instrument;
use uuid::Uuid;
use windmill::services::audit_log::AuditActor;
use windmill::services::audit_log_export::{
    AuditExportFormat, AuditExportRequest, AuditExportScope,
};
use windmill::services::celery_app::get_celery_app;
use windmill::services::tasks_execution::post;
use windmill::tasks::export_audit_logs::export_audit_logs;
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportTenantLogsInput {
    /// `CSV`, `SQL` or `PDF`.
    format: String,
    /// `TENANT_LOG`, `ACCOUNT` or `ACCOUNT_WITH_EVENTS`.
    scope: String,
    #[serde(default)]
    username: Option<String>,
    /// The first minute to export (RFC 3339 instant), inclusive.
    #[serde(default)]
    created_from: Option<String>,
    /// The last minute to export (RFC 3339 instant), inclusive.
    #[serde(default)]
    created_to: Option<String>,
    /// The IANA zone the times are shown in; the tenant's log zone when absent.
    #[serde(default)]
    time_zone: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportTenantLogsOutput {
    document_id: String,
    task_execution: TasksExecution,
}

/// The export the request asks for, of the caller's own tenant.
pub fn tenant_export_request(
    claims: &jwt::JwtClaims,
    input: &ExportTenantLogsInput,
) -> Result<AuditExportRequest, (Status, String)> {
    let bad_request = |error: String| (Status::BadRequest, error);
    let request = AuditExportRequest {
        tenant_id: claims.hasura_claims.tenant_id.clone(),
        election_event_id: None,
        format: AuditExportFormat::from_str(&input.format).map_err(|_| {
            bad_request(format!("Unknown format {:?}", input.format))
        })?,
        scope: AuditExportScope::from_str(&input.scope).map_err(|_| {
            bad_request(format!("Unknown scope {:?}", input.scope))
        })?,
        username: input.username.clone(),
        election_id: None,
        area_id: None,
        created_from: input.created_from.clone(),
        created_to: input.created_to.clone(),
        time_zone: input.time_zone.clone(),
        requested_by: AuditActor {
            user_id: Some(claims.hasura_claims.user_id.clone()),
            username: claims.preferred_username.clone(),
            role: None,
        },
    };
    request
        .validate()
        .map_err(|error| bad_request(format!("{error:#}")))?;
    Ok(request)
}

/// Exports the caller's tenant log. Without `tenant-logs-export` it refuses.
#[instrument(skip(claims))]
#[post("/export-tenant-logs", format = "json", data = "<input>")]
pub async fn export_tenant_logs_route(
    claims: jwt::JwtClaims,
    input: Json<ExportTenantLogsInput>,
) -> Result<Json<ExportTenantLogsOutput>, (Status, String)> {
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    authorize(
        &claims,
        false,
        Some(tenant_id.clone()),
        vec![Permissions::TENANT_LOGS_EXPORT],
    )
    .map_err(|(_, reason)| (Status::Forbidden, reason))?;
    let request = tenant_export_request(&claims, &input)?;

    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());
    let task_execution = post(
        &tenant_id,
        None,
        ETasksExecution::EXPORT_TENANT_LOGS,
        &executer_name,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {error:?}"),
        )
    })?;

    let document_id = Uuid::new_v4().to_string();
    get_celery_app()
        .await
        .send_task(export_audit_logs::new(
            request,
            document_id.clone(),
            task_execution.clone(),
        ))
        .await
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!("Error sending the tenant logs export task: {error:?}"),
            )
        })?;

    Ok(Json(ExportTenantLogsOutput {
        document_id,
        task_execution,
    }))
}

#[cfg(test)]
#[path = "../../tests/support/export_tenant_logs_routes.rs"]
mod route_tests;
