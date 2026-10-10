// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;
use windmill::services::tasks_execution::*;
use windmill::tasks::export_templates;
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportTemplateBody {
    tenant_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportTemplateOutput {
    document_id: String,
    error_msg: Option<String>,
    task_execution: TasksExecution,
}

/// Checks TEMPLATE_WRITE for the requested tenant and returns the caller's
/// tenant, where the export runs and its task is recorded.
fn authorized_export_tenant(
    claims: &jwt::JwtClaims,
    body: &ExportTemplateBody,
) -> Result<String, (Status, String)> {
    authorize(
        claims,
        true,
        Some(body.tenant_id.clone()),
        vec![Permissions::TEMPLATE_WRITE],
    )?;
    Ok(claims.hasura_claims.tenant_id.clone())
}

/// Queues a template export once the caller is authorized for the requested
/// tenant.
#[instrument(skip(claims))]
#[post("/export-template", format = "json", data = "<input>")]
pub async fn export_template(
    claims: jwt::JwtClaims,
    input: Json<ExportTemplateBody>,
) -> Result<Json<ExportTemplateOutput>, (Status, String)> {
    let body = input.into_inner();
    let tenant_id = authorized_export_tenant(&claims, &body)?;

    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    // Insert the task execution record
    let task_execution = post(
        &tenant_id,
        None,
        ETasksExecution::EXPORT_TEMPLATES,
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
        .send_task(export_templates::export_templates::new(
            tenant_id.clone(),
            document_id.clone(),
            task_execution.clone(),
        ))
        .await;

    let _celery_task = match celery_task {
        Ok(celery_task) => celery_task,
        Err(error) => {
            return Ok(Json(ExportTemplateOutput {
                document_id: document_id.clone(),
                error_msg: Some(format!(
                    "Failed to send task to Celery: {error:?}"
                )),
                task_execution: task_execution.clone(),
            }));
        }
    };

    let output = ExportTemplateOutput {
        document_id,
        error_msg: None,
        task_execution,
    };

    Ok(Json(output))
}

#[cfg(test)]
mod export_template_scope_tests {
    use super::*;
    use crate::test_claims::Claims;

    /// Export request for the given tenant.
    fn request(tenant_id: &str) -> ExportTemplateBody {
        ExportTemplateBody {
            tenant_id: tenant_id.into(),
        }
    }

    /// Admin of the given tenant with TEMPLATE_WRITE.
    fn template_writer(tenant_id: &str) -> jwt::JwtClaims {
        Claims::new(tenant_id, "admin")
            .roles([Permissions::TEMPLATE_WRITE])
            .build()
    }

    /// Another tenant, or a caller without TEMPLATE_WRITE, is refused before
    /// the task is recorded.
    #[test]
    fn export_template_rejects_another_tenant_before_recording_the_task() {
        assert_eq!(
            authorized_export_tenant(
                &template_writer("tenant-a"),
                &request("tenant-b")
            )
            .unwrap_err()
            .0,
            Status::Unauthorized
        );
        assert!(authorized_export_tenant(
            &Claims::new("tenant-a", "admin").build(),
            &request("tenant-a")
        )
        .is_err());
    }

    /// The task is recorded in the caller's tenant, the one the export reads.
    #[test]
    fn export_template_records_the_task_in_the_exported_tenant() {
        assert_eq!(
            authorized_export_tenant(
                &template_writer("tenant-a"),
                &request("tenant-a")
            )
            .unwrap(),
            "tenant-a"
        );
    }
}
