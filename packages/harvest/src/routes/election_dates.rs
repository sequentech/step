// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use anyhow::{anyhow, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::database::get_hasura_pool;
use windmill::services::{election_dates, election_event_dates};

#[derive(Deserialize, Debug)]
pub struct ManageElectionDatesBody {
    election_event_id: String,
    election_id: Option<String>,
    scheduled_date: Option<String>,
    is_start: bool, // TODO USE ENUM
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ManageElectionDatesResponse {}

#[instrument(skip(claims))]
#[post("/manage-election-dates", format = "json", data = "<body>")]
pub async fn manage_election_dates(
    body: Json<ManageElectionDatesBody>,
    claims: JwtClaims,
) -> Result<Json<ManageElectionDatesResponse>, JsonError> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::SCHEDULED_EVENT_WRITE],
    )
    .map_err(|e| {
        ErrorResponse::new(
            Status::Unauthorized,
            &format!("{e:?}"),
            ErrorCode::Unauthorized,
        )
    })?;
    let input = body.into_inner();

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|e| {
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("hasura db client failed: {e:?}"),
                ErrorCode::InternalServerError,
            )
        })?;
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("hasura transaction failed: {e:?}"),
                ErrorCode::InternalServerError,
            )
        })?;

    match input.election_id {
        Some(id) => {
            election_dates::manage_dates(
                &hasura_transaction,
                &claims.hasura_claims.tenant_id,
                &input.election_event_id,
                &id,
                input.scheduled_date.as_deref(),
                input.is_start,
            )
            .await
            .map_err(|e| {
                ErrorResponse::new(
                    Status::InternalServerError,
                    &format!("manage election dates failed:  {e:?}"),
                    ErrorCode::InternalServerError,
                )
            })?;
        }
        None => {
            election_event_dates::manage_dates(
                &hasura_transaction,
                &claims.hasura_claims.tenant_id,
                &input.election_event_id,
                input.scheduled_date.as_deref(),
                input.is_start,
            )
            .await
            .map_err(|e| {
                ErrorResponse::new(
                    Status::InternalServerError,
                    &format!("manage election event dates failed: {e:?}"),
                    ErrorCode::InternalServerError,
                )
            })?;
        }
    }

    let _commit = hasura_transaction.commit().await.map_err(|e| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("commit failed: {e:?}"),
            ErrorCode::InternalServerError,
        )
    })?;

    Ok(Json(ManageElectionDatesResponse {}))
}
