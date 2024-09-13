// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2024 David Ruescas <david@sequentech.io>
// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize_voter;
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

/// Gets the POST coming from the frontend->Hasura->Harvest->Here.
/// Then it tries to insert the vote into the database (windmill) and returns
/// the Json result in case of success or logs the information of the
/// error(coming from windmill) before returning the error.
#[instrument(skip_all)]
#[post("/insert-cast-vote", format = "json", data = "<body>")]
pub async fn insert_cast_vote(
    body: Json<InsertCastVoteInput>,
    claims: JwtClaims,
) -> Result<Json<InsertCastVoteOutput>, (Status, String)> {
    let start = Instant::now();
    let area_id = authorize_voter(&claims, vec![VoterPermissions::CAST_VOTE])?;
    let input = body.into_inner();

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

        match &cast_vote_err {
            CastVoteError::AreaNotFound => {
                error!("AreaNotFound by ID Error.")
            }
            CastVoteError::ElectionEventNotFound(error_str) => {
                error!("ElectionEventNotFound Error: {}", error_str)
            }
            CastVoteError::ElectoralLogNotFound(error_str) => {
                error!("ElectoralLogNotFound Error: {}", error_str)
            }
            CastVoteError::CheckStatusFailed(error_str) => {
                error!("CheckStatusFailed Error: {}", error_str)
            }
            CastVoteError::CheckPreviousVotesFailed(error_str) => {
                error!("CheckPreviousVotesFailed Error: {}", error_str)
            }
            CastVoteError::InsertFailed(error_str) => {
                error!("InsertFailed Error: {}", error_str)
            }
            CastVoteError::CommitFailed(error_str) => {
                error!("CommitFailed Error: {}", error_str)
            }
            CastVoteError::GetDbClientFailed(error_str) => {
                error!("GetDbClientFailed Error: {}", error_str)
            }
            CastVoteError::GetClientCredentialsFailed(error_str) => {
                error!("GetClientCredentialsFailed Error: {}", error_str)
            }
            CastVoteError::GetAreaIdFailed(error_str) => {
                error!("GetAreaIdFailed Error: {}", error_str)
            }
            CastVoteError::GetTransactionFailed(error_str) => {
                error!("GetTransactionFailed Error: {}", error_str)
            }
            CastVoteError::DeserializeBallotFailed(error_str) => {
                error!("Error deserializing ballot content: DeserializeBallotFailed Error: {}", error_str)
            }
            CastVoteError::DeserializeContestsFailed(error_str) => {
                error!("Error deserializing ballot contests: DeserializeContestsFailed Error: {}", error_str)
            }
            CastVoteError::SerializeVoterIdFailed(error_str) => {
                error!("Error hashing voter id: SerializeVoterIdFailed Error: {}", error_str)
            }
            CastVoteError::SerializeBallotFailed(error_str) => {
                error!("Error hashing ballot: SerializeBallotFailed Error: {}", error_str)
            }
            CastVoteError::PokValidationFailed(error_str) => {
                error!("PokValidationFailed Error: {}", error_str)
            }
            CastVoteError::BallotSignFailed(error_str) => {
                error!("BallotSignFailed Error: {}", error_str)
            }
            CastVoteError::UuidParseFailed(error_str1, error_str2) => {
                error!("UuidParseFailed Error: {} Error parsing:{}", error_str1, error_str2)
            }
            CastVoteError::UnknownError(error_str) => {
                error!("Unknown Error: {}", error_str)
            }
        };

        (
            Status::InternalServerError,
            format!("Error inserting vote: {:?}", cast_vote_err),
        )
    })?;

    // If there is no error:
    let duration = start.elapsed();
    info!(
        "insert-cast-vote took {} ms to complete and succeded.",
        duration.as_millis()
    );
    debug!(cast_vote = ?inserted_cast_vote, "CastVote inserted: ");
    Ok(Json(inserted_cast_vote))
}
