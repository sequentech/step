// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use windmill::services::event_list::*;
use rocket::serde::json::Json;
use rocket::http::Status;
use sequent_core::services::jwt::JwtClaims;
use tracing::{info};
use sequent_core::types::permissions::Permissions;
use deadpool_postgres::Client as DbClient;
use windmill::services::database::{get_hasura_pool};
use tracing::{instrument, event};

#[instrument]
#[post("/get_event_list", format = "json", data = "<body>")]
pub async fn get_event_list(
    body: Json<GetEventListInput>,
    claims: JwtClaims,
) -> Result<Json<Vec<GetEventListOutput>>, (Status, String)> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::USER_READ],
    ).map_err(|e| (Status::Forbidden, format!("{:?}", e)))?;

    let mut hasura_db_client: DbClient =
    get_hasura_pool().await.get().await.map_err(|err| {
        (
            Status::InternalServerError,
            format!("Error loading hasura db client: {err}"),
        )
    })?;
    let hasura_transaction =
    hasura_db_client.transaction().await.map_err(|err| {
        (
            Status::InternalServerError,
            format!("Error creating a transaction: {err}"),
        )
    })?;

    let schedule_events = get_all_scheduled_events_from_db(&hasura_transaction,input).await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let output: Vec<GetEventListOutput> = schedule_events
        .into_iter()
        .filter_map(|event| GetEventListOutput::try_from(event).ok()) // Convert and filter out errors
        .collect();

    Ok(Json(output))
}
