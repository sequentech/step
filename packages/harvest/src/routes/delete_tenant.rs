// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak::get_tenant_realm;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::celery_app::get_celery_app;
use windmill::services::tasks_execution::*;
use windmill::tasks::delete_tenant;
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteTenantOutput {
    id: String,
    task_execution: TasksExecution,
    error_msg: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteTenantInput {
    tenant_id: String,
}

/// Deletes a tenant. Only callable by the super-admin tenant (same
/// authorization model as insertTenant), and only once the target tenant has
/// no election events left — see count_tenant_election_events.
#[instrument(skip(claims))]
#[post("/delete-tenant", format = "json", data = "<body>")]
pub async fn delete_tenant_f(
    body: Json<DeleteTenantInput>,
    claims: JwtClaims,
) -> Result<Json<DeleteTenantOutput>, (Status, String)> {
    let input = body.into_inner();
    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    let task_execution = post(
        &claims.hasura_claims.tenant_id,
        None,
        ETasksExecution::DELETE_TENANT,
        &executer_name,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {error:?}"),
        )
    })?;

    if let Err(error) =
        authorize(&claims, true, None, vec![Permissions::TENANT_DELETE])
    {
        let _ = update_fail(
            &task_execution,
            &format!("Failed to authorize executing the task: {error:?}"),
        )
        .await;
        return Err(error);
    };

    let celery_app = get_celery_app().await;

    let realm = get_tenant_realm(&input.tenant_id);

    let celery_task_result = celery_app
        .send_task(delete_tenant::delete_tenant_t::new(
            input.tenant_id.clone(),
            realm,
            task_execution.clone(),
        ))
        .await;

    let _celery_task = match celery_task_result {
        Ok(task) => task,
        Err(error) => {
            return Ok(Json(DeleteTenantOutput {
                id: input.tenant_id,
                error_msg: Some(format!(
                    "Error sending Delete Tenant task: ${error}"
                )),
                task_execution: task_execution.clone(),
            }));
        }
    };

    Ok(Json(DeleteTenantOutput {
        id: input.tenant_id,
        error_msg: None,
        task_execution,
    }))
}
