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
use tracing::{event, instrument, Level};
use windmill::tasks::import_candidates::import_candidates_task;

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportCandidatesInput {
    election_event_id: String,
    document_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportCandidatesOutput {}

#[instrument(skip(claims))]
#[post("/import-candidates", format = "json", data = "<input>")]
pub async fn import_candidates_route(
    claims: jwt::JwtClaims,
    input: Json<ImportCandidatesInput>,
) -> Result<Json<ImportCandidatesOutput>, (Status, String)> {
    let body = input.into_inner();
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ADMIN_USER],
    )?;
    import_candidates_task(
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

    Ok(Json(ImportCandidatesOutput {}))
}
