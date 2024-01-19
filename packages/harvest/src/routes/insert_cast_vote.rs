// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2024 David Ruescas <david@sequentech.io>
// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres;
use crate::services::authorization::authorize_voter;
use anyhow::{anyhow, Context, Result};
use board_messages::braid::message::Signer;
use board_messages::electoral_log::newtypes::*;
use chrono::{DateTime, Utc};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use rocket::futures::TryFutureExt;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::ballot::ElectionStatus;
use sequent_core::ballot::VotingStatus;
use sequent_core::services::connection::AuthHeaders;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak;
use sequent_core::types::permissions::VoterPermissions;
use strand::backend::ristretto::RistrettoCtx;
use strand::elgamal::Ciphertext;
use strand::hash::HashWrapper;
use strand::serialization::StrandDeserialize;
use strand::signature::StrandSignatureSk;
use strand::zkp::Schnorr;
use strand::zkp::Zkp;
use tracing::instrument;
use tracing::{event, Level};
use uuid::Uuid;
use windmill::hasura;
use windmill::services::election_event_board::get_election_event_board;
use windmill::services::electoral_log::ElectoralLog;
use windmill::services::insert_cast_vote::*;
use windmill::services::protocol_manager::get_protocol_manager;
use windmill::{
    hasura::election_event::get_election_event::GetElectionEventSequentBackendElectionEvent,
    services::database::get_hasura_pool,
};

#[instrument(skip(claims))]
#[post("/insert-cast-vote", format = "json", data = "<body>")]
pub async fn insert_cast_vote(
    body: Json<InsertCastVoteInput>,
    claims: JwtClaims,
) -> Result<Json<InsertCastVoteOutput>, (Status, String)> {
    let area_id = authorize_voter(&claims, vec![VoterPermissions::CAST_VOTE])?;
    let input = body.into_inner();

    let result = try_insert_cast_vote(
        input,
        &claims.hasura_claims.tenant_id,
        &claims.hasura_claims.user_id,
        &area_id,
    )
    .await
    .map_err(|e| {
        (
            Status::InternalServerError,
            format!("Error inserting vote: {:?}", e),
        )
    })?;
    Ok(Json(result))
}
