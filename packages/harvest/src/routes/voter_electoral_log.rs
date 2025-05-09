// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize_voter_election;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use sequent_core::types::permissions::VoterPermissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::electoral_log::{
    list_cast_vote_messages, CastVoteMessagesInput, CastVoteMessagesOutput,
};
use windmill::types::resources::DataList;

#[instrument]
#[post("/immudb/cast-vote-messages", format = "json", data = "<body>")]
pub async fn cast_vote_messages(
    body: Json<CastVoteMessagesInput>,
    claims: JwtClaims,
) -> Result<Json<DataList<CastVoteMessagesOutput>>, JsonError> {
    let input = body.into_inner();
    // let election_id = input.election_id.as_deref().unwrap_or_default();
    let election_id = input.election_id.clone().unwrap_or_default(); // TODO: Temporary till merging the ballot performace inprovements.
    let (area_id, voting_channel) = authorize_voter_election(
        &claims,
        vec![VoterPermissions::CAST_VOTE],
        &election_id,
    )
    .map_err(|e| {
        ErrorResponse::new(
            Status::Unauthorized,
            &format!("{:?}", e),
            ErrorCode::Unauthorized,
        )
    })?; // TODO: Temporary till merging the ballot performace inprovements.

    let ret_val = list_cast_vote_messages(input).await.map_err(|e| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("{:?}", e),
            ErrorCode::InternalServerError,
        )
    })?;

    Ok(Json(ret_val))
}
