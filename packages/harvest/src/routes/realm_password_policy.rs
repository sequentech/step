// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak::{
    get_realm_password_policy, update_realm_password_policy,
    RealmPasswordPolicy,
};
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{error, instrument};

#[derive(Deserialize, Debug)]
pub struct GetRealmPasswordPolicyInput {
    pub election_event_id: String,
}

#[derive(Deserialize, Debug)]
pub struct UpdateRealmPasswordPolicyInput {
    pub election_event_id: String,
    pub minimum_length: i32,
    pub maximum_length: i32,
    pub include_uppercase: bool,
    pub include_lowercase: bool,
    pub include_digits: bool,
    pub include_special_characters: bool,
}

#[derive(Serialize)]
pub struct UpdateRealmPasswordPolicyOutput {
    pub updated: bool,
}

#[instrument(skip_all)]
#[post("/get-realm-password-policy", format = "json", data = "<input>")]
pub async fn get_realm_password_policy_route(
    claims: JwtClaims,
    input: Json<GetRealmPasswordPolicyInput>,
) -> Result<Json<RealmPasswordPolicy>, JsonError> {
    let body = input.into_inner();

    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_EVENT_READ],
    )
    .map_err(|err| {
        error!("Authorization failed: {:?}", err);
        ErrorResponse::new(
            Status::Forbidden,
            "Authorization failed",
            ErrorCode::Unauthorized,
        )
    })?;

    let password_policy = get_realm_password_policy(
        &claims.hasura_claims.tenant_id,
        &body.election_event_id,
    )
    .await
    .map_err(|error| {
        error!("Failed to get realm password policy: {:?}", error);
        ErrorResponse::new(
            Status::InternalServerError,
            "Failed to get realm password policy",
            ErrorCode::InternalServerError,
        )
    })?;

    Ok(Json(password_policy))
}

#[instrument(skip_all)]
#[post("/update-realm-password-policy", format = "json", data = "<input>")]
pub async fn update_realm_password_policy_route(
    claims: JwtClaims,
    input: Json<UpdateRealmPasswordPolicyInput>,
) -> Result<Json<UpdateRealmPasswordPolicyOutput>, JsonError> {
    let body = input.into_inner();

    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_EVENT_WRITE],
    )
    .map_err(|err| {
        error!("Authorization failed: {:?}", err);
        ErrorResponse::new(
            Status::Forbidden,
            "Authorization failed",
            ErrorCode::Unauthorized,
        )
    })?;

    let password_policy = RealmPasswordPolicy {
        configured: true,
        minimum_length: body.minimum_length,
        maximum_length: body.maximum_length,
        include_uppercase: body.include_uppercase,
        include_lowercase: body.include_lowercase,
        include_digits: body.include_digits,
        include_special_characters: body.include_special_characters,
    };
    password_policy.validate().map_err(|error| {
        ErrorResponse::new(
            Status::BadRequest,
            &error.to_string(),
            ErrorCode::InvalidPasswordPolicy,
        )
    })?;

    update_realm_password_policy(
        &claims.hasura_claims.tenant_id,
        &body.election_event_id,
        password_policy,
    )
    .await
    .map_err(|error| {
        error!("Failed to update realm password policy: {:?}", error);
        ErrorResponse::new(
            Status::InternalServerError,
            "Failed to update realm password policy",
            ErrorCode::InternalServerError,
        )
    })?;

    Ok(Json(UpdateRealmPasswordPolicyOutput { updated: true }))
}
