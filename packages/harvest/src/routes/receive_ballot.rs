// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::cast_votes::CastVoter;
use crate::routes::insert_cast_vote::cast_vote_error_response;
use crate::services::authorization::authorize_voter_election;
use crate::services::dependencies::HarvestServices;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::connection::UserLocation;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::VoterPermissions;
use std::time::Instant;
use tracing::{info, instrument};
use windmill::services::receive_ballot::{
    ReceiveBallotInput, ReceiveBallotOutput,
};

/// The ballot box receives a voter's ballot at review. POST coming from the
/// frontend->Hasura->Harvest->Here.
///
/// It answers with the ballot box's signed receipt only once the ballot is
/// stored, so the Ballot ID the voter sees means the ballot was received.
/// Receiving the same ballot again answers with the same receipt.
#[instrument(skip_all)]
#[post("/receive-ballot", format = "json", data = "<body>")]
pub async fn receive_ballot(
    body: Json<ReceiveBallotInput>,
    claims: JwtClaims,
    user_info: UserLocation,
    services: &State<HarvestServices>,
) -> Result<Json<ReceiveBallotOutput>, JsonError> {
    let start = Instant::now();
    let input: ReceiveBallotInput = body.into_inner();
    let election_id = input.election_id.to_string();

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
    })?;

    let received = services
        .cast_votes
        .try_receive(
            input,
            CastVoter {
                tenant_id: &claims.hasura_claims.tenant_id,
                voter_id: &claims.hasura_claims.user_id,
                area_id: &area_id,
                voting_channel,
                auth_time: &claims.auth_time,
                voter_ip: &user_info.ip.map(|ip| ip.to_string()),
                voter_country: &user_info
                    .country_code
                    .clone()
                    .map(|country_code| country_code.to_string()),
                username: &claims.preferred_username,
            },
        )
        .await
        .map_err(|receive_err| {
            info!(
                "receive-ballot took {} ms to complete but failed with error={receive_err:?}",
                start.elapsed().as_millis()
            );
            cast_vote_error_response(receive_err)
        })?;

    info!(
        phase = "request",
        duration_us = start.elapsed().as_micros() as u64,
        "receive-ballot route completed"
    );
    Ok(Json(received))
}

#[cfg(test)]
#[path = "../../tests/support/receive_ballot_routes.rs"]
mod route_tests;
