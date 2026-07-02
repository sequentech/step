// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::limit_access_by_countries::handle_limit_ip_access_by_countries;

#[derive(Serialize, Deserialize, Debug)]
pub struct LimitAccessByCountriesInput {
    voting_countries: Vec<String>,
    enroll_countries: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LimitAccessByCountriesOutput {
    success: bool,
}

#[instrument(skip(claims))]
#[post("/limit-access-by-countries", format = "json", data = "<body>")]
pub async fn limit_access_by_countries(
    claims: JwtClaims,
    body: Json<LimitAccessByCountriesInput>,
) -> Result<Json<LimitAccessByCountriesOutput>, JsonError> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::CLOUDFLARE_WRITE],
    )
    .map_err(|e| {
        ErrorResponse::new(
            Status::Unauthorized,
            &format!("{e:?}"),
            ErrorCode::Unauthorized,
        )
    })?;

    info!(
        "Limiting access to tenant {} by countries: {:?} and enroll {:?}",
        claims.hasura_claims.tenant_id,
        input.voting_countries,
        input.enroll_countries
    );

    handle_limit_ip_access_by_countries(
        claims.hasura_claims.tenant_id.clone(),
        input.voting_countries,
        input.enroll_countries,
    )
    .await
    .map_err(|e| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("{e:?}"),
            ErrorCode::InternalServerError,
        )
    })?;

    Ok(Json(LimitAccessByCountriesOutput { success: true }))
}
