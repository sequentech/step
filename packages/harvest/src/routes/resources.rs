use rocket::{http::Status, serde::json::Json};
use sequent_core::{services::{jwt::JwtClaims, keycloak::KeycloakAdminClient}, types::permissions::Permissions};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::resources::create_resource_impl;

use crate::services::authorization::authorize;


#[derive(Serialize, Deserialize, Debug)]
pub struct CreateResourceBody {
    resource_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateResourceResponse {
    id: String,
}

#[instrument]
#[post("/create-resource", format = "json", data = "<body>")]
pub async fn create_resource (
    body: Json<CreateResourceBody>,
    claims: JwtClaims
) -> Result<Json<CreateResourceResponse>, (Status, String)> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::RESOURCE_WRITE],
    )?;
    
    let permissions = vec!["view-resource".to_string()];
    let created_resource_id = create_resource_impl(
        &input.resource_id,
        &permissions
    ).await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;


    Ok(Json(CreateResourceResponse {
        id: created_resource_id,
    }))
    
}