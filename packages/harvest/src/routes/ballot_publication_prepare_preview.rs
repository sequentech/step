// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;
use windmill::services::tasks_execution::*;
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Deserialize, Debug)]
pub struct PreparePublPreviewInput {
    election_event_id: String,
    ballot_publication_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PreparePublPreviewOutput {
    error_msg: Option<String>,
    document_id: String,
    task_execution: Option<TasksExecution>,
}

#[instrument(skip(claims))]
#[post(
    "/prepare-ballot-publication-preview",
    format = "json",
    data = "<input>"
)]
pub async fn prepare_ballot_publication_preview(
    claims: jwt::JwtClaims,
    input: Json<PreparePublPreviewInput>,
) -> Result<Json<PreparePublPreviewOutput>, (Status, String)> {
    let body = input.into_inner();
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::DOCUMENT_UPLOAD],
    )?;

    let document_id = Uuid::new_v4().to_string();
    let ballot_publication_id = body.ballot_publication_id.clone();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let election_event_id = body.election_event_id.clone();
    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    // Insert the task execution record
    let task_execution: TasksExecution = match post(
        &tenant_id,
        Some(&election_event_id),
        ETasksExecution::PREPARE_PUBLICATION_PREVIEW,
        &executer_name,
    )
    .await
    {
        Ok(task_execution) => task_execution,
        Err(err) => {
            return Ok(Json(PreparePublPreviewOutput {
                error_msg: Some(format!(
                    "Failed to insert task execution record: {err:?}"
                )),
                document_id,
                task_execution: None,
            }));
        }
    };

    let celery_app = get_celery_app().await;
    let _celery_task = match celery_app
        .send_task(
            windmill::tasks::prepare_publication_preview::prepare_publication_preview::new(
                tenant_id,
                election_event_id,
                ballot_publication_id,
                task_execution.clone(),
                document_id.clone(),
            ),
        )
        .await
    {
        Ok(celery_task) => celery_task,
        Err(err) => {
            return Ok(Json(PreparePublPreviewOutput {
                error_msg: Some(format!(
                    "Error sending prepare_publication_preview task: ${err}"
                )),
                document_id,
                task_execution: Some(task_execution),
            }));
        }
    };

    Ok(Json(PreparePublPreviewOutput {
        error_msg: None,
        document_id,
        task_execution: Some(task_execution),
    }))
}
