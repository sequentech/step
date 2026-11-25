// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The bulletin boards of a tally session and the manager messages they were
//! created with, from which the admin portal's public verifier derives its
//! trust anchors.

use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::Deserialize;
use tracing::error;
use uuid::Uuid;
use windmill::services::tally_public_verification::{
    self, TallyVerificationBoards,
};

#[derive(Debug, Deserialize)]
pub struct TallyVerificationBoardsInput {
    election_event_id: Uuid,
    tally_session_id: Uuid,
}

/// Logged here; the caller learns only that the boards could not be read.
fn internal(error: impl std::fmt::Debug) -> (Status, String) {
    error!("error reading a tally session's boards: {error:?}");
    (
        Status::InternalServerError,
        "the tally session's boards could not be read".to_string(),
    )
}

/// The boards of a tally session of the caller's tenant, by batch, with the
/// `Configuration` and `Ballots` messages of their DKG and tally boards.
#[post("/tally-public-verification-boards", format = "json", data = "<body>")]
pub async fn tally_public_verification_boards(
    body: Json<TallyVerificationBoardsInput>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> Result<Json<TallyVerificationBoards>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TALLY_RESULTS_READ],
    )?;
    let mut hasura_db_client = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(internal)?;
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(internal)?;
    tally_public_verification::list_boards(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &body.election_event_id.to_string(),
        &body.tally_session_id.to_string(),
    )
    .await
    .map(Json)
    .map_err(internal)
}

#[cfg(test)]
#[path = "../../tests/support/tally_public_verification_routes.rs"]
mod tally_public_verification_routes;
