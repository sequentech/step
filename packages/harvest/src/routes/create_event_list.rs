// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use tracing::info;
use tracing::{event, instrument};
use windmill::services::database::get_hasura_pool;
use windmill::{
    postgres::scheduled_event::PostgresScheduledEvent, services::event_list::*,
    types::scheduled_event::EventProcessors,
};

#[instrument]
#[post("/get_event_list", format = "json", data = "<body>")]
pub async fn get_event_list(
    body: Json<GetEventListInput>,
    claims: JwtClaims,
) -> Result<Json<EventListOutput>, (Status, String)> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::USER_READ],
    )
    .map_err(|e| (Status::Forbidden, format!("{:?}", e)))?;

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

    let schedule_events =
        get_all_scheduled_events_from_db(&hasura_transaction, input)
            .await
            .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(schedule_events))
}

#[instrument]
#[post("/create_event", format = "json", data = "<body>")]
pub async fn create_event(
    body: Json<PostgresScheduledEvent>,
    claims: JwtClaims,
) -> Result<Json<PostgresScheduledEvent>, (Status, String)> {
    let input = body.into_inner();

    info!("Creating event {:?}", input);
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone().expect("REASON")),
        vec![Permissions::USER_WRITE],
    )
    .map_err(|e| (Status::Forbidden, format!("{:?}", e)))?;

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

    let event = create_event_in_db(&hasura_transaction, input)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    hasura_transaction.commit().await.map_err(|err| {
        (
            Status::InternalServerError,
            format!("Error committing transaction: {err}"),
        )
    })?;

    Ok(Json(event))
}
