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
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportApplicationBody {
    tenant_id: String,
    election_event_id: String,
    election_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportApplicationOutput {
    document_id: String,
    error_msg: Option<String>,
    task_execution: TasksExecution,
}
#[instrument(skip(claims))]
#[post("/export-application", format = "json", data = "<input>")]
pub async fn export_application_route(
    claims: jwt::JwtClaims,
    input: Json<ExportApplicationBody>,
) -> Result<Json<ExportApplicationOutput>, (Status, String)> {
    let body = input.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let election_event_id = body.election_event_id.clone();
    let election_id = body.election_id.clone();
    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    // Insert the task execution record
    let task_execution = post(
        &tenant_id,
        Some(&election_event_id),
        ETasksExecution::EXPORT_APPLICATION,
        &executer_name,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {error:?}"),
        )
    })?;

    authorize(
        &claims,
        true,
        Some(body.tenant_id.clone()),
        vec![Permissions::APPLICATION_EXPORT],
    )?;

    let document_id = Uuid::new_v4().to_string();
    let celery_app = get_celery_app().await;
    let celery_task_result = match celery_app
        .send_task(
            windmill::tasks::export_application::export_application::new(
                tenant_id.clone(),
                election_event_id.clone(),
                election_id.clone(),
                document_id.clone(),
                task_execution.clone(),
            ),
        )
        .await
    {
        Err(error) => {
            update_fail(
                &task_execution,
                &format!("Error sending export_application task: {error:?}"),
            )
            .await;
            return Err((
                Status::InternalServerError,
                format!("Error sending export_application task: {error:?}"),
            ));
        }
        Ok(task) => task,
    };

    let _res =
        update_complete(&task_execution, Some(document_id.clone())).await;

    let output = ExportApplicationOutput {
        document_id,
        error_msg: None,
        task_execution: task_execution.clone(),
    };

    Ok(Json(output))
}
