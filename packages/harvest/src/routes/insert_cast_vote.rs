// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::cast_votes::CastVoter;
use crate::services::authorization::authorize_voter_election;
use crate::services::dependencies::HarvestServices;
use crate::services::vitrine::CastReceipts;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::services::connection::UserLocation;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::VoterPermissions;
use sequent_core::util::retry::retry_with_exponential_backoff;
use serde::Serialize;
use std::time::Duration;
use std::time::Instant;
use tracing::{error, info, instrument};
use windmill::services::insert_cast_vote::{
    CastVoteError, InsertCastVoteInput, InsertCastVoteOutput,
    InsertCastVoteResult,
};
use windmill::tasks::process_cast_vote;
use windmill::types::vitrine::VitrineCast;

const ROUTE_PHASE_COMPLETED: &str = "cast-vote route phase completed";

/// The cast vote, and what the event's Vitrine `ballots` log said about its
/// entry, or `None` when the event's transparency is off.
#[derive(Debug, Serialize)]
pub struct InsertCastVoteResponse {
    #[serde(flatten)]
    pub cast_vote: InsertCastVoteOutput,
    pub vitrine: Option<VitrineCast>,
}

/// API endpoint for inserting votes. POST coming from the
/// frontend->Hasura->Harvest->Here.
///
/// It tries to insert the vote into the database and returns the Json result in
/// case of success or logs the information of the error (coming from a
/// synchronous windmill library function, `try_insert_cast_vote`) before
/// returning the error.
///
/// Once the vote is in, it waits a bounded time for the ballot's Vitrine
/// receipt, which never fails the cast.
#[instrument(skip_all)]
#[post("/insert-cast-vote", format = "json", data = "<body>")]
pub async fn insert_cast_vote(
    body: Json<InsertCastVoteInput>,
    claims: JwtClaims,
    user_info: UserLocation,
    services: &State<HarvestServices>,
    receipts: &State<CastReceipts>,
) -> Result<Json<InsertCastVoteResponse>, JsonError> {
    let start = Instant::now();
    let input: InsertCastVoteInput = body.into_inner();
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
    let auth_time = &claims.auth_time.or_else(|| {
        matches!(voting_channel, VotingStatusChannel::TELEPHONE)
            .then_some(claims.iat)
    });

    info!(
        phase = "authorization",
        duration_us = start.elapsed().as_micros() as u64,
        "{ROUTE_PHASE_COMPLETED}"
    );
    info!("insert-cast-vote: starting");

    let insert_result_wrapped = retry_with_exponential_backoff(
        // The closure we want to call repeatedly
        || async {
            services
                .cast_votes
                .try_insert(
                    input.clone(),
                    CastVoter {
                        tenant_id: &claims.hasura_claims.tenant_id,
                        voter_id: &claims.hasura_claims.user_id,
                        area_id: &area_id,
                        voting_channel,
                        auth_time,
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
        // Maximum number of retries:
        5,
        // Initial backoff:
        Duration::from_millis(100),
    )
    .await;

    // Unwrap SkipRetryFailure into a normal Result/Error
    let insert_result = match insert_result_wrapped {
        Ok(insert_cv_result) => match insert_cv_result {
            InsertCastVoteResult::Success(inserted_cast_vote, vitrine) => {
                Ok((inserted_cast_vote, None, vitrine))
            }
            InsertCastVoteResult::PendingDatafix(
                inserted_cast_vote,
                vitrine,
            ) => {
                let cast_vote_id = inserted_cast_vote.id.clone();
                Ok((inserted_cast_vote, Some(cast_vote_id), vitrine))
            }
            InsertCastVoteResult::SkipRetryFailure(cast_vote_error) => {
                Err(cast_vote_error)
            }
        },
        Err(e) => Err(e),
    };

    let (inserted_cast_vote, pending_cast_vote_id, vitrine) = insert_result
        .map_err(|cast_vote_err| {
            let duration = start.elapsed();
            info!(
                "insert-cast-vote took {} ms to complete but was refused",
                duration.as_millis()
            );

            cast_vote_error_response(cast_vote_err)
        })?;

    // If there is no error:
    let duration = start.elapsed();
    info!(
        "insert-cast-vote took {} ms to complete and succeeded.",
        duration.as_millis()
    );

    let enqueue_start = Instant::now();
    if pending_cast_vote_id.is_some() {
        enqueue_datafix_cast_vote(services, &inserted_cast_vote).await;
    }

    info!(
        phase = "enqueue",
        duration_us = enqueue_start.elapsed().as_micros() as u64,
        "{ROUTE_PHASE_COMPLETED}"
    );

    let vitrine_start = Instant::now();
    let vitrine = receipts.cast(vitrine).await;
    info!(
        phase = "vitrine",
        duration_us = vitrine_start.elapsed().as_micros() as u64,
        "{ROUTE_PHASE_COMPLETED}"
    );
    info!(
        phase = "request",
        duration_us = start.elapsed().as_micros() as u64,
        "cast-vote route completed"
    );
    Ok(Json(InsertCastVoteResponse {
        cast_vote: inserted_cast_vote,
        vitrine,
    }))
}

/// Hands a committed Datafix vote to its pipeline. An enqueue failure must not
/// fail the request: the review beat recovers in-progress rows.
pub(crate) async fn enqueue_datafix_cast_vote(
    services: &HarvestServices,
    cast_vote: &InsertCastVoteOutput,
) {
    let cast_vote_id = &cast_vote.id;
    match services
        .tasks
        .send_task(process_cast_vote::process_cast_vote::new(
            cast_vote.tenant_id.clone(),
            cast_vote.election_event_id.clone(),
            cast_vote_id.clone(),
        ))
        .await
    {
        Ok(celery_task) => {
            info!("Sent process_cast_vote task {}", celery_task.task_id);
        }
        Err(e) => {
            error!(
                "Error sending process_cast_vote task for cast vote {cast_vote_id}: {e:?}; the review_cast_votes beat will retry it"
            );
        }
    }
}

/// The answer each refusal or failure of the ballot box maps to. A ballot the
/// ballot box refuses is the voter's request to correct, not a server error.
pub(crate) fn cast_vote_error_response(
    cast_vote_err: CastVoteError,
) -> JsonError {
    match cast_vote_err {
        CastVoteError::AreaNotFound => ErrorResponse::new(
            Status::NotFound,
            "Area not found",
            ErrorCode::AreaNotFound,
        ),
        CastVoteError::ElectionEventNotFound(_) => ErrorResponse::new(
            Status::NotFound,
            "Election Event Not Found",
            ErrorCode::ElectionEventNotFound,
        ),
        CastVoteError::InvalidDatafixConfiguration(_) => ErrorResponse::new(
            Status::InternalServerError,
            "Invalid Datafix election event configuration",
            ErrorCode::InternalServerError,
        ),
        CastVoteError::ElectoralLogNotFound(_) => ErrorResponse::new(
            Status::NotFound,
            "Electoral Log Not Found",
            ErrorCode::ElectoralLogNotFound,
        ),
        CastVoteError::CheckStatusFailed(msg) => ErrorResponse::new(
            Status::Unauthorized,
            &msg,
            ErrorCode::CheckStatusFailed,
        ),
        CastVoteError::VotingChannelNotEnabled(_) => ErrorResponse::new(
            Status::Unauthorized,
            ErrorCode::CheckStatusFailed.to_string().as_str(),
            ErrorCode::CheckStatusFailed,
        ),
        CastVoteError::CheckStatusInternalFailed(_) => ErrorResponse::new(
            Status::InternalServerError,
            ErrorCode::InternalServerError.to_string().as_str(),
            ErrorCode::InternalServerError,
        ),
        CastVoteError::VoterStateLocked(_) => ErrorResponse::new(
            Status::Conflict,
            "The voter state is being updated; retry the vote",
            ErrorCode::CheckStatusFailed,
        ),
        CastVoteError::CheckPreviousVotesFailed(msg) => ErrorResponse::new(
            Status::BadRequest,
            &msg,
            ErrorCode::CheckPreviousVotesFailed,
        ),
        CastVoteError::CheckRevotesFailed(msg) => ErrorResponse::new(
            Status::BadRequest,
            &msg,
            ErrorCode::CheckRevotesFailed,
        ),
        CastVoteError::CheckVotesInOtherAreasFailed(msg) => ErrorResponse::new(
            Status::BadRequest,
            &msg,
            ErrorCode::CheckVotesInOtherAreasFailed,
        ),
        CastVoteError::InsertFailedExceedsAllowedRevotes => ErrorResponse::new(
            Status::BadRequest,
            ErrorCode::InsertFailedExceedsAllowedRevotes
                .to_string()
                .as_str(),
            ErrorCode::InsertFailedExceedsAllowedRevotes,
        ),
        CastVoteError::InsertFailed(_) => ErrorResponse::new(
            Status::InternalServerError,
            ErrorCode::InternalServerError.to_string().as_str(),
            ErrorCode::InternalServerError,
        ),
        CastVoteError::CommitFailed(_) => ErrorResponse::new(
            Status::InternalServerError,
            ErrorCode::InternalServerError.to_string().as_str(),
            ErrorCode::InternalServerError,
        ),
        CastVoteError::GetDbClientFailed(_) => ErrorResponse::new(
            Status::InternalServerError,
            ErrorCode::InternalServerError.to_string().as_str(),
            ErrorCode::InternalServerError,
        ),
        CastVoteError::GetClientCredentialsFailed(_) => ErrorResponse::new(
            Status::Unauthorized,
            ErrorCode::GetClientCredentialsFailed.to_string().as_str(),
            ErrorCode::GetClientCredentialsFailed,
        ),
        CastVoteError::GetAreaIdFailed(_) => ErrorResponse::new(
            Status::BadRequest,
            ErrorCode::GetAreaIdFailed.to_string().as_str(),
            ErrorCode::GetAreaIdFailed,
        ),
        CastVoteError::GetTransactionFailed(_) => ErrorResponse::new(
            Status::InternalServerError,
            ErrorCode::InternalServerError.to_string().as_str(),
            ErrorCode::GetTransactionFailed,
        ),
        CastVoteError::DeserializeBallotFailed(_) => ErrorResponse::new(
            Status::BadRequest,
            ErrorCode::DeserializeBallotFailed.to_string().as_str(),
            ErrorCode::DeserializeBallotFailed,
        ),
        CastVoteError::DeserializeContestsFailed(_) => ErrorResponse::new(
            Status::BadRequest,
            ErrorCode::DeserializeContestsFailed.to_string().as_str(),
            ErrorCode::DeserializeContestsFailed,
        ),
        CastVoteError::DeserializeAreaPresentationFailed(_) => {
            ErrorResponse::new(
                Status::BadRequest,
                ErrorCode::DeserializeAreaPresentationFailed
                    .to_string()
                    .as_str(),
                ErrorCode::DeserializeAreaPresentationFailed,
            )
        }
        CastVoteError::SerializeVoterIdFailed(_) => ErrorResponse::new(
            Status::InternalServerError,
            ErrorCode::InternalServerError.to_string().as_str(),
            ErrorCode::InternalServerError,
        ),
        CastVoteError::SerializeBallotFailed(_) => ErrorResponse::new(
            Status::InternalServerError,
            ErrorCode::InternalServerError.to_string().as_str(),
            ErrorCode::InternalServerError,
        ),
        CastVoteError::PokValidationFailed(_) => ErrorResponse::new(
            Status::BadRequest,
            ErrorCode::PokValidationFailed.to_string().as_str(),
            ErrorCode::PokValidationFailed,
        ),
        CastVoteError::BallotSignFailed(_) => ErrorResponse::new(
            Status::InternalServerError,
            ErrorCode::InternalServerError.to_string().as_str(),
            ErrorCode::InternalServerError,
        ),
        CastVoteError::BallotVoterSignatureFailed(_)
        | CastVoteError::BallotVoterSignatureRequired
        | CastVoteError::BallotStyleMismatch(_)
        | CastVoteError::BallotNotReceived
        | CastVoteError::BallotCastSignatureRequired
        | CastVoteError::BallotCastSignatureFailed(_)
        | CastVoteError::BallotAlreadyCast
        | CastVoteError::BallotAudited => ErrorResponse::new(
            Status::BadRequest,
            ErrorCode::PokValidationFailed.to_string().as_str(),
            ErrorCode::PokValidationFailed,
        ),
        CastVoteError::UuidParseFailed(_, _) => ErrorResponse::new(
            Status::BadRequest,
            ErrorCode::UuidParseFailed.to_string().as_str(),
            ErrorCode::UuidParseFailed,
        ),
        CastVoteError::UnknownError(_) => ErrorResponse::new(
            Status::InternalServerError,
            ErrorCode::UnknownError.to_string().as_str(),
            ErrorCode::UnknownError,
        ),
        CastVoteError::BallotIdMismatch(msg) => ErrorResponse::new(
            Status::BadRequest,
            &msg,
            ErrorCode::BallotIdMismatch,
        ),
        CastVoteError::GetBallotStyleFailed(_) => ErrorResponse::new(
            Status::InternalServerError,
            ErrorCode::InternalServerError.to_string().as_str(),
            ErrorCode::InternalServerError,
        ),
    }
}

#[cfg(test)]
#[path = "../../tests/support/cast_vote_routes.rs"]
mod route_tests;
