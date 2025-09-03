// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{anyhow, Context, Result};
use rocket::http::Status;
use rocket::serde::json::{self, Json};
use sequent_core::services::jwt;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;
use windmill::services::tasks_execution::*;
use windmill::tasks::export_ballot_publication::export_ballot_publication;
use windmill::tasks::export_election_event::{self, ExportOptions};
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportTallyResultsInput {
    tenant_id: String,
    election_event_id: String,
    results_event_id: String,
    results_sqlite_document_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportTallyResultsOutput {
    document_id: String,
    task_execution: TasksExecution,
    error_msg: Option<String>,
}

#[instrument(skip(claims))]
#[post("/export-tally-results", format = "json", data = "<input>")]
pub async fn export_tally_results_route(
    claims: jwt::JwtClaims,
    input: Json<ExportTallyResultsInput>,
) -> Result<Json<ExportTallyResultsOutput>, (Status, String)> {
    let body = input.into_inner();

    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TALLY_RESULTS_READ],
    )?;

    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let election_event_id = body.election_event_id.clone();
    let results_sqlite_document_id = body.results_sqlite_document_id.clone();
    let results_event_id = body.results_event_id.clone();

    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    let task_execution = post(
        &tenant_id,
        Some(&election_event_id),
        ETasksExecution::EXPORT_TALLY_RESULTS_XLSX,
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

    let _celery_task = match celery_app.send_task(windmill::tasks::export_tally_results::export_tally_results_to_xlsx_task::new(
        tenant_id,
        election_event_id,
        results_sqlite_document_id,
        results_event_id,
        document_id.clone(),
        task_execution.clone(),
    ))
        .await
    {
        Ok(celery_task) => celery_task,
        Err(err) => {
            return Ok(Json(ExportTallyResultsOutput {
                document_id,
                task_execution: task_execution.clone(),
                error_msg: Some(format!("Failed to send EXPORT_TALLY_RESULTS_XLSX task: {err:?}")),
            }))
        }
    };

    let output = ExportTallyResultsOutput {
        document_id,
        task_execution: task_execution.clone(),
        error_msg: None,
    };

    info!("Sent EXPORT_ELECTION_EVENT task {task_execution:?}");

    Ok(Json(output))
}
