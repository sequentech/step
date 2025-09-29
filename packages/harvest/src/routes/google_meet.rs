// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use deadpool_postgres::{Client as DbClient, Transaction};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::database::get_hasura_pool;
use windmill::services::google_meet::{
    generate_google_meet_link_impl, GenerateGoogleMeetBody, GoogleMeetError,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateGoogleMeetOutput {
    pub meet_link: Option<String>,
}

#[instrument(skip(claims))]
#[post("/generate-google-meeting", format = "json", data = "<body>")]
pub async fn generate_google_meeting(
    body: Json<GenerateGoogleMeetBody>,
    claims: JwtClaims,
) -> Result<Json<GenerateGoogleMeetOutput>, (Status, String)> {
    // Authorize the user - require election event management permissions
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ADMIN_USER, Permissions::GOOGLE_MEET_LINK],
    )
    .map_err(|(status, msg)| {
        (
            status,
            GoogleMeetError::Other(format!("Authorization failed: {msg}"))
                .to_string(),
        )
    })?;

    let input = body.into_inner();

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error getting hasura db pool: {err}"),
            )
        })?;

    let hasura_transaction = hasura_db_client.transaction().await.map_err(
        |err: tokio_postgres::Error| {
            (
                Status::InternalServerError,
                format!("Error starting hasura transaction: {err}"),
            )
        },
    )?;

    info!("Generating Google Meet link...");

    let meet_link = generate_google_meet_link_impl(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &input,
    )
    .await
    .map_err(|e| (Status::InternalServerError, e.to_string()))?;

    Ok(Json(GenerateGoogleMeetOutput {
        meet_link: Some(meet_link),
    }))
}
