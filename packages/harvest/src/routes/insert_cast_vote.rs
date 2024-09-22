// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-License-IdentifierText: 2024 David Ruescas <david@sequentech.io>
// SPDX-License-IdentifierText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize_voter;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::VoterPermissions;
use std::time::Instant;
use tracing::{debug, error, info, instrument};
use windmill::services::insert_cast_vote::{
    try_insert_cast_vote, CastVoteError, InsertCastVoteInput,
    InsertCastVoteOutput,
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
) -> Result<Json<InsertCastVoteOutput>, JsonError> {
    let start = Instant::now();
    let area_id = authorize_voter(&claims, vec![VoterPermissions::CAST_VOTE])
        .map_err(|e| {
        ErrorResponse::new(
            Status::Unauthorized,
            &format!("{:?}", e),
            ErrorCode::Unauthorized,
        )
    })?;
    let input = body.into_inner();

    info!("insert-cast-vote: starting");

    let inserted_cast_vote = try_insert_cast_vote(
        input,
        &claims.hasura_claims.tenant_id,
        &claims.hasura_claims.user_id,
        &area_id,
    )
    .await
    .map_err(|cast_vote_err| {
        let duration = start.elapsed();
        info!(
            "insert-cast-vote took {} ms to complete but failed.",
            duration.as_millis()
        );

        // Map CastVoteError to JsonError
        match cast_vote_err {
            CastVoteError::AreaNotFound => ErrorResponse::new(
                Status::NotFound,
                "Area not found",
                ErrorCode::AreaNotFound,
            ),
            CastVoteError::ElectionEventNotFound(error_str) => {
                ErrorResponse::new(
                    Status::NotFound,
                    &error_str,
                    ErrorCode::ElectionEventNotFound,
                )
            }
            CastVoteError::ElectoralLogNotFound(error_str) => {
                ErrorResponse::new(
                    Status::NotFound,
                    &error_str,
                    ErrorCode::ElectoralLogNotFound,
                )
            }
            CastVoteError::CheckStatusFailed(error_str) => ErrorResponse::new(
                Status::BadRequest,
                &error_str,
                ErrorCode::CheckStatusFailed,
            ),
            CastVoteError::CheckPreviousVotesFailed(error_str) => {
                ErrorResponse::new(
                    Status::Conflict,
                    &error_str,
                    ErrorCode::CheckPreviousVotesFailed,
                )
            }
            CastVoteError::InsertFailed(error_str) => ErrorResponse::new(
                Status::InternalServerError,
                &error_str,
                ErrorCode::InsertFailed,
            ),
            CastVoteError::CommitFailed(error_str) => ErrorResponse::new(
                Status::InternalServerError,
                &error_str,
                ErrorCode::CommitFailed,
            ),
            CastVoteError::GetDbClientFailed(error_str) => ErrorResponse::new(
                Status::InternalServerError,
                &error_str,
                ErrorCode::GetDbClientFailed,
            ),
            CastVoteError::GetClientCredentialsFailed(error_str) => {
                ErrorResponse::new(
                    Status::Unauthorized,
                    &error_str,
                    ErrorCode::GetClientCredentialsFailed,
                )
            }
            CastVoteError::GetAreaIdFailed(error_str) => ErrorResponse::new(
                Status::BadRequest,
                &error_str,
                ErrorCode::GetAreaIdFailed,
            ),
            CastVoteError::GetTransactionFailed(error_str) => {
                ErrorResponse::new(
                    Status::InternalServerError,
                    &error_str,
                    ErrorCode::GetTransactionFailed,
                )
            }
            CastVoteError::DeserializeBallotFailed(error_str) => {
                ErrorResponse::new(
                    Status::BadRequest,
                    &error_str,
                    ErrorCode::DeserializeBallotFailed,
                )
            }
            CastVoteError::DeserializeContestsFailed(error_str) => {
                ErrorResponse::new(
                    Status::BadRequest,
                    &error_str,
                    ErrorCode::DeserializeContestsFailed,
                )
            }
            CastVoteError::SerializeVoterIdFailed(error_str) => {
                ErrorResponse::new(
                    Status::InternalServerError,
                    &error_str,
                    ErrorCode::SerializeVoterIdFailed,
                )
            }
            CastVoteError::SerializeBallotFailed(error_str) => {
                ErrorResponse::new(
                    Status::InternalServerError,
                    &error_str,
                    ErrorCode::SerializeBallotFailed,
                )
            }
            CastVoteError::PokValidationFailed(error_str) => {
                ErrorResponse::new(
                    Status::BadRequest,
                    &error_str,
                    ErrorCode::PokValidationFailed,
                )
            }
            CastVoteError::BallotSignFailed(error_str) => ErrorResponse::new(
                Status::InternalServerError,
                &error_str,
                ErrorCode::BallotSignFailed,
            ),
            CastVoteError::UuidParseFailed(error_str1, error_str2) => {
                ErrorResponse::new(
                    Status::BadRequest,
                    &format!("{} Error parsing: {}", error_str1, error_str2),
                    ErrorCode::UuidParseFailed,
                )
            }
            CastVoteError::UnknownError(error_str) => ErrorResponse::new(
                Status::InternalServerError,
                &error_str,
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
