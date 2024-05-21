// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use sequent_core::types::permissions::VoterPermissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use windmill::tasks::import_areas::import_areas_task;

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportAreasInput {
    election_event_id: String,
    document_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportAreasOutput {}

#[instrument(skip(claims))]
#[post("/import-areas", format = "json", data = "<input>")]
pub async fn import_areas_route(
    claims: jwt::JwtClaims,
    input: Json<ImportAreasInput>,
) -> Result<Json<ImportAreasOutput>, (Status, String)> {
    let body = input.into_inner();
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::AREA_WRITE],
    )?;
    import_areas_task(
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

    Ok(Json(ImportAreasOutput {}))
}
