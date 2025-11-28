// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{anyhow, Result};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::reports::utils::get_public_asset_template;

#[derive(Deserialize, Debug)]
pub struct GetUserTemplateBody {
    template_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetUserTemplateResponse {
    template_hbs: String,
    extra_config: String,
}

#[instrument(skip(claims))]
#[post("/get-user-template", format = "json", data = "<body>")]
pub async fn get_user_template(
    body: Json<GetUserTemplateBody>,
    claims: JwtClaims,
) -> Result<Json<GetUserTemplateResponse>, (Status, String)> {
    let input = body.into_inner();

    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::REPORT_READ],
    )?;

    let base_name = if "list_of_overseas_voters" == &input.template_type {
        "overseas_voters".to_string()
    } else {
        input.template_type
    };
    let template_hbs =
        get_public_asset_template(format!("{base_name}_user.hbs").as_str())
            .await
            .map_err(|err| {
                (
                    Status::InternalServerError,
                    format!("Error fetching template: ${err}"),
                )
            })?;

    let extra_config = get_public_asset_template(
        format!("{base_name}_extra_config.json").as_str(),
    )
    .await
    .map_err(|err| {
        (
            Status::InternalServerError,
            format!(
                "Error fetching the extra_config file of the template: ${err}"
            ),
        )
    })?;

    Ok(Json(GetUserTemplateResponse {
        template_hbs,
        extra_config,
    }))
}
