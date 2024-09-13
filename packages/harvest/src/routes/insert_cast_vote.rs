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
    try_insert_cast_vote, InsertCastVoteInput, InsertCastVoteOutput,
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
    .map_err(|e| {
        let duration = start.elapsed();
        info!(
            "insert-cast-vote took {} ms to complete but failed.",
            duration.as_millis()
        );
        error!(error=?e, "Error inserting vote: ");
        (
            Status::InternalServerError,
            format!("Error inserting vote: {:?}", e),
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
