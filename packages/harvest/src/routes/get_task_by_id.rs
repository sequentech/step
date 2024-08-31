// // SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
// //
// // SPDX-License-Identifier: AGPL-3.0-only

// use anyhow::{anyhow, Context, Result};
// use rocket::http::Status;
// use rocket::serde::json::Json;
// use sequent_core::types::hasura::core::TasksExecution;
// use sequent_core::{services::jwt, types::permissions::Permissions};
// use serde::{Deserialize, Serialize};
// use tracing::{event, instrument, Level};
// use uuid::Uuid;
// use windmill::postgres::tasks_execution::get_task_by_id;

// use crate::services::authorization::authorize;

// #[derive(Serialize, Deserialize, Debug)]
// pub struct GetTaskByIdInput {
//     task_id: String,
// }

// #[derive(Serialize, Deserialize, Debug)]
// pub struct TaskOutput {
//     task: TasksExecution,
// }

// #[instrument(skip(claims))]
// #[post("/get-task-by-id", format = "json", data = "<input>")]
// pub async fn get_task_by_id_route(
//     claims: jwt::JwtClaims,
//     input: Json<GetTaskByIdInput>,
// ) -> Result<Json<TaskOutput>, (Status, String)> {
//     info!("------------------------------------");
//     authorize(
//         &claims,
//         true,
//         Some(claims.hasura_claims.tenant_id.clone()),
//         vec![Permissions::TASKS_READ],
//     )?;

//     let body = input.into_inner();
//     let task_id = body.task_id.clone();

//     let task = get_task_by_id(&task_id.clone()).await.map_err(|error| {
//         (
//             Status::InternalServerError,
//             format!("Error fetching task: {error:?}"),
//         )
//     })?;

//     let output = TaskOutput { task: task.clone() };

//     info!("Created Task {:?}", &task);

//     Ok(Json(output))
// }
