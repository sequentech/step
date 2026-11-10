// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::{
    authorize, authorize_election_permission_labels,
};
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use rocket::{http::Status, serde::json::Json};
use sequent_core::services::{jwt::JwtClaims, uuid_validation::parse_uuid_v4};
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::database::get_hasura_pool;
use windmill::services::election_revoting::{
    disable_revoting, RevotingRefusal, SINGLE_VOTE,
};

/// The election whose re-voting is disabled.
#[derive(Serialize, Deserialize, Debug)]
pub struct DisableElectionRevotingInput {
    /// The election event of the election.
    election_event_id: String,
    /// The election.
    election_id: String,
}

/// The election and its new number of allowed votes.
#[derive(Serialize, Deserialize, Debug)]
pub struct DisableElectionRevotingOutput {
    /// The election.
    election_id: String,
    /// The number of allowed votes after the change: one.
    num_allowed_revotes: i32,
}

/// The answer to a failure that is not a refusal; the log keeps the cause.
const FAILED: &str = "Could not disable re-voting.";

/// Logs `error` and answers with [`FAILED`].
fn internal(error: impl std::fmt::Debug) -> JsonError {
    tracing::error!(?error, "Disable re-voting request failed");
    ErrorResponse::new(
        Status::InternalServerError,
        FAILED,
        ErrorCode::InternalServerError,
    )
}

/// The authorization refusals keep their text; other failures are logged.
fn refused((status, message): (Status, String)) -> JsonError {
    if status == Status::Unauthorized || status == Status::Forbidden {
        ErrorResponse::new(status, &message, ErrorCode::Unauthorized)
    } else {
        internal(message)
    }
}

/// Lowers the number of allowed votes of an election to one: the
/// administrator's operation, in Preparation before voting starts on any
/// channel. The electoral log records who did it.
#[instrument(skip(claims))]
#[post("/disable-election-revoting", format = "json", data = "<body>")]
pub async fn disable_election_revoting(
    body: Json<DisableElectionRevotingInput>,
    claims: JwtClaims,
) -> Result<Json<DisableElectionRevotingOutput>, JsonError> {
    let input = body.into_inner();
    let tenant = &claims.hasura_claims.tenant_id;
    authorize(
        &claims,
        false,
        Some(tenant.clone()),
        vec![Permissions::ADMIN_USER],
    )
    .map_err(refused)?;
    for (name, id) in [
        ("election_event_id", &input.election_event_id),
        ("election_id", &input.election_id),
    ] {
        parse_uuid_v4(id).map_err(|_| {
            ErrorResponse::new(
                Status::BadRequest,
                &format!("{name} is not a UUID"),
                ErrorCode::UuidParseFailed,
            )
        })?;
    }
    let mut client = get_hasura_pool().await.get().await.map_err(internal)?;
    let tx = client.transaction().await.map_err(internal)?;
    authorize_election_permission_labels(
        &tx,
        &claims,
        &input.election_event_id,
        Some(std::slice::from_ref(&input.election_id)),
    )
    .await
    .map_err(refused)?;
    disable_revoting(
        &tx,
        tenant,
        &input.election_event_id,
        &input.election_id,
        &claims.hasura_claims.user_id,
        claims.preferred_username.as_deref(),
    )
    .await
    .map_err(|error| match error.downcast_ref::<RevotingRefusal>() {
        Some(refusal) => ErrorResponse::new(
            Status::BadRequest,
            &refusal.to_string(),
            ErrorCode::RevotingValidation,
        ),
        None => internal(error),
    })?;
    tx.commit().await.map_err(internal)?;
    Ok(Json(DisableElectionRevotingOutput {
        election_id: input.election_id,
        num_allowed_revotes: SINGLE_VOTE,
    }))
}
