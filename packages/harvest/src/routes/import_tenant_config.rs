// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{anyhow, Context, Result};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;
use windmill::services::tasks_execution::post;
use windmill::tasks::import_tenant_config::{self, ImportOptions};
use windmill::types::tasks::ETasksExecution;
#[derive(Serialize, Deserialize, Debug)]
/// Request body for importing tenant config.
pub struct ImportTenantConfigInput {
    /// The tenant ID.  
    tenant_id: String,
    /// The document ID.
    document_id: String,
    /// The import configurations.
    import_configurations: ImportOptions,
    /// The SHA-256 hash of the document.
    sha256: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
/// Response body for importing tenant config.
pub struct ImportTenantConfigOutput {
    /// The message.
    message: Option<String>,
    /// The error message.
    error: Option<String>,
    /// The task execution.
    task_execution: Option<TasksExecution>,
}

/// Import tenant config.
#[instrument(skip(claims))]
#[post("/import-tenant-config", format = "json", data = "<input>")]
pub async fn import_tenant_config_route(
    claims: jwt::JwtClaims,
    input: Json<ImportTenantConfigInput>,
) -> Result<Json<ImportTenantConfigOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TENANT_WRITE],
    )?;

    let body = input.into_inner();
    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    // Insert the task execution record
    let task_execution = post(
        &body.tenant_id.clone(),
        None,
        ETasksExecution::IMPORT_TENANT_CONFIG,
        &executer_name,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {error:?}"),
        )
    })?;

    let celery_app = get_celery_app().await;

    let celery_task = celery_app
        .send_task(import_tenant_config::import_tenant_config::new(
            body.import_configurations,
            body.tenant_id.clone(),
            body.document_id.clone(),
            body.sha256.clone(),
            task_execution.clone(),
        ))
        .await
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!(
                    "Error sending import_tenant_config task:
    {error:?}"
                ),
            )
        })?;

    let output = ImportTenantConfigOutput {
        message: Some("Upserted Tenant Config successfully".to_string()),
        error: None,
        task_execution: Some(task_execution.clone()),
    };

    Ok(Json(output))
}
