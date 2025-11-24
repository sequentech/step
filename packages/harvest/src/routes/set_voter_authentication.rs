// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::{Enrollment, Otp};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument};
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::services::database::get_hasura_pool;
use windmill::tasks::manage_election_event_enrollment::{
    update_keycloak_enrollment, update_keycloak_otp,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct SetVoterAuthentication {
    pub election_event_id: String,
    pub enrollment: String,
    pub otp: String,
}

#[derive(Serialize)]
struct SetVoterAuthenticationOutput {
    success: bool,
    message: String,
}

#[instrument(skip(claims))]
#[post("/set-voter-authentication", format = "json", data = "<input>")]
pub async fn set_voter_authentication(
    claims: JwtClaims,
    input: Json<SetVoterAuthentication>,
) -> Result<Json<SetVoterAuthenticationOutput>, (Status, String)> {
    let body = input.into_inner();

    // Authorization check
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![],
    )
    .map_err(|err| {
        error!("Authorization failed: {:?}", err);
        (Status::Forbidden, "Authorization failed".to_string())
    })?;

    let mut hasura_db_client =
        get_hasura_pool().await.get().await.map_err(|e| {
            error!("Failed to get DB pool: {:?}", e);
            (Status::InternalServerError, format!("{:?}", e))
        })?;

    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            error!("Failed to start transaction: {:?}", e);
            (Status::InternalServerError, format!("{:?}", e))
        })?;

    let election_event = get_election_event_by_id(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &body.election_event_id,
    )
    .await
    .map_err(|e| {
        error!("Failed to fetch election event: {:?}", e);
        (Status::InternalServerError, format!("{:?}", e))
    })?;

    // Extract or set default enrollment and OTP values
    let (prev_enrollment, prev_otp) =
        election_event.presentation.as_ref().map_or(
            (Enrollment::ENABLED.to_string(), Otp::ENABLED.to_string()),
            |presentation| {
                let enrollment = presentation
                    .get("enrollment")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&Enrollment::ENABLED.to_string())
                    .to_string();

                let otp = presentation
                    .get("otp")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&Otp::ENABLED.to_string())
                    .to_string();

                (enrollment, otp)
            },
        );

    // Update enrollment if it has changed
    if !body.enrollment.trim().is_empty() && prev_enrollment != body.enrollment
    {
        let enable_enrollment =
            body.enrollment.eq(&Enrollment::ENABLED.to_string());
        info!("Updating enrollment to: {}", enable_enrollment);

        update_keycloak_enrollment(
            Some(claims.hasura_claims.tenant_id.clone()),
            Some(body.election_event_id.clone()),
            enable_enrollment,
        )
        .await
        .map_err(|error| {
            error!("Failed to update enrollment: {:?}", error);
            (
                Status::InternalServerError,
                format!("Error updating enrollment: {error:?}"),
            )
        })?;
    }

    if !body.otp.trim().is_empty() && prev_otp != body.otp {
        let new_otp_state = if body.otp == Otp::ENABLED.to_string() {
            "REQUIRED".to_string()
        } else {
            "DISABLED".to_string()
        };

        info!("Updating OTP to: {}", new_otp_state);

        update_keycloak_otp(
            Some(claims.hasura_claims.tenant_id.clone()),
            Some(body.election_event_id.clone()),
            new_otp_state,
        )
        .await
        .map_err(|error| {
            error!("Failed to update OTP: {:?}", error);
            (
                Status::InternalServerError,
                format!("Error updating OTP: {error:?}"),
            )
        })?;
    }

    // Commit transaction
    hasura_transaction.commit().await.map_err(|e| {
        error!("Transaction commit failed: {:?}", e);
        (Status::InternalServerError, format!("{:?}", e))
    })?;

    Ok(Json(SetVoterAuthenticationOutput {
        success: true,
        message: "Authentication updated successfully".to_string(),
    }))
}
