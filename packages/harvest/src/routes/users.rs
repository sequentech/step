// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use rocket::response::Debug;
use rocket::serde::json::Json;
use tracing::instrument;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::services::connection;
use sequent_core::types::keycloak::User;

#[instrument]
#[get("/users?<search>&<email>&<max>")]
pub async fn get_users(
    auth_headers: connection::AuthHeaders,
    search: Option<String>,
    email: Option<String>,
    max: Option<i32>
) -> Result<Json<Vec<User>>, Debug<anyhow::Error>> {
    if  auth_headers.key != "authorization" {
        return Err(Debug(anyhow!("Unauthorized")));
    }
    let client = KeycloakAdminClient::new().await.map_err(|e| anyhow::Error::from(e))?;
    let realm: &str = "";
    let users = client.list_users(realm, search, email, max).await.map_err(|e| anyhow::Error::from(e))?;
    Ok(Json(users))
}