// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak::{
    get_realm_attributes, redacted_attributes, update_realm_attributes,
    validate_realm_attributes,
};
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, instrument};

#[derive(Serialize, Deserialize, Debug)]
pub struct GetRealmAttributesInput {
    pub election_event_id: String,
}

#[derive(Serialize)]
pub struct GetRealmAttributesOutput {
    pub attributes: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateRealmAttributesInput {
    pub election_event_id: String,
    pub attributes: HashMap<String, String>,
}

#[derive(Serialize)]
pub struct UpdateRealmAttributesOutput {
    pub updated: bool,
}

// skip_all: the attribute values may contain secrets and must not be recorded
// in the tracing span.
#[instrument(skip_all)]
#[post("/get-realm-attributes", format = "json", data = "<input>")]
pub async fn get_realm_attributes_route(
    claims: JwtClaims,
    input: Json<GetRealmAttributesInput>,
) -> Result<Json<GetRealmAttributesOutput>, (Status, String)> {
    let body = input.into_inner();

    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::KEYCLOAK_REALM_ATTRIBUTES_READ],
    )
    .map_err(|err| {
        error!("Authorization failed: {:?}", err);
        (Status::Forbidden, "Authorization failed".to_string())
    })?;

    let attributes = get_realm_attributes(
        &claims.hasura_claims.tenant_id,
        &body.election_event_id,
    )
    .await
    .map_err(|e| {
        error!("Failed to get realm attributes: {:?}", e);
        (
            Status::InternalServerError,
            "Failed to get realm attributes".to_string(),
        )
    })?;

    Ok(Json(GetRealmAttributesOutput {
        attributes: redacted_attributes(&attributes),
    }))
}

// skip_all: the attribute values may contain secrets and must not be recorded
// in the tracing span.
#[instrument(skip_all)]
#[post("/update-realm-attributes", format = "json", data = "<input>")]
pub async fn update_realm_attributes_route(
    claims: JwtClaims,
    input: Json<UpdateRealmAttributesInput>,
) -> Result<Json<UpdateRealmAttributesOutput>, (Status, String)> {
    let body = input.into_inner();

    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::KEYCLOAK_REALM_ATTRIBUTES_WRITE],
    )
    .map_err(|err| {
        error!("Authorization failed: {:?}", err);
        (Status::Forbidden, "Authorization failed".to_string())
    })?;

    validate_realm_attributes(&body.attributes)
        .map_err(|e| (Status::BadRequest, e.to_string()))?;

    update_realm_attributes(
        &claims.hasura_claims.tenant_id,
        &body.election_event_id,
        body.attributes,
    )
    .await
    .map_err(|e| {
        error!("Failed to update realm attributes: {:?}", e);
        (
            Status::InternalServerError,
            "Failed to update realm attributes".to_string(),
        )
    })?;

    Ok(Json(UpdateRealmAttributesOutput { updated: true }))
}
