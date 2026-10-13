// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::audit;
use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak::{
    get_realm_password_policy, update_realm_password_policy,
    validate_keycloak_scope, RealmPasswordPolicy,
};
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, instrument};
use windmill::postgres::scheduled_event::lock_scheduling_event;
use windmill::services::audit_log::{diff_values, stage, AdminAction};
use windmill::tasks::audit_log_outbox::kick_audit_log_outbox;

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

fn validate_event_scope(
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(), JsonError> {
    validate_keycloak_scope(tenant_id, Some(election_event_id)).map_err(
        |error| {
            ErrorResponse::new(
                Status::BadRequest,
                &error.to_string(),
                ErrorCode::UuidParseFailed,
            )
        },
    )
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

    validate_event_scope(
        &claims.hasura_claims.tenant_id,
        &body.election_event_id,
    )?;

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

    Ok(Json(password_policy.to_admin_configuration()))
}

#[instrument(skip_all)]
#[post("/update-realm-password-policy", format = "json", data = "<input>")]
pub async fn update_realm_password_policy_route(
    claims: JwtClaims,
    input: Json<UpdateRealmPasswordPolicyInput>,
    services: &State<HarvestServices>,
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

    validate_event_scope(
        &claims.hasura_claims.tenant_id,
        &body.election_event_id,
    )?;

    // The entry lists the policy that was set: the route does not read the
    // one it replaces.
    let policy_set = json!({
        "minimum_length": body.minimum_length,
        "maximum_length": body.maximum_length,
        "include_uppercase": body.include_uppercase,
        "include_lowercase": body.include_lowercase,
        "include_digits": body.include_digits,
        "include_special_characters": body.include_special_characters,
    });
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

    let mut connection = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(|error| {
            error!("Failed to get password-policy DB connection: {:?}", error);
            ErrorResponse::new(
                Status::InternalServerError,
                "Failed to update realm password policy",
                ErrorCode::InternalServerError,
            )
        })?;
    let transaction = connection.transaction().await.map_err(|error| {
        error!("Failed to start password-policy transaction: {:?}", error);
        ErrorResponse::new(
            Status::InternalServerError,
            "Failed to update realm password policy",
            ErrorCode::InternalServerError,
        )
    })?;
    // The shared helper writes a full realm. Serialize with enrollment repair
    // for this event until the external write completes.
    lock_scheduling_event(
        &transaction,
        &claims.hasura_claims.tenant_id,
        &body.election_event_id,
    )
    .await
    .map_err(|error| {
        error!("Failed to lock password-policy event: {:?}", error);
        ErrorResponse::new(
            Status::InternalServerError,
            "Failed to update realm password policy",
            ErrorCode::InternalServerError,
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

    let staged = match audit::target(
        &claims.hasura_claims.tenant_id,
        Some(&body.election_event_id),
    ) {
        Ok(target) => stage(
            &transaction,
            &audit::change(
                &claims,
                target,
                audit::election_event(&body.election_event_id),
                AdminAction::Update,
            )
            .with_changes(diff_values(
                audit::PASSWORD_POLICY_PATH,
                None,
                Some(&policy_set),
            )),
        )
        .await
        .map(|_| ())
        .map_err(|error| format!("{error:?}")),
        Err((_, message)) => Err(message),
    };
    staged.map_err(|error| {
        error!("Failed to queue the password-policy audit entry: {error}");
        ErrorResponse::new(
            Status::InternalServerError,
            "Failed to update realm password policy",
            ErrorCode::InternalServerError,
        )
    })?;

    transaction.commit().await.map_err(|error| {
        error!("Failed to commit password-policy transaction: {:?}", error);
        ErrorResponse::new(
            Status::InternalServerError,
            "Failed to update realm password policy",
            ErrorCode::InternalServerError,
        )
    })?;
    kick_audit_log_outbox();
    Ok(Json(UpdateRealmPasswordPolicyOutput { updated: true }))
}
