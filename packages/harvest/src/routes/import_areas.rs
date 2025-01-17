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
use windmill::{
    services::providers::transactions_provider::provide_hasura_transaction,
    tasks::{import_areas::import_areas_task, upsert_areas::upsert_areas_task},
};

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportAreasInput {
    election_event_id: String,
    document_id: String,
    sha256: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportAreasOutput {}

#[instrument(skip(claims))]
#[post("/upsert-areas", format = "json", data = "<input>")]
pub async fn upsert_areas_route(
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

    Ok(Json(ImportAreasOutput {}))
}

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

    provide_hasura_transaction(|hasura_transaction| {
        let tenant_id = claims.hasura_claims.tenant_id.clone();
        let election_event_id = body.election_event_id.clone();
        let document_id = body.document_id.clone();
        Box::pin(async move {
            // Your async code here
            import_areas_task(
                hasura_transaction,
                tenant_id,
                election_event_id,
                document_id,
                body.sha256,
            )
            .await
        })
    })
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Error importing areas: {error:?}"),
        )
    })?;

    Ok(Json(ImportAreasOutput {}))
}
