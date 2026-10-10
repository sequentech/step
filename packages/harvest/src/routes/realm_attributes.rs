// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak::{
    get_realm_attributes, redacted_attributes, update_realm_attributes,
    validate_realm_attributes,
};
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, instrument};
use windmill::postgres::scheduled_event::lock_scheduling_event;
use windmill::services::enrollment_windows::ENROLLMENT_WINDOWS_ATTRIBUTE;
use windmill::tasks::migrate_registration_flows::REGISTRATION_RESTORE_ATTRIBUTE;

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
    services: &State<HarvestServices>,
) -> Result<Json<UpdateRealmAttributesOutput>, JsonError> {
    let body = input.into_inner();
    let tenant_id = &claims.hasura_claims.tenant_id;

    authorize(
        &claims,
        true,
        Some(tenant_id.clone()),
        vec![Permissions::KEYCLOAK_REALM_ATTRIBUTES_WRITE],
    )
    .map_err(|err| {
        error!("Authorization failed: {:?}", err);
        ErrorResponse::new(
            Status::Forbidden,
            "Authorization failed",
            ErrorCode::Unauthorized,
        )
    })?;
    validate_realm_attributes(&body.attributes).map_err(|error| {
        ErrorResponse::new(
            Status::BadRequest,
            &error.to_string(),
            ErrorCode::RealmAttributesValidation,
        )
    })?;

    let mut connection = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(|error| {
            error!("Failed to get realm-update DB connection: {:?}", error);
            ErrorResponse::new(
                Status::InternalServerError,
                "Failed to update realm attributes",
                ErrorCode::InternalServerError,
            )
        })?;
    let transaction = connection.transaction().await.map_err(|error| {
        error!("Failed to start realm-update transaction: {:?}", error);
        ErrorResponse::new(
            Status::InternalServerError,
            "Failed to update realm attributes",
            ErrorCode::InternalServerError,
        )
    })?;
    // Keycloak writes a full realm representation. Keep enrollment repair and
    // every other event writer out until this read/merge/write has completed.
    lock_scheduling_event(&transaction, tenant_id, &body.election_event_id)
        .await
        .map_err(|error| {
            error!("Failed to lock realm-update event: {:?}", error);
            ErrorResponse::new(
                Status::InternalServerError,
                "Failed to update realm attributes",
                ErrorCode::InternalServerError,
            )
        })?;
    let current = get_realm_attributes(tenant_id, &body.election_event_id)
        .await
        .map_err(|error| {
            error!("Failed to read realm attributes for update: {:?}", error);
            ErrorResponse::new(
                Status::InternalServerError,
                "Failed to update realm attributes",
                ErrorCode::InternalServerError,
            )
        })?;
    let mut updates = body.attributes;
    for key in [ENROLLMENT_WINDOWS_ATTRIBUTE, REGISTRATION_RESTORE_ATTRIBUTE] {
        if let Some(value) = updates.get(key) {
            let changes_authority = current.get(key) != Some(value)
                && !(value.is_empty() && !current.contains_key(key));
            if changes_authority {
                return Err(ErrorResponse::new(Status::BadRequest,
                    &format!("Realm attribute {key} is managed by enrollment scheduling"),
                    ErrorCode::RealmAttributesValidation));
            }
        }
        // Keep an accepted unchanged empty value from becoming a deletion in
        // the trusted merge. Only enrollment writers own these keys.
        updates.remove(key);
    }
    update_realm_attributes(tenant_id, &body.election_event_id, updates)
        .await
        .map_err(|error| {
            error!("Failed to update realm attributes: {:?}", error);
            ErrorResponse::new(
                Status::InternalServerError,
                "Failed to update realm attributes",
                ErrorCode::InternalServerError,
            )
        })?;
    transaction.commit().await.map_err(|error| {
        error!("Failed to commit realm-update transaction: {:?}", error);
        ErrorResponse::new(
            Status::InternalServerError,
            "Failed to update realm attributes",
            ErrorCode::InternalServerError,
        )
    })?;
    Ok(Json(UpdateRealmAttributesOutput { updated: true }))
}

#[cfg(test)]
#[path = "../../tests/support/realm_attributes_routes.rs"]
mod route_tests;
