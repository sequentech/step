// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The bulletin boards of a tally session, the manager messages they were
//! created with, from which the admin portal's public verifier derives its
//! trust anchors, and each board's public transcript, which Harvest reads
//! from b4 and its storage so that the browser needs to reach neither.

use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use base64::{engine::general_purpose::STANDARD, Engine};
use protocol_board::{BoardHandle, BoardName};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Deserialize)]
pub struct TallyVerificationTranscriptInput {
    election_event_id: Uuid,
    tally_session_id: Uuid,
    tally_board: String,
}

/// Every message of a tally board's DKG board and of the tally board, in
/// padded base64: the public transcript `verifyTranscript` reads as
/// `{version: 1, dkg_messages, tally_messages}`.
#[derive(Debug, Serialize)]
pub struct TallyVerificationTranscript {
    dkg_messages: Vec<String>,
    tally_messages: Vec<String>,
}

/// Logged here; the caller learns only that the boards could not be read.
fn internal(error: impl std::fmt::Debug) -> (Status, String) {
    error!("error reading a tally session's boards: {error:?}");
    (
        Status::InternalServerError,
        "the tally session's boards could not be read".to_string(),
    )
}

/// Logged here; the caller learns only that b4 could not be read.
fn unreadable(error: anyhow::Error) -> (Status, String) {
    error!("error reading a board from the board service: {error:?}");
    (
        Status::BadGateway,
        "the board service could not be read".to_string(),
    )
}

/// The boards of a tally session of the caller's tenant, for a caller who
/// may read tally results.
async fn boards_of(
    claims: &JwtClaims,
    services: &HarvestServices,
    election_event_id: &Uuid,
    tally_session_id: &Uuid,
) -> Result<TallyVerificationBoards, (Status, String)> {
    authorize(
        claims,
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
        &election_event_id.to_string(),
        &tally_session_id.to_string(),
    )
    .await
    .map_err(internal)
}

/// Every message of `board`, as b4 and its storage serve it.
async fn board_messages(
    base_url: &str,
    board: &str,
) -> Result<Vec<String>, (Status, String)> {
    let name = BoardName::parse(board).map_err(internal)?;
    let messages = BoardHandle::new(base_url, name)
        .export_messages()
        .await
        .map_err(unreadable)?;
    Ok(messages
        .iter()
        .map(|message| STANDARD.encode(message))
        .collect())
}

/// The boards of a tally session of the caller's tenant, by batch, with the
/// `Configuration` and `Ballots` messages of their DKG and tally boards.
#[post("/tally-public-verification-boards", format = "json", data = "<body>")]
pub async fn tally_public_verification_boards(
    body: Json<TallyVerificationBoardsInput>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> Result<Json<TallyVerificationBoards>, (Status, String)> {
    boards_of(
        &claims,
        services,
        &body.election_event_id,
        &body.tally_session_id,
    )
    .await
    .map(Json)
}

/// The public transcript of one tally board of a tally session of the
/// caller's tenant, read from b4 here rather than in the browser.
#[post(
    "/tally-public-verification-transcript",
    format = "json",
    data = "<body>"
)]
pub async fn tally_public_verification_transcript(
    body: Json<TallyVerificationTranscriptInput>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> Result<Json<TallyVerificationTranscript>, (Status, String)> {
    let boards = boards_of(
        &claims,
        services,
        &body.election_event_id,
        &body.tally_session_id,
    )
    .await?;
    let Some(board) = boards
        .boards
        .into_iter()
        .find(|board| board.tally_board == body.tally_board)
    else {
        return Err((
            Status::NotFound,
            "not a tally board of this tally session".to_string(),
        ));
    };
    let Some(base_url) = services.b4.base_url() else {
        return Err((
            Status::BadGateway,
            "the board service is not configured".to_string(),
        ));
    };
    Ok(Json(TallyVerificationTranscript {
        dkg_messages: board_messages(base_url, &board.dkg_board).await?,
        tally_messages: board_messages(base_url, &board.tally_board).await?,
    }))
}

#[cfg(test)]
#[path = "../../tests/support/tally_public_verification_routes.rs"]
mod tally_public_verification_routes;
