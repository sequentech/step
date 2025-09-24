// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::google_meet::{
    generate_google_meet_link_impl, GenerateGoogleMeetBody, GoogleMeetError,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateGoogleMeetResponse {
    pub meet_link: Option<String>,
}

#[instrument(skip(claims))]
#[post("/generate-google-meeting", format = "json", data = "<body>")]
pub async fn generate_google_meeting(
    body: Json<GenerateGoogleMeetBody>,
    claims: JwtClaims,
) -> Result<Json<GenerateGoogleMeetResponse>, Json<GoogleMeetError>> {
    // Authorize the user - require election event management permissions
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ADMIN_USER],
    )
    .map_err(|e| {
        Json(GoogleMeetError::Other(format!(
            "Authorization failed: {e:?}"
        )))
    })?;

    let input = body.into_inner();

    info!("Generating Google Meet link for: {input:?}");

    let meet_link = generate_google_meet_link_impl(&input)
        .await
        .map_err(|e| Json(e))?;

    Ok(Json(GenerateGoogleMeetResponse {
        meet_link: Some(meet_link),
    }))
}
