// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
#[derive(Serialize, Deserialize, Debug)]
pub struct SendEmlInput {
    election_event_id: String,
    tally_session_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SendEmlOutput {}

#[instrument(skip(claims))]
#[post("/send-eml", format = "json", data = "<input>")]
pub async fn send_eml(
    claims: jwt::JwtClaims,
    input: Json<SendEmlOutput>,
) -> Result<Json<SendEmlOutput>, (Status, String)> {
    let body = input.into_inner();
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TALLY_WRITE],
    )?;
    /*
    upsert_areas_task(
        claims.hasura_claims.tenant_id.clone(),
        body.election_event_id.clone(),
        body.document_id.clone(),
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Error importing areas: {error:?}"),
        )
    })?;
    */

    Ok(Json(SendEmlOutput {}))
}
