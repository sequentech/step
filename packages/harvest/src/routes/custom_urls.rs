// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{event, instrument, Level};
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::services::custom_url::{
    get_page_rule, set_custom_url, PageRule, PreviousCustomUrls, Target,
};
use windmill::services::database::get_hasura_pool;

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateCustomUrlInput {
    pub origin: String,
    pub redirect_to: String,
    pub dns_prefix: String,
    pub election_id: String,
    pub key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetCustomUrlInput {
    pub redirect_to: String,
}

#[derive(Serialize)]
struct GetCustomUrlOutput {
    success: bool,
    message: String,
    origin: String,
}

#[derive(Serialize)]
struct UpdateCustomUrlOutput {
    success: bool,
    message: String,
}

#[instrument(skip(claims))]
#[post("/set-custom-url", format = "json", data = "<input>")]
pub async fn update_custom_url(
    claims: JwtClaims,
    input: Json<UpdateCustomUrlInput>,
) -> Result<Json<UpdateCustomUrlOutput>, (Status, String)> {
    let body: UpdateCustomUrlInput = input.into_inner();
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
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let election_event = get_election_event_by_id(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &body.election_id,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    info!("update_custom_url body: {:?}", body);

    match set_custom_url(
        &body.redirect_to,
        &body.origin,
        &body.dns_prefix,
        &body.key,
    )
    .await
    {
        Ok(message) => {
            info!("Custom URL successfully updated");
            let success_message = format!("Success updating custom URL");
            Ok(Json(UpdateCustomUrlOutput {
                success: true,
                message: success_message,
            }))
        }
        Err(error) => {
            let error_message =
                format!("Error updating custom URL: {:?}", error);
            error!("{}", error_message);

            Ok(Json(UpdateCustomUrlOutput {
                success: false,
                message: error_message,
            }))
        }
    }
}

#[instrument(skip(claims))]
#[post("/get-custom-url", format = "json", data = "<input>")]
pub async fn get_custom_url(
    claims: JwtClaims,
    input: Json<GetCustomUrlInput>,
) -> Result<Json<GetCustomUrlOutput>, (Status, String)> {
    let body = input.into_inner();
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_EVENT_READ],
    )?;
    // TODO FIX
    let rule = get_page_rule(&body.redirect_to, "")
        .await
        .map_err(|error| {
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
                .map(|target| target.constraint.value.clone());

            match origin {
                Some(origin) => Ok(Json(GetCustomUrlOutput {
                    success: true,
                    message: "Page rule found".to_string(),
                    origin,
                })),
                None => Ok(Json(GetCustomUrlOutput {
                    success: false,
                    message: "Error extracting page rule".to_string(),
                    origin: "".to_string(),
                })),
            }
        }
        None => Ok(Json(GetCustomUrlOutput {
            success: false,
            message: "No matching page rule found".to_string(),
            origin: "".to_string(),
        })),
    }
}
