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
use tracing::{event, info, instrument, Level};
use windmill::services::celery_app::get_celery_app;
use windmill::services::tasks_execution::*;
use windmill::types::tasks::ETasksExecution;
use windmill::{
    services::providers::transactions_provider::provide_hasura_transaction,
    tasks::import_application,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportApplicationsInput {
    tenant_id: String,
    election_event_id: String,
    election_id: Option<String>,
    document_id: String,
    sha256: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportApplicationsOutput {
    error_msg: Option<String>,
    document_id: String,
    task_execution: Option<TasksExecution>,
}

#[instrument(skip(claims))]
#[post("/import-application", format = "json", data = "<input>")]
pub async fn import_application_route(
    claims: jwt::JwtClaims,
    input: Json<ImportApplicationsInput>,
) -> Result<Json<ImportApplicationsOutput>, (Status, String)> {
    let body = input.into_inner();
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::APPLICATION_IMPORT],
    )?;

    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let document_id = body.document_id.clone();
    let election_event_id = body.election_event_id.clone();
    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    // Insert the task execution record
    let task_execution: TasksExecution = match post(
        &tenant_id,
        Some(&election_event_id),
        ETasksExecution::IMPORT_APPLICATION,
        &executer_name,
    )
    .await
    {
        Ok(task_execution) => task_execution,
        Err(err) => {
            return Ok(Json(ImportApplicationsOutput {
                error_msg: Some(format!(
                    "Failed to insert task execution record: {err:?}"
                )),
                document_id: body.document_id,
                task_execution: None,
            }));
        }
    };

    let celery_app = get_celery_app().await;
    let _celery_task = match celery_app
        .send_task(import_application::import_applications::new(
            tenant_id,
            election_event_id,
            document_id,
            body.sha256.clone(),
            task_execution.clone(),
        ))
        .await
    {
        Ok(celery_task) => celery_task,
        Err(err) => {
            return Ok(Json(ImportApplicationsOutput {
                error_msg: Some(format!(
                    "Error sending import_applications task: ${err}"
                )),
                document_id: body.document_id,
                task_execution: Some(task_execution),
            }));
        }
    };

    Ok(Json(ImportApplicationsOutput {
        error_msg: None,
        document_id: body.document_id,
        task_execution: Some(task_execution),
    }))
}
