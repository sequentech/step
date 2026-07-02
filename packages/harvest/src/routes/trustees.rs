// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{anyhow, Context, Result};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;
use windmill::services::tasks_execution::*;
use windmill::tasks::export_election_event::{self, ExportOptions};
use windmill::tasks::export_trustees;
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportTrusteesInput {
    password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportTrusteesOutput {
    document_id: String,
    task_execution: TasksExecution,
}

#[instrument(skip(claims))]
#[post("/export-trustees", format = "json", data = "<input>")]
pub async fn export_trustees_route(
    claims: jwt::JwtClaims,
    input: Json<ExportTrusteesInput>,
) -> Result<Json<ExportTrusteesOutput>, (Status, String)> {
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let body = input.into_inner();
    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    // Insert the task execution record
    let task_execution = post(
        &tenant_id,
        None,
        ETasksExecution::EXPORT_TRUSTEES,
        &executer_name,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {error:?}"),
        )
    })?;

    if let Err(error) = authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TRUSTEES_EXPORT],
    ) {
        update_fail(
            &task_execution,
            &format!("Failed to authorize executing the task: {error:?}"),
        )
        .await
        .map_err(|err| {
            (
                Status::InternalServerError,
                format!("Failed to record update failure: {err:?} {error:?}"),
            )
        })?;
        return Err(error);
    };

    let document_id = Uuid::new_v4().to_string();
    let celery_app = get_celery_app().await;

    let celery_task = celery_app
        .send_task(export_trustees::export_trustees_task::new(
            tenant_id,
            document_id.clone(),
            body.password.clone(),
            task_execution.clone(),
        ))
        .await
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!("Error sending export_trustees task: {error:?}"),
            )
        })?;

    let output = ExportTrusteesOutput {
        document_id,
        task_execution: task_execution.clone(),
    };

    info!("Sent EXPORT_TRUSTEES task  {:?}", &task_execution);

    Ok(Json(output))
}
