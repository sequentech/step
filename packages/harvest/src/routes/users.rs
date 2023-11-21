// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use anyhow::{anyhow, Result};
use immu_board::util::get_tenant_name;
use rocket::http::Status;
use rocket::response::Debug;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::types::keycloak::User;
use tracing::instrument;

#[instrument(skip(claims))]
#[get("/<tenant_id>/users?<search>&<email>&<max>")]
pub async fn get_users(
    claims: jwt::JwtClaims,
    tenant_id: &str,
    search: Option<String>,
    email: Option<String>,
    max: Option<i32>,
) -> Result<Json<Vec<User>>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(tenant_id.into()),
        vec!["read-users".into()],
    )?;
    let board = get_tenant_name(tenant_id);
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let users = client
        .list_users(&board, search, email, max)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    Ok(Json(users))
}
