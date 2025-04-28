// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2024 David Ruescas <david@sequentech.io>
// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize_voter;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::connection::UserLocation;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::VoterPermissions;
use sequent_core::util::retry::retry_with_exponential_backoff;
use std::time::Duration;
use std::time::Instant;
use tracing::{debug, error, info, instrument};
use windmill::services::insert_cast_vote::{
    try_insert_cast_vote, CastVoteError, InsertCastVoteInput,
    InsertCastVoteOutput, InsertCastVoteResult,
};

/// API endpoint for inserting votes. POST coming from the
/// frontend->Hasura->Harvest->Here.
///
/// It tries to insert the vote into the database and returns the Json result in
/// case of success or logs the information of the error (coming from a
/// synchronous windmill library function, `try_insert_cast_vote`) before
/// returning the error.
#[instrument(skip_all)]
#[post("/insert-cast-vote", format = "json", data = "<body>")]
pub async fn insert_cast_vote(
    body: Json<InsertCastVoteInput>,
    claims: JwtClaims,
    user_info: UserLocation,
) -> Result<Json<InsertCastVoteOutput>, JsonError> {
    let start = Instant::now();
    let input: InsertCastVoteInput = body.into_inner();
    let election_id = input.election_id.to_string();

    let (area_id, voting_channel) = authorize_voter(
        &claims,
        vec![VoterPermissions::USER_ROLE],
        Some(election_id.clone()),
    )
    .map_err(|e| {
        ErrorResponse::new(
            Status::Unauthorized,
            &format!("{:?}", e),
            ErrorCode::Unauthorized,
        )
    })?;

    info!("insert-cast-vote: starting");

    let insert_result_wrapped = retry_with_exponential_backoff(
        // The closure we want to call repeatedly
        || async {
            try_insert_cast_vote(
                input.clone(),
                &claims.hasura_claims.tenant_id,
                &claims.hasura_claims.user_id,
                &area_id,
                &voting_channel,
                &claims.auth_time,
                &user_info.ip.map(|ip| ip.to_string()),
                &user_info
                    .country_code
                    .clone()
                    .map(|country_code| country_code.to_string()),
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
            InsertCastVoteResult::Success(inserted_cast_vote) => {
                Ok(inserted_cast_vote)
            }
            InsertCastVoteResult::SkipRetryFailure(cast_vote_error) => {
                Err(cast_vote_error)
            }
        },
        Err(e) => Err(e),
    };

    let inserted_cast_vote = insert_result
    .map_err(|cast_vote_err| {
        let duration = start.elapsed();
        info!(
            "insert-cast-vote took {} ms to complete but failed with error={cast_vote_err:?}",
            duration.as_millis()
        );

        // Map CastVoteError to JsonError
        match cast_vote_err {
            CastVoteError::AreaNotFound => ErrorResponse::new(
                Status::NotFound,
                "Area not found",
                ErrorCode::AreaNotFound,
            ),
            CastVoteError::ElectionEventNotFound(_) => {
                ErrorResponse::new(
                    Status::NotFound,
                    "Election Event Not Found",
                    ErrorCode::ElectionEventNotFound,
                )
            }
            CastVoteError::ElectoralLogNotFound(_) => {
                ErrorResponse::new(
                    Status::NotFound,
                    "Electoral Log Not Found",
                    ErrorCode::ElectoralLogNotFound,
                )
            }
            CastVoteError::CheckStatusFailed(_) => ErrorResponse::new(
                Status::Unauthorized,
                ErrorCode::CheckStatusFailed.to_string().as_str(),
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
            CastVoteError::CheckPreviousVotesFailed(_) => {
                ErrorResponse::new(
                    Status::BadRequest,
                    ErrorCode::CheckPreviousVotesFailed.to_string().as_str(),
                    ErrorCode::CheckPreviousVotesFailed,
                )
            }
            CastVoteError::InsertFailedExceedsAllowedRevotes => ErrorResponse::new(
                Status::BadRequest,
                ErrorCode::CheckPreviousVotesFailed.to_string().as_str(),
                ErrorCode::CheckPreviousVotesFailed,
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
            CastVoteError::GetClientCredentialsFailed(_) => {
                ErrorResponse::new(
                    Status::Unauthorized,
                    ErrorCode::GetClientCredentialsFailed.to_string().as_str(),
                    ErrorCode::GetClientCredentialsFailed,
                )
            }
            CastVoteError::GetAreaIdFailed(_) => ErrorResponse::new(
                Status::BadRequest,
                ErrorCode::GetAreaIdFailed.to_string().as_str(),
                ErrorCode::GetAreaIdFailed,
            ),
            CastVoteError::GetTransactionFailed(_) => {
                ErrorResponse::new(
                    Status::InternalServerError,
                    ErrorCode::InternalServerError.to_string().as_str(),
                    ErrorCode::GetTransactionFailed,
                )
            }
            CastVoteError::DeserializeBallotFailed(_) => {
                ErrorResponse::new(
                    Status::BadRequest,
                    ErrorCode::DeserializeBallotFailed.to_string().as_str(),
                    ErrorCode::DeserializeBallotFailed,
                )
            }
            CastVoteError::DeserializeContestsFailed(_) => {
                ErrorResponse::new(
                    Status::BadRequest,
                    ErrorCode::DeserializeContestsFailed.to_string().as_str(),
                    ErrorCode::DeserializeContestsFailed,
                )
            }
            CastVoteError::SerializeVoterIdFailed(_) => {
                ErrorResponse::new(
                    Status::InternalServerError,
                    ErrorCode::InternalServerError.to_string().as_str(),
                    ErrorCode::InternalServerError,
                )
            }
            CastVoteError::SerializeBallotFailed(_) => {
                ErrorResponse::new(
                    Status::InternalServerError,
                    ErrorCode::InternalServerError.to_string().as_str(),
                    ErrorCode::InternalServerError,
                )
            }
            CastVoteError::PokValidationFailed(_) => {
                ErrorResponse::new(
                    Status::BadRequest,
                    ErrorCode::PokValidationFailed.to_string().as_str(),
                    ErrorCode::PokValidationFailed,
                )
            }
            CastVoteError::BallotSignFailed(_) => ErrorResponse::new(
                Status::InternalServerError,
                ErrorCode::InternalServerError.to_string().as_str(),
                ErrorCode::InternalServerError,
            ),
            CastVoteError::BallotVoterSignatureFailed(_) => ErrorResponse::new(
                Status::InternalServerError,
                ErrorCode::InternalServerError.to_string().as_str(),
                ErrorCode::InternalServerError,
            ),
            CastVoteError::UuidParseFailed(_, _) => {
                ErrorResponse::new(
                    Status::BadRequest,
                    ErrorCode::UuidParseFailed.to_string().as_str(),
                    ErrorCode::UuidParseFailed,
                )
            }
            CastVoteError::UnknownError(_) => ErrorResponse::new(
                Status::InternalServerError,
                ErrorCode::UnknownError.to_string().as_str(),
                ErrorCode::UnknownError,
            ),
        }
    })?;

    // If there is no error:
    let duration = start.elapsed();
    info!(
        "insert-cast-vote took {} ms to complete and succeeded.",
        duration.as_millis()
    );
    debug!(cast_vote = ?inserted_cast_vote, "CastVote inserted: ");
    Ok(Json(inserted_cast_vote))
}
