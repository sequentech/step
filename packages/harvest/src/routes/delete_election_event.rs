// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use windmill::postgres::tenant;
use windmill::services::celery_app::get_celery_app;
use windmill::services::tasks_execution::*;
use windmill::services::tasks_execution::{update_complete, update_fail};
use windmill::tasks::delete_election_event;
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteElectionEventOutput {
    id: String,
    task_execution: TasksExecution,
    error_msg: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteElectionEventInput {
    election_event_id: String,
}

#[instrument(skip(claims))]
#[post("/delete-election-event", format = "json", data = "<body>")]
pub async fn delete_election_event_f(
    body: Json<DeleteElectionEventInput>,
    claims: JwtClaims,
) -> Result<Json<DeleteElectionEventOutput>, (Status, String)> {
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    let task_execution = post(
        &tenant_id,
        None,
        ETasksExecution::DELETE_ELECTION_EVENT,
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
        Some(tenant_id.clone()),
        vec![Permissions::ELECTION_EVENT_DELETE],
    ) {
        let _ = update_fail(
            &task_execution,
            &format!("Failed to authorize executing the task: {error:?}"),
        )
        .await;
        return Err(error);
    };

    let celery_app = get_celery_app().await;

    let realm = get_event_realm(&tenant_id, &input.election_event_id);

    let celery_task_result = celery_app
        .send_task(delete_election_event::delete_election_event_t::new(
            claims.hasura_claims.tenant_id.clone(),
            input.election_event_id.clone(),
            realm.clone(),
            task_execution.clone(),
        ))
        .await;

    let _celery_task = match celery_task_result {
        Ok(task) => task,
        Err(error) => {
            return Ok(Json(DeleteElectionEventOutput {
                id: input.election_event_id,
                error_msg: Some(format!(
                    "Error sending Delete Election Event task: ${error}"
                )),
                task_execution: task_execution.clone(),
            }));
        }
    };

    Ok(Json(DeleteElectionEventOutput {
        id: input.election_event_id,
        error_msg: None,
        task_execution,
    }))
}
