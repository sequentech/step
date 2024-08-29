// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::{
    authorization::authorize,
    custom_url::{get_page_rule, set_custom_url, PageRule, Target},
};
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateCustomUrlInput {
    pub origin: String,
    pub redirect_to: String,
    pub dns_prefix: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetCustomUrlInput {
    pub redirect_to: String,
}
#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

// TODO: Add env for cloudflare auth + for local / remote
#[instrument(skip(claims))]
#[post("/set-custom-url", format = "json", data = "<input>")]
pub async fn update_custom_url(
    claims: JwtClaims,
    input: Json<UpdateCustomUrlInput>,
) -> Result<Json<String>, (Status, String)> {
    let body = input.into_inner();
    if let Err(err) = authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_EVENT_WRITE],
    ) {
        error!("Authorization failed: {:?}", err);
        return Err((Status::Forbidden, "Authorization failed".to_string()));
    }

    info!("Authorization succeeded, processing URL update");

    match set_custom_url(&body.redirect_to, &body.origin, &body.dns_prefix).await {
        Ok(_) => {
            info!("Custom URL successfully updated");
            Ok(Json("Successfully Updated".to_string()))
        },
        Err(error) => {
            let error_message = format!("Error updating custom URL: {:?}", error);
            error!("{}", error_message);

            let error_response = ErrorResponse {
                error: "InternalServerError".to_string(),
                message: error_message.clone(),
            };

            // Serialize the error response to a string
            let error_response_string = serde_json::to_string(&error_response)
                .unwrap_or_else(|_| "Failed to serialize error response".to_string());

            // Return the serialized string in the Err variant
            return Err((Status::InternalServerError, error_response_string))
        }
    }
    }

#[instrument(skip(claims))]
#[post("/get-custom-url", format = "json", data = "<input>")]
pub async fn get_custom_url(
    claims: JwtClaims,
    input: Json<GetCustomUrlInput>,
) -> Result<Json<String>, (Status, String)> {
    let body = input.into_inner();
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_EVENT_READ],
    )?;
    let rule = get_page_rule(&body.redirect_to).await.map_err(|error| {
        (
            Status::InternalServerError,
            format!("Error reading custom url: {error:?}"),
        )
    })?;

    match rule {
        Some(r) => {
            let origin = r
                .targets
                .get(0)
                .map(|target| target.constraint.value.clone()) // The origin domain url
                .ok_or_else(|| {
                    (
                        Status::InternalServerError,
                        "Error extracting page rule".to_string(),
                    )
                })?;
            Ok(Json(origin))
        }
        None => Err((
            Status::InternalServerError,
            "No matching page rule found".to_string(),
        )),
    }
}
