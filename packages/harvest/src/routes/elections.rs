use deadpool_postgres::Client as DbClient;
use rocket::serde::json::Json;
use sequent_core::{services::jwt::JwtClaims, types::{hasura::core::Election, permissions::Permissions}};
use serde::{Deserialize, Serialize};
use windmill::services::database::get_hasura_pool;
use rocket::http::Status;

use crate::services::authorization::authorize;

#[derive(Serialize, Deserialize, Debug)]
pub struct GelElectionsResponse {
    elections: Vec<Election>
}

#[get("/get_elections", format = "json")]
pub async fn get_elections_with_permissions(
    claims: JwtClaims
) -> Result<Json<GelElectionsResponse>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ADMIN_USER],
    )?;

    let mut hasura_db_client: DbClient = get_hasura_pool()
    .await
    .get()
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let hasura_transaction = hasura_db_client
    .transaction()
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let elections = windmill::postgres::election::get_elections(&hasura_transaction, &claims.hasura_claims.tenant_id)
        .await
        .map_err(|e| (rocket::http::Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(GelElectionsResponse {elections}))
}
