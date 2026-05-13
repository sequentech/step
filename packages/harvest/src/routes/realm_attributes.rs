// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak::update_realm_attributes;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, instrument};

#[derive(Serialize, Deserialize, Debug)]
/// Request body for updating realm attributes.
pub struct UpdateRealmAttributesInput {
    /// The election event ID
    pub election_event_id: String,
    /// The attributes
    pub attributes: HashMap<String, String>,
}

#[derive(Serialize)]
/// Response for updating realm attributes.
pub struct UpdateRealmAttributesOutput {
    /// Whether the update was successful
    pub updated: bool,
}

/// Updates realm attributes endpoint.
#[instrument(skip(claims))]
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
        vec![Permissions::ELECTION_EVENT_WRITE],
    )
    .map_err(|err| {
        error!("Authorization failed: {err:?}");
        (Status::Forbidden, "Authorization failed".to_string())
    })?;

    update_realm_attributes(
        &claims.hasura_claims.tenant_id,
        &body.election_event_id,
        body.attributes,
    )
    .await
    .map_err(|e| {
        error!("Failed to update realm attributes: {e:?}");
        (Status::InternalServerError, format!("{e:?}"))
    })?;

    Ok(Json(UpdateRealmAttributesOutput { updated: true }))
}
