// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::{
    ElectionEventPresentation, ElectionPresentation, Enrollment, InitReport,
    VotingStatus,
};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{event, instrument, Level};
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::services::database::get_hasura_pool;
use windmill::tasks::manage_election_event_enrollment::update_keycloak_enrollment;

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
    let body: SetVoterAuthentication = input.into_inner();

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
    info!("Authorization succeeded, processing URL update");

    // Database client initialization
    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|e| {
            error!("Failed to get DB pool: {:?}", e);
            (Status::InternalServerError, format!("{:?}", e))
        })?;

    // Begin transaction
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            error!("Failed to start transaction: {:?}", e);
            (Status::InternalServerError, format!("{:?}", e))
        })?;

    // Fetch election event data
    let election_event = get_election_event_by_id(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &body.election_event_id.clone(),
    )
    .await
    .map_err(|e| {
        error!("Failed to fetch election event: {:?}", e);
        (Status::InternalServerError, format!("{:?}", e))
    })?;

    // Determine previous data
    let prev_data = election_event.presentation.as_ref().map(|presentation| {
        let enrollment = presentation
            .get("enrollment")
            .and_then(|v| v.as_str())
            .unwrap_or(Enrollment::ENABLED.to_string().as_str())
            .to_string();

        let otp = presentation
            .get("otp")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        SetVoterAuthentication {
            election_event_id: body.election_event_id.clone(),
            enrollment,
            otp,
        }
    });

    // Compare and update enrollment if needed
    if let Some(prev_data) = prev_data {
        if prev_data.enrollment != body.enrollment {
            let enable_enrollment =
                body.enrollment.eq_ignore_ascii_case("Enabled");

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
