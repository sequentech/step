// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::types::error_response::ErrorCode;
use anyhow::anyhow;
use anyhow::{Context, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::ElectionPresentation;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use windmill::postgres::election;
use windmill::services::database::get_hasura_pool;
use windmill::services::import::import_election_event::upsert_b3_and_elog;

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateElectionInput {
    election_event_id: String,
    name: String,
    presentation: ElectionPresentation,
    description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateElectionOutput {
    id: String,
}

#[instrument(skip(claims))]
#[post("/create-election", format = "json", data = "<body>")]
pub async fn create_election(
    body: Json<CreateElectionInput>,
    claims: JwtClaims,
) -> Result<Json<CreateElectionOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_EVENT_WRITE],
    )?;

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let election = election::create_election(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &body.election_event_id,
        &body.name,
        &body.presentation,
        body.description.clone(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    upsert_b3_and_elog(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &body.election_event_id,
        &vec![election.id.clone()],
        false,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(CreateElectionOutput { id: election.id }))
}
