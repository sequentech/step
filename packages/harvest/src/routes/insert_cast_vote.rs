// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize_voter_election;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::services::connection::UserLocation;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::VoterPermissions;
use sequent_core::util::retry::retry_with_exponential_backoff;
use std::time::Duration;
use std::time::Instant;
use tracing::{error, info, instrument};
use windmill::services::celery_app::get_celery_app;
use windmill::services::insert_cast_vote::{
    try_insert_cast_vote, CastVoteError, InsertCastVoteInput,
    InsertCastVoteOutput, InsertCastVoteResult,
};
use windmill::tasks::process_cast_vote;

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
    let auth_time = &claims
        .auth_time
        .or_else(|| auth_time_iat_fallback_allowed(voting_channel).then_some(claims.iat));

    info!("insert-cast-vote: starting");

    let insert_result_wrapped = retry_with_exponential_backoff(
        // The closure we want to call repeatedly
        || async {
            try_insert_cast_vote(
                input.clone(),
                &claims.hasura_claims.tenant_id,
                &claims.hasura_claims.user_id,
                &area_id,
                voting_channel,
                auth_time,
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
                Ok((inserted_cast_vote, None))
            }
            InsertCastVoteResult::PendingDatafix(inserted_cast_vote) => {
                let cast_vote_id = inserted_cast_vote.id.clone();
                Ok((inserted_cast_vote, Some(cast_vote_id)))
            }
            InsertCastVoteResult::SkipRetryFailure(cast_vote_error) => {
                Err(cast_vote_error)
            }
        },
        Err(e) => Err(e),
    };

    let (inserted_cast_vote, pending_cast_vote_id) = insert_result
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
            CastVoteError::InvalidDatafixConfiguration(_) => ErrorResponse::new(
                Status::InternalServerError,
                "Invalid Datafix election event configuration",
                ErrorCode::InternalServerError,
            ),
            CastVoteError::ElectoralLogNotFound(_) => {
                ErrorResponse::new(
                    Status::NotFound,
                    "Electoral Log Not Found",
                    ErrorCode::ElectoralLogNotFound,
                )
            }
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
            CastVoteError::CheckPreviousVotesFailed(msg) => {
                ErrorResponse::new(
                    Status::BadRequest,
                    &msg,
                    ErrorCode::CheckPreviousVotesFailed,
                )
            }
            CastVoteError::CheckRevotesFailed(msg) => {
                ErrorResponse::new(
                    Status::BadRequest,
                    &msg,
                    ErrorCode::CheckRevotesFailed,
                )
            }
            CastVoteError::CheckVotesInOtherAreasFailed(msg) => {
                ErrorResponse::new(
                    Status::BadRequest,
                    &msg,
                    ErrorCode::CheckVotesInOtherAreasFailed,
                )
            }
            CastVoteError::InsertFailedExceedsAllowedRevotes => ErrorResponse::new(
                Status::BadRequest,
                ErrorCode::InsertFailedExceedsAllowedRevotes.to_string().as_str(),
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
            CastVoteError::DeserializeAreaPresentationFailed(_) => {
                ErrorResponse::new(
                    Status::BadRequest,
                    ErrorCode::DeserializeAreaPresentationFailed.to_string().as_str(),
                    ErrorCode::DeserializeAreaPresentationFailed,
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
            CastVoteError::BallotIdMismatch(msg) => ErrorResponse::new(
                Status::BadRequest,
                &msg,
                ErrorCode::BallotIdMismatch,
            ),
        }
    })?;

    // If there is no error:
    let duration = start.elapsed();
    info!(
        "insert-cast-vote took {} ms to complete and succeeded.",
        duration.as_millis()
    );

    if let Some(cast_vote_id) = pending_cast_vote_id {
        // The Datafix vote is already committed: an enqueue failure must not
        // fail the request. The review beat recovers in-progress rows.
        let celery_app = get_celery_app().await;
        match celery_app
            .send_task(process_cast_vote::process_cast_vote::new(
                inserted_cast_vote.tenant_id.clone(),
                inserted_cast_vote.election_event_id.clone(),
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

    Ok(Json(inserted_cast_vote))
}

/// Whether a missing `auth_time` claim may fall back to `iat` for the
/// given voting channel.
///
/// Always allowed for `TELEPHONE`: that channel already has no
/// browser-derived `auth_time` to rely on. `ONLINE` real voters always go
/// through a browser and get a genuine `auth_time` for free — Keycloak
/// only ever sets the `AUTH_TIME` session note on the `authorization_code`
/// flow, never on `grant_type=password` — so requiring it there doubles
/// as an implicit "this came from a browser" check. This fallback is an
/// explicit, env-gated escape hatch from that for non-browser tooling
/// (e.g. `headless-load-test`'s password-grant voter login,
/// `packages/headless-load-test/src/vote/cast.rs`) against environments
/// that opt in via `HARVEST_ALLOW_ONLINE_AUTH_TIME_IAT_FALLBACK=true` —
/// unset (the default) preserves the strict, browser-only requirement, and
/// no real deployment's config sets it.
fn auth_time_iat_fallback_allowed(voting_channel: VotingStatusChannel) -> bool {
    match voting_channel {
        VotingStatusChannel::TELEPHONE => true,
        VotingStatusChannel::ONLINE => {
            std::env::var("HARVEST_ALLOW_ONLINE_AUTH_TIME_IAT_FALLBACK")
                .map(|value| value == "true")
                .unwrap_or(false)
        }
        VotingStatusChannel::KIOSK | VotingStatusChannel::EARLY_VOTING => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENV_VAR: &str = "HARVEST_ALLOW_ONLINE_AUTH_TIME_IAT_FALLBACK";

    // One test, run sequentially, so concurrent `cargo test` threads don't
    // race on this shared process-global env var.
    #[test]
    fn auth_time_iat_fallback_is_gated_per_channel_and_env_var() {
        std::env::remove_var(ENV_VAR);
        assert!(auth_time_iat_fallback_allowed(VotingStatusChannel::TELEPHONE));
        assert!(!auth_time_iat_fallback_allowed(VotingStatusChannel::ONLINE));
        assert!(!auth_time_iat_fallback_allowed(VotingStatusChannel::KIOSK));
        assert!(!auth_time_iat_fallback_allowed(
            VotingStatusChannel::EARLY_VOTING
        ));

        std::env::set_var(ENV_VAR, "true");
        assert!(auth_time_iat_fallback_allowed(VotingStatusChannel::ONLINE));
        assert!(auth_time_iat_fallback_allowed(VotingStatusChannel::TELEPHONE));

        std::env::set_var(ENV_VAR, "false");
        assert!(!auth_time_iat_fallback_allowed(VotingStatusChannel::ONLINE));

        std::env::remove_var(ENV_VAR);
    }
}
