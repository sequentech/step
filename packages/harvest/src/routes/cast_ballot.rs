// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::cast_votes::CastVoter;
use crate::routes::insert_cast_vote::{
    cast_vote_error_response, enqueue_datafix_cast_vote,
};
use crate::services::authorization::authorize_voter_election;
use crate::services::dependencies::HarvestServices;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::connection::UserLocation;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::VoterPermissions;
use sequent_core::util::retry::retry_with_exponential_backoff;
use std::time::{Duration, Instant};
use tracing::{info, instrument};
use windmill::services::cast_ballot::{
    CastBallotInput, CastBallotOutput, CastBallotResult,
};

const MAX_RETRIES: usize = 5;
const INITIAL_BACKOFF: Duration = Duration::from_millis(100);

/// The voter casts a ballot the ballot box received at review, by signing its
/// Ballot ID. POST coming from the frontend->Hasura->Harvest->Here.
///
/// It answers with the ballot box's signed cast receipt only once the cast is
/// stored. Casting again with the same signature answers with the same
/// receipt.
#[instrument(skip_all)]
#[post("/cast-ballot", format = "json", data = "<body>")]
pub async fn cast_ballot(
    body: Json<CastBallotInput>,
    claims: JwtClaims,
    user_info: UserLocation,
    services: &State<HarvestServices>,
) -> Result<Json<CastBallotOutput>, JsonError> {
    let start = Instant::now();
    let input: CastBallotInput = body.into_inner();
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

    let cast_result = retry_with_exponential_backoff(
        || async {
            services
                .cast_votes
                .try_cast(
                    input.clone(),
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
        },
        MAX_RETRIES,
        INITIAL_BACKOFF,
    )
    .await;

    let receipt = match cast_result {
        Ok(CastBallotResult::Cast(receipt)) => receipt,
        Ok(CastBallotResult::PendingDatafix(receipt, cast_vote)) => {
            enqueue_datafix_cast_vote(services, &cast_vote).await;
            receipt
        }
        Ok(CastBallotResult::SkipRetryFailure(cast_err)) | Err(cast_err) => {
            info!(
                "cast-ballot took {} ms to complete but failed with error={cast_err:?}",
                start.elapsed().as_millis()
            );
            return Err(cast_vote_error_response(cast_err));
        }
    };

    info!(
        phase = "request",
        duration_us = start.elapsed().as_micros() as u64,
        "cast-ballot route completed"
    );
    Ok(Json(receipt))
}

#[cfg(test)]
#[path = "../../tests/support/cast_ballot_routes.rs"]
mod route_tests;
