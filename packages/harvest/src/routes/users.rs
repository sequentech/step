// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use immu_board::util::get_tenant_name;
use rocket::response::Debug;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::types::keycloak::User;
use tracing::instrument;

#[instrument]
#[get("/users?<search>&<email>&<max>")]
pub async fn get_users(
    jwt: jwt::JwtClaims,
    search: Option<String>,
    email: Option<String>,
    max: Option<i32>,
) -> Result<Json<Vec<User>>, Debug<anyhow::Error>> {
    let board = get_tenant_name(&jwt.hasura_claims.tenant_id);
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| anyhow::Error::from(e))?;
    let users = client
        .list_users(&board, search, email, max)
        .await
        .map_err(|e| anyhow::Error::from(e))?;
    Ok(Json(users))
}
