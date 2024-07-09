// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{anyhow, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::database::get_hasura_pool;
use windmill::services::election_dates;

#[derive(Deserialize, Debug)]
pub struct ManageElectionDatesBody {
    election_event_id: String,
    election_id: String,
    is_start: bool,
    is_unset: bool,
    date: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ManageElectionDatesResponse {}

#[instrument(skip(claims))]
#[post("/manage-election-dates", format = "json", data = "<body>")]
pub async fn manage_election_dates(
    body: Json<ManageElectionDatesBody>,
    claims: JwtClaims,
) -> Result<Json<ManageElectionDatesResponse>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_EVENT_WRITE],
    )?;

    let input = body.into_inner();

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    election_dates::manage_dates(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
        &input.election_id,
        input.is_start,
        input.is_unset,
        input.date.as_deref()
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed: {}", e));

    Ok(Json(ManageElectionDatesResponse {}))
}
