// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;
use tracing::instrument;
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;
use windmill::services::tasks_execution::post;
use windmill::types::tasks::ETasksExecution;
use windmill::{
    services::plugins_manager::plugin_manager,
    tasks::plugins_tasks::execute_plugin_task,
};
#[derive(Deserialize, Debug)]
pub struct PluginsRouteInput {
    path: String,
    data: Value,
    task_execution: Option<String>,
    generate_document: Option<bool>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct PluginsRouteOutput {
    data: Value,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct PluginsRouteTaskOutput {
    document_id: Option<String>,
    task_execution: TasksExecution,
}

#[instrument(skip(claims))]
#[post("/plugin", format = "json", data = "<body>")]
pub async fn plugin_routes(
    claims: jwt::JwtClaims,
    body: Json<PluginsRouteInput>,
) -> Result<Json<PluginsRouteOutput>, (Status, String)> {
    let input = body.into_inner();

    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    let plugin_manager = plugin_manager::get_plugin_manager()
        .await
        .map_err(|e| (Status::InternalServerError, e.to_string()))?;

    let mut route_data = input.data;

    let claims_json_string: String = serde_json::to_string(&claims)
        .expect("Failed to serialize JwtClaims to string");

    route_data["claims"] = serde_json::Value::String(claims_json_string);

    let task = plugin_manager.get_route_task_handler(&input.path);
    let task_name = input.task_execution.clone();

    if let (Some(task), Some(task_name)) = (task, task_name) {
        let election_event_id = route_data
            .get("election_event_id")
            .map(|s| s.as_str().unwrap_or_default());

        let task_execution = post(
            &claims.hasura_claims.tenant_id,
            election_event_id,
            ETasksExecution::from_str(&task_name)
                .map_err(|e| (Status::InternalServerError, e.to_string()))?,
            &executer_name,
        )
        .await
        .map_err(|e| (Status::InternalServerError, e.to_string()))?;

        route_data["task_execution"] = serde_json::Value::String(
            serde_json::to_string(&task_execution)
                .expect("Failed to serialize task_execution to string"),
        );

        let document_id = match input.generate_document {
            Some(true) => {
                let doc_id = Uuid::new_v4().to_string();
                route_data["document_id"] =
                    serde_json::Value::String(doc_id.clone());
                Some(doc_id)
            }
            _ => None,
        };

        let celery_app = get_celery_app().await;
        let _task = celery_app
        .send_task(execute_plugin_task::new(
            task,
            route_data,
            task_execution.clone(),
            document_id.clone()
        ))
        .await
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!("Error sending send_transmission_package_task task: {error:?}"),
            )
        })?;

        let res = PluginsRouteTaskOutput {
            document_id: document_id.clone(),
            task_execution: task_execution.clone(),
        };

        let res_json = serde_json::to_string(&res)
            .map_err(|e| (Status::InternalServerError, e.to_string()))?;

        Ok(Json(PluginsRouteOutput {
            data: serde_json::Value::String(res_json),
        }))
    } else {
        let res = plugin_manager
            .call_route(&input.path, route_data.to_string())
            .await
            .map_err(|e| (Status::InternalServerError, e.to_string()))?;

        Ok(Json(PluginsRouteOutput { data: res }))
    }
}
