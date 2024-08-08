// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use windmill::{
    services::celery_app::get_celery_app, tasks::send_eml::send_eml_task,
};
#[derive(Serialize, Deserialize, Debug)]
pub struct SendEmlInput {
    election_event_id: String,
    tally_session_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SendEmlOutput {}

#[instrument(skip(claims))]
#[post("/send-eml", format = "json", data = "<input>")]
pub async fn send_eml(
    claims: jwt::JwtClaims,
    input: Json<SendEmlInput>,
) -> Result<Json<SendEmlOutput>, (Status, String)> {
    let body = input.into_inner();
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TALLY_WRITE],
    )?;
    let celery_app = get_celery_app().await;
    let task = celery_app
        .send_task(send_eml_task::new(
            claims.hasura_claims.tenant_id.clone(),
            body.election_event_id.clone(),
            body.tally_session_id.clone(),
        ))
        .await
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!("Error sending send_eml task: {error:?}"),
            )
        })?;
    info!("Sent send_eml task {}", task.task_id);

    Ok(Json(SendEmlOutput {}))
}
