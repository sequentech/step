// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::{VotingStatus, VotingStatusChannel};
use sequent_core::services::jwt::{has_gold_permission, JwtClaims};
use sequent_core::types::permissions::Permissions;
use sequent_core::types::tally_sheets::VotingChannel;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::database::get_hasura_pool;
use windmill::services::{election_event_status, voting_status};

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateEventVotingStatusInput {
    pub election_event_id: String,
    pub voting_status: VotingStatus,
    pub voting_channels: Option<Vec<VotingStatusChannel>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateEventVotingStatusOutput {
    pub election_event_id: String,
}

#[instrument(skip(claims))]
#[post("/update-event-voting-status", format = "json", data = "<body>")]
pub async fn update_event_status(
    body: Json<UpdateEventVotingStatusInput>,
    claims: JwtClaims,
) -> Result<Json<UpdateEventVotingStatusOutput>, (Status, String)> {
    // Check if the user has the required "Gold" role
    if !has_gold_permission(&claims) {
        return Err((Status::Forbidden, "Insufficient privileges".into()));
    }

    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_STATE_WRITE],
    )?;

    let input = body.into_inner();
    let tenant_id = &claims.hasura_claims.tenant_id;
    let user_id = claims.hasura_claims.user_id;
    let username = claims.preferred_username;

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error getting hasura client {:?}", e),
            )
        })?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    election_event_status::update_event_voting_status(
        &hasura_transaction,
        tenant_id,
        Some(&user_id),
        username.as_deref(),
        &input.election_event_id,
        &input.voting_status,
        &input.voting_channels,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(UpdateEventVotingStatusOutput {
        election_event_id: input.election_event_id.clone(),
    }))
}

#[instrument(skip(claims))]
#[post("/update-election-voting-status", format = "json", data = "<body>")]
pub async fn update_election_status(
    body: Json<voting_status::UpdateElectionVotingStatusInput>,
    claims: JwtClaims,
) -> Result<
    Json<voting_status::UpdateElectionVotingStatusOutput>,
    (Status, String),
> {
    // Check if the user has the required "Gold" role
    if !has_gold_permission(&claims) {
        return Err((Status::Forbidden, "Insufficient privileges".into()));
    }

    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_STATE_WRITE],
    )?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let user_id = claims.hasura_claims.user_id;
    let username = claims.preferred_username;

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error getting hasura client {:?}", e),
            )
        })?;

    let hasura_transaction: deadpool_postgres::Transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    voting_status::update_election_status(
        tenant_id,
        Some(&user_id),
        username.as_deref(),
        &hasura_transaction,
        &input.election_event_id,
        &input.election_id,
        &input.voting_status,
        &input.voting_channels,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(voting_status::UpdateElectionVotingStatusOutput {
        election_id: input.election_id.clone(),
    }))
}
