// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use deadpool_postgres::{Config as PgConfig, Pool, Runtime};
use serde_json;
use sequent_core::services::keycloak::{
    generate_keycloak_token as generate_token_inner,
    refresh_keycloak_token as refresh_token_inner,
};
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use tokio_postgres::NoTls;

use crate::types::keycloak::KeycloakTokenResponse;

/// Generates a Keycloak token using Resource Owner Password Credentials flow.
///
/// This is a wrapper around the shared implementation in sequent-core that
/// converts the response to the step-cli's simplified KeycloakTokenResponse type.
pub fn generate_keycloak_token(
    keycloak_url: &str,
    username: &str,
    password: &str,
    client_id: &str,
    client_secret: &str,
    tenant_id: &str,
) -> Result<KeycloakTokenResponse, Box<dyn Error>> {
    let token_resp = generate_token_inner(
        keycloak_url,
        username,
        password,
        client_id,
        client_secret,
        tenant_id,
    )?;
    Ok(KeycloakTokenResponse {
        access_token: token_resp.access_token,
        refresh_token: token_resp.refresh_token.unwrap_or_default(),
    })
}

/// Refreshes a Keycloak token using a refresh token.
///
/// This is a wrapper around the shared implementation in sequent-core that
/// converts the response to the step-cli's simplified KeycloakTokenResponse type.
pub fn refresh_keycloak_token(
    keycloak_url: &str,
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
    tenant_id: &str,
) -> Result<KeycloakTokenResponse, Box<dyn Error>> {
    let token_resp = refresh_token_inner(
        keycloak_url,
        refresh_token,
        client_id,
        client_secret,
        tenant_id,
    )?;
    Ok(KeycloakTokenResponse {
        access_token: token_resp.access_token,
        refresh_token: token_resp.refresh_token.unwrap_or_default(),
    })
}

pub fn get_auth_token_dir() -> PathBuf {
    let exe_path = env::current_exe().expect("Failed to get current executable path");
    exe_path
        .parent()
        .expect("Failed to get executable directory")
        .join("keycloak")
}

pub fn read_token() -> Result<KeycloakTokenResponse, Box<dyn std::error::Error>> {
    let auth_dir = get_auth_token_dir();
    let auth_file = auth_dir.join("authToken.json");

    let json_data = fs::read_to_string(&auth_file)
        .expect("Failed to read auth file, Plase make sure to run `sequent generate-auth` first");
    let auth_data = serde_json::from_str(&json_data).expect("Failed to parse auth file");
    Ok(auth_data)
}

pub async fn get_keyckloak_pool() -> Result<Pool, Box<dyn std::error::Error>> {
    let mut kc_cfg = PgConfig::default();
    kc_cfg.host = Some(env::var("KC_DB_URL_HOST")?);
    kc_cfg.port = Some(env::var("KC_DB_URL_PORT")?.parse::<u16>()?);
    kc_cfg.user = Some(env::var("KC_DB_USERNAME")?);
    kc_cfg.password = Some(env::var("KC_DB_PASSWORD")?);
    kc_cfg.dbname = Some(env::var("KC_DB")?);
    Ok(kc_cfg.create_pool(Some(Runtime::Tokio1), NoTls)?)
}
