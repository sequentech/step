// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
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
#[instrument(skip(claims))]
#[post("/export-template", format = "json", data = "<input>")]
pub async fn export_template(
    claims: jwt::JwtClaims,
    input: Json<ExportTemplateBody>,
) -> Result<Json<ExportTemplateOutput>, (Status, String)> {
    let body = input.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    authorize(
        &claims,
        true,
        Some(body.tenant_id.clone()),
        vec![Permissions::TEMPLATE_WRITE],
    )?;

    let document_id = Uuid::new_v4().to_string();

    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    // Insert the task execution record
    let task_execution = post(
        &body.tenant_id.clone(),
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

    let celery_app = get_celery_app().await;
    let celery_task = celery_app
        .send_task(export_templates::export_templates::new(
            tenant_id.clone(),
            document_id.clone(),
        ))
        .await;

    let _celery_task = match celery_task {
        Ok(celery_task) => celery_task,
        Err(error) => {
            let _ = update_fail(
                &task_execution,
                &format!("Error sending insert_tenant task: {error:?}"),
            )
            .await;
            return Err((
                Status::InternalServerError,
                format!("Error sending insert_tenant task: {error:?}"),
            ));
        }
    };

    info!("Sent EXPORT_TEMPLATES task {task_execution:?}");
    let _res = update_complete(&task_execution, None).await;

    let output = ExportTemplateOutput {
        document_id,
        error_msg: None,
        task_execution,
    };

    Ok(Json(output))
}
