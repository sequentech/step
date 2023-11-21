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
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Deserialize, Debug)]
pub struct GetUsersBody {
    tenant_id: String,
    search: Option<String>,
    email: Option<String>,
    max: Option<i32>,
}

#[instrument(skip(claims))]
#[get("/users", format = "json", data = "<body>")]
pub async fn get_users(
    claims: jwt::JwtClaims,
    body: Json<GetUsersBody>,
) -> Result<Json<Vec<User>>, (Status, String)> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec!["read-users".into()],
    )?;
    let board = get_tenant_name(&input.tenant_id);
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let users = client
        .list_users(&board, input.search, input.email, input.max)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    Ok(Json(users))
}
