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
use tracing::{event, instrument, Level};
use windmill::services::insert_cast_vote::*;

#[instrument(skip_all)]
#[post("/insert-cast-vote", format = "json", data = "<body>")]
pub async fn insert_cast_vote(
    body: Json<InsertCastVoteInput>,
    claims: JwtClaims,
) -> Result<Json<InsertCastVoteOutput>, (Status, String)> {
    let start = Instant::now();
    let area_id = authorize_voter(&claims, vec![VoterPermissions::CAST_VOTE])?;
    let input = body.into_inner();

    let result = try_insert_cast_vote(
        input,
        &claims.hasura_claims.tenant_id,
        &claims.hasura_claims.user_id,
        &area_id,
        &claims.auth_time,
    )
    .await
    .map_err(|e| {
        let duration = start.elapsed();
        event!(
            Level::INFO,
            "insert-cast-vote took {} ms to complete but failed",
            duration.as_millis()
        );
        (
            Status::InternalServerError,
            format!("Error inserting vote: {:?}", e),
        )
    })?;
    let duration = start.elapsed();
    event!(
        Level::INFO,
        "insert-cast-vote took {} ms to complete",
        duration.as_millis()
    );
    Ok(Json(result))
}
