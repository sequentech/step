// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::types::permissions::Permissions;
use sequent_core::{services::jwt, types::hasura::core::TasksExecution};
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use windmill::{
    services::providers::transactions_provider::provide_hasura_transaction,
    tasks::{
        import_templates::{import_templates, import_templates_task},
        upsert_areas::upsert_areas_task,
    },
};

use windmill::services::celery_app::get_celery_app;
use windmill::services::tasks_execution::*;
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportTemplatesInput {
    tenant_id: String,
    document_id: String,
    sha256: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportTemplatesOutput {
    error_msg: Option<String>,
    document_id: String,
    task_execution: TasksExecution,
}

#[instrument(skip(claims))]
#[post("/import-templates", format = "json", data = "<input>")]
pub async fn import_templates_route(
    claims: jwt::JwtClaims,
    input: Json<ImportTemplatesInput>,
) -> Result<Json<ImportTemplatesOutput>, (Status, String)> {
    let body = input.into_inner();
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TEMPLATE_WRITE],
    )?;

    let celery_app = get_celery_app().await;

    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    let tenant_id = body.tenant_id.clone();
    let document_id = body.document_id.clone();

    // Insert the task execution record
    let task_execution = post(
        &tenant_id,
        None,
        ETasksExecution::IMPORT_TEMPLATES,
        &executer_name,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {error:?}"),
        )
    })?;

    let celery_task_results = celery_app
        .send_task(import_templates_task::new(
            tenant_id,
            document_id,
            body.sha256.clone(),
            task_execution.clone(),
        ))
        .await;

    let error_msg = match celery_task_results {
        Ok(_) => None,
        Err(err) => Some(err.to_string()),
    };

    Ok(Json(ImportTemplatesOutput {
        error_msg,
        document_id: body.document_id,
        task_execution,
    }))
}
