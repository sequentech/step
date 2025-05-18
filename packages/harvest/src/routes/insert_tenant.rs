// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;
use windmill::services::tasks_execution::*;
use windmill::tasks;
use windmill::types::tasks::ETasksExecution;
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateTenantInput {
    slug: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateTenantOutput {
    id: String,
    slug: String,
    task_execution: TasksExecution,
}

#[instrument(skip(claims))]
#[post("/insert-tenant", format = "json", data = "<body>")]
pub async fn insert_tenant(
    body: Json<CreateTenantInput>,
    claims: JwtClaims,
) -> Result<Json<CreateTenantOutput>, (Status, String)> {
    authorize(&claims, true, None, vec![Permissions::TENANT_CREATE])?;

    let celery_app = get_celery_app().await;

    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    // always set an id;
    let id = Uuid::new_v4().to_string();

    let tenant_id = claims.hasura_claims.tenant_id.clone();

    // Insert the task execution record
    let task_execution = post(
        &tenant_id,
        None,
        ETasksExecution::CREATE_TENANT,
        &executer_name,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {error:?}"),
        )
    })?;

    let celery_task_result = celery_app
        .send_task(tasks::insert_tenant::insert_tenant::new(
            id.clone(),
            body.slug.clone(),
        ))
        .await;

    let _celery_task = match celery_task_result {
        Ok(task) => task,
        Err(error) => {
            let _ = update_fail(
                &task_execution,
                &format!("Error sending insert_tenant task: {error:?}"),
            )
            .await;
            return Err((
                Status::InternalServerError,
                format!("Error sending insert_tenant task: {error:?}"),
            ));
        }
    };

    info!("Sent CREATE_TENANT task {task_execution:?}");
    let _res = update_complete(&task_execution, None).await;

    Ok(Json(CreateTenantOutput {
        id,
        slug: body.slug.clone(),
        task_execution: task_execution.clone(),
    }))
}
