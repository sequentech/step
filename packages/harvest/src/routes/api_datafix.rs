// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::connection::DatafixClaims;

use sequent_core::types::permissions::Permissions;
use serde::Deserialize;
use serde::Serialize;
use tracing::{error, info, instrument};
use windmill::services::api_datafix::*;
use windmill::services::database::get_hasura_pool;

#[instrument(skip(claims))]
#[post("/add-voter", format = "json", data = "<body>")]
pub async fn add_voter(
    claims: DatafixClaims,
    body: Json<VoterInformationBody>,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let input: VoterInformationBody = body.into_inner();

    info!("Delete voter: {input:?}");

    let required_perm = vec![Permissions::DATAFIX_ACCOUNT];
    info!("{claims:?}");
    authorize(
        &claims.jwt_claims,
        true,
        Some(claims.tenant_id.clone()),
        required_perm,
    )
    .map_err(|e| {
        error!("Error authorizing {e:?}");
        DatafixResponse::new(Status::Unauthorized)
    })?;

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|e| {
            error!("Error getting hasura client {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            error!("Error starting hasura transaction {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;

    windmill::services::api_datafix::add_datafix_voter(
        &hasura_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
        &input,
    )
    .await
}

#[derive(Deserialize, Debug)]
pub struct VoterIdBody {
    voter_id: String,
}

#[instrument(skip(claims))]
#[post("/delete-voter", format = "json", data = "<body>")]
pub async fn delete_voter(
    claims: DatafixClaims,
    body: Json<VoterIdBody>,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let input: VoterIdBody = body.into_inner();

    info!("Delete voter: {input:?}");

    let required_perm = vec![Permissions::DATAFIX_ACCOUNT];
    info!("{claims:?}");
    authorize(
        &claims.jwt_claims,
        true,
        Some(claims.tenant_id.clone()),
        required_perm,
    )
    .map_err(|e| {
        error!("Error authorizing {e:?}");
        DatafixResponse::new(Status::Unauthorized)
    })?;

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|e| {
            error!("Error getting hasura client {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            error!("Error starting hasura transaction {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;

    windmill::services::api_datafix::disable_datafix_voter(
        &hasura_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
        &input.voter_id,
    )
    .await
}

#[instrument(skip(claims))]
#[post("/unmark-voted", format = "json", data = "<body>")]
pub async fn unmark_voted(
    claims: DatafixClaims,
    body: Json<VoterIdBody>,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    // WIP
    let status = Status::Unauthorized;
    Err(DatafixResponse::new(status))
}

#[derive(Deserialize, Debug)]
pub struct MarkVotedBody {
    voter_id: String,
    channel: String,
}

#[instrument(skip(claims))]
#[post("/mark-voted", format = "json", data = "<body>")]
pub async fn mark_voted(
    claims: DatafixClaims,
    body: Json<MarkVotedBody>,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    // WIP
    let status = Status::Unauthorized;
    Err(DatafixResponse::new(status))
}

#[derive(Serialize, Debug)]
pub struct ReplacePinOutput {
    pin: String,
}

#[instrument]
#[post("/replace-pin", format = "json", data = "<body>")]
pub async fn replace_pin(
    claims: DatafixClaims,
    body: Json<VoterIdBody>,
) -> Result<Json<ReplacePinOutput>, JsonErrorResponse> {
    // WIP
    let pin = "684400123987".to_string();
    Ok(Json(ReplacePinOutput { pin }))
}
