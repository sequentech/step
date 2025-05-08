// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
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
use tracing::instrument;
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;
use windmill::services::tasks_execution::*;
use windmill::tasks::export_tenant_config::{self};
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportVerifiableBulletinBoardInput {
    election_event_id: String,
    tenant_id: String,
    tally_session_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportVerifiableBulletinBoardOutput {
    document_id: String,
    error_msg: Option<String>,
    task_execution: TasksExecution,
}

#[instrument(skip(claims))]
#[post("/export-verifiable-bulletin-board", format = "json", data = "<input>")]
pub async fn export_tenant_config_route(
    claims: jwt::JwtClaims,
    input: Json<ExportVerifiableBulletinBoardInput>,
) -> Result<Json<ExportVerifiableBulletinBoardOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::EXPORT_VERIFIABLE_BULLETIN_BOARD],
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
        ETasksExecution::EXPORT_VERIFIABLE_BULLETIN_BOARD,
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

    // let celery_task = celery_app
    //     .send_task()
    //     .await
    //     .map_err(|error| {
    //         (
    //             Status::InternalServerError,
    //             format!("Error sending _ task: {error:?}"),
    //         )
    //     })?;

    let output = ExportVerifiableBulletinBoardOutput {
        document_id,
        error_msg: None,
        task_execution: task_execution.clone(),
    };

    Ok(Json(output))
}
