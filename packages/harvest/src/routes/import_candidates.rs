// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
use tracing::{event, instrument, Level};
use windmill::services::tasks_execution::*;
use windmill::{
    tasks::import_candidates::import_candidates_task,
    types::tasks::ETasksExecution,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportCandidatesInput {
    election_event_id: String,
    document_id: String,
    sha256: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportCandidatesOutput {
    error_msg: Option<String>,
    document_id: String,
    task_execution: TasksExecution,
}

#[instrument(skip(claims))]
#[post("/import-candidates", format = "json", data = "<input>")]
pub async fn import_candidates_route(
    claims: jwt::JwtClaims,
    input: Json<ImportCandidatesInput>,
) -> Result<Json<ImportCandidatesOutput>, (Status, String)> {
    let body = input.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let election_event_id = body.election_event_id.clone();
    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    // Insert the task execution record
    let task_execution = post(
        &tenant_id,
        Some(&election_event_id),
        ETasksExecution::IMPORT_CANDIDATES,
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
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ADMIN_USER],
    )?;

    match import_candidates_task(
        claims.hasura_claims.tenant_id.clone(),
        election_event_id,
        body.document_id.clone(),
        task_execution.clone(),
        body.sha256.clone(),
    )
    .await
    {
        Ok(_) => (),
        Err(err) => {
            return Ok(Json(ImportCandidatesOutput {
                error_msg: Some(err.to_string()),
                document_id: body.document_id.clone(),
                task_execution: task_execution.clone(),
            }));
        }
    };

    let output = ImportCandidatesOutput {
        error_msg: None,
        document_id: body.document_id.clone(),
        task_execution: task_execution.clone(),
    };

    Ok(Json(output))
}
