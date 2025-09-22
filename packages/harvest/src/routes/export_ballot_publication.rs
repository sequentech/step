// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
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
pub struct ExportBallotPublicationInput {
    tenant_id: String,
    election_event_id: String,
    election_id: Option<String>,
    ballot_publication_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportBallotPublicationOutput {
    document_id: String,
    task_execution: TasksExecution,
}

#[instrument(skip(claims))]
#[post("/export-ballot-publication", format = "json", data = "<input>")]
pub async fn export_ballot_publication_route(
    claims: jwt::JwtClaims,
    input: Json<ExportBallotPublicationInput>,
) -> Result<Json<ExportBallotPublicationOutput>, (Status, String)> {
    let body = input.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let election_event_id = body.election_event_id.clone();
    let ballot_publication_id = body.ballot_publication_id.clone();
    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    // Insert the task execution record
    let task_execution = post(
        &tenant_id,
        Some(&election_event_id),
        ETasksExecution::EXPORT_BALLOT_PUBLICATION,
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
        vec![Permissions::PUBLISH_WRITE],
    ) {
        update_fail(
            &task_execution,
            &format!("Failed to authorize executing the task: {:?}", error),
        )
        .await;
        return Err(error);
    };

    let document_id = Uuid::new_v4().to_string();
    let celery_app = get_celery_app().await;

    let celery_task = match celery_app
    .send_task(windmill::tasks::export_ballot_publication::export_ballot_publication::new(
        tenant_id,
        election_event_id,
        document_id.clone(),
        ballot_publication_id.clone(),
        task_execution.clone(),
    ))
    .await {
        Err(error) =>  {
            update_fail(&task_execution, &format!("Failed to send task to the queue: {error:?}")).await;
            return Err((
                Status::InternalServerError,
                format!("Error sending export_election_event task: {error:?}")
            ));
        },
        Ok(task) => task,
    };

    let output = ExportBallotPublicationOutput {
        document_id,
        task_execution: task_execution.clone(),
    };

    info!("Sent EXPORT_ELECTION_EVENT task {task_execution:?}");

    Ok(Json(output))
}
