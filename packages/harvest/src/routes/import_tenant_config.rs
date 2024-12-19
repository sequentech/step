// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{anyhow, Context, Result};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;
use windmill::services::tasks_execution::*;
use windmill::tasks::export_election_event::{self};
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportTenantConfigInput {
    tenant_id: String,
    document_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportTenantConfigOutput {
    id: Option<String>,
    message: Option<String>,
    error: Option<String>,
    task_execution: Option<TasksExecution>,
}

#[instrument(skip(claims))]
#[post("/import-tenant-config", format = "json", data = "<input>")]
pub async fn import_tenant_config_route(
    claims: jwt::JwtClaims,
    input: Json<ImportTenantConfigInput>,
) -> Result<Json<ImportTenantConfigOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![], // vec![Permissions::ELECTION_EVENT_READ],
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
        ETasksExecution::IMPORT_TENANT_CONFIG,
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
    //     .send_task(export_election_event::export_election_event::new(
    //         // TODO: create and put here the real task
    //         tenant_id,
    //         election_event_id,
    //         document_id.clone(),
    //         task_execution.clone(),
    //     ))
    //     .await
    //     .map_err(|error| {
    //         (
    //             Status::InternalServerError,
    //             format!("Error sending export_election_event task:
    // {error:?}"),         )
    //     })?;

    let output = ImportTenantConfigOutput {
        id: None, //TODO: delete id from struct and action
        message: Some(format!("Upserted Tenant Config  succ")),
        error: None,
        task_execution: Some(task_execution.clone()),
    };

    info!("Sent EXPORT_ELECTION_EVENT task  {:?}", &task_execution);

    Ok(Json(output))
}
