// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::cast_votes::CastVoter;
use crate::routes::insert_cast_vote::{
    cast_vote_error_response, enqueue_datafix_cast_vote,
};
use crate::services::authorization::authorize_voter_election;
use crate::services::dependencies::HarvestServices;
use crate::services::vitrine::CastReceipts;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::connection::UserLocation;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::VoterPermissions;
use sequent_core::util::retry::retry_with_exponential_backoff;
use std::time::{Duration, Instant};
use serde::Serialize;
use tracing::{info, instrument};
use windmill::types::vitrine::VitrineCast;
use windmill::services::cast_ballot::{
    CastBallotInput, CastBallotOutput, CastBallotResult,
};

/// The ballot box's cast receipt, and what the event's Vitrine `ballots` log
/// said about the ballot's `cast` entry, or `None` when the event's
/// transparency is off.
#[derive(Debug, Serialize)]
pub struct CastBallotResponse {
    #[serde(flatten)]
    pub receipt: CastBallotOutput,
    pub vitrine: Option<VitrineCast>,
}

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
    receipts: &State<CastReceipts>,
) -> Result<Json<CastBallotResponse>, JsonError> {
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
                        user_agent: &user_info.user_agent,
                    },
                )
                .await
        },
        MAX_RETRIES,
        INITIAL_BACKOFF,
    )
    .await;

    let (receipt, vitrine) = match cast_result {
        Ok(CastBallotResult::Cast(receipt, vitrine)) => (receipt, vitrine),
        Ok(CastBallotResult::PendingDatafix(receipt, cast_vote, vitrine)) => {
            enqueue_datafix_cast_vote(services, &cast_vote).await;
            (receipt, vitrine)
        }
        Ok(CastBallotResult::SkipRetryFailure(cast_err)) | Err(cast_err) => {
            info!(
                "cast-ballot took {} ms to complete but was refused",
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
    let vitrine = receipts.cast(vitrine).await;
    Ok(Json(CastBallotResponse { receipt, vitrine }))
}

#[cfg(test)]
#[path = "../../tests/support/cast_ballot_routes.rs"]
mod route_tests;
