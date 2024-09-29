use crate::services::authorization::authorize;
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
    countries: Vec<String>,
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
) -> Result<Json<LimitAccessByCountriesOutput>, (Status, String)> {
    let input = body.into_inner();
    info!("innnnnn: {:?}", input);
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ADMIN_USER],
    )?;
    info!("innnnnn: {:?}", input);

    // handle_limit_ip_access_by_countries(input.tenant_id, input.countries)
    //     .await
    //     .map_err(|err| (Status::InternalServerError, format!("{:?}", err)))?;

    Ok(Json(LimitAccessByCountriesOutput { success: true }))
}
