// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
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
use windmill::services::tasks_execution::*;
use windmill::tasks::export_tenant_config::{self};
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportTenantConfigInput {
    tenant_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportTenantConfigOutput {
    document_id: String,
    error_msg: Option<String>,
    task_execution: TasksExecution,
}

fn authorize_export_tenant_config(
    claims: &JwtClaims,
    input: &ExportTenantConfigInput,
) -> Result<(), (Status, String)> {
    authorize(
        claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::TENANT_READ],
    )
}

#[instrument(skip(claims))]
#[post("/export-tenant-config", format = "json", data = "<input>")]
pub async fn export_tenant_config_route(
    claims: jwt::JwtClaims,
    input: Json<ExportTenantConfigInput>,
) -> Result<Json<ExportTenantConfigOutput>, (Status, String)> {
    let body = input.into_inner();
    authorize_export_tenant_config(&claims, &body)?;

    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    // Insert the task execution record
    let task_execution = post(
        &body.tenant_id.clone(),
        None,
        ETasksExecution::EXPORT_TENANT_CONFIG,
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
    let celery_app = get_celery_app().await;

    let celery_task = celery_app
        .send_task(export_tenant_config::export_tenant_config::new(
            body.tenant_id.clone(),
            document_id.clone(),
            task_execution.clone(),
        ))
        .await
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!("Error sending export_tenant_config task: {error:?}"),
            )
        })?;

    let output = ExportTenantConfigOutput {
        document_id,
        error_msg: None,
        task_execution: task_execution.clone(),
    };

    info!("Sent EXPORT_TENANT_CONFIG task  {:?}", &task_execution);

    Ok(Json(output))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::authorization::test_claims::{
        admin_claims, CALLER_TENANT_ID, OTHER_TENANT_ID,
    };

    fn input(tenant_id: &str) -> ExportTenantConfigInput {
        ExportTenantConfigInput {
            tenant_id: tenant_id.to_string(),
        }
    }

    #[test]
    fn export_tenant_config_rejects_tenant_other_than_callers() {
        let claims = admin_claims(CALLER_TENANT_ID, &["tenant-read"]);
        let result =
            authorize_export_tenant_config(&claims, &input(OTHER_TENANT_ID));
        assert_eq!(result.unwrap_err().0, Status::Unauthorized);
    }

    #[test]
    fn export_tenant_config_accepts_callers_tenant() {
        let claims = admin_claims(CALLER_TENANT_ID, &["tenant-read"]);
        assert!(authorize_export_tenant_config(
            &claims,
            &input(CALLER_TENANT_ID)
        )
        .is_ok());
    }
}
