// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Context;
use deadpool_postgres::{Config as PgConfig, Pool, Runtime};
use serde_json;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use tokio_postgres::NoTls;

use crate::types::keycloak::KeycloakTokenResponse;

/// Generate keycloak token
pub fn generate_keycloak_token(
    keycloak_url: &str,
    username: &str,
    password: &str,
    client_id: &str,
    client_secret: &str,
    tenant_id: &str,
) -> Result<KeycloakTokenResponse, Box<dyn Error>> {
    let params = [
        ("grant_type", "password"),
        ("scope", "openid"),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("username", username),
        ("password", password),
    ];

    let realm = format!("tenant-{tenant_id}");
    let url = format!("{keycloak_url}/realms/{realm}/protocol/openid-connect/token",);

    let client = reqwest::blocking::Client::new();
    let response = client.post(&url).form(&params).send()?;

    if response.status().is_success() {
        let token_response: KeycloakTokenResponse = response.json()?;
        Ok(token_response)
    } else {
        let status = response.status();
        let error_message = response.text()?;
        let error = format!("HTTP Status: {status}\nError Message: {error_message}");
        Err(Box::from(error))
    }
}

/// Refresh keycloak token
pub fn refresh_keycloak_token(
    keycloak_url: &str,
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
    tenant_id: &str,
) -> Result<KeycloakTokenResponse, Box<dyn Error>> {
    let params = [
        ("grant_type", "refresh_token"),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("refresh_token", refresh_token),
    ];

    let realm = format!("tenant-{tenant_id}");
    let url = format!("{keycloak_url}/realms/{realm}/protocol/openid-connect/token",);

    let client = reqwest::blocking::Client::new();
    let response = client.post(&url).form(&params).send()?;

    if response.status().is_success() {
        let token_response: KeycloakTokenResponse = response.json()?;
        Ok(token_response)
    } else {
        let status = response.status();
        let error_message = response.text()?;
        let error = format!("HTTP Status: {status}\nError Message: {error_message}");
        Err(Box::from(error))
    }
}

/// Get auth token directory
pub fn get_auth_token_dir() -> PathBuf {
    let exe_path = env::current_exe().expect("Failed to get current executable path");
    exe_path
        .parent()
        .expect("Failed to get executable directory")
        .join("keycloak")
}

/// Read keycloak token
pub fn read_token() -> KeycloakTokenResponse {
    let auth_dir = get_auth_token_dir();
    let auth_file = auth_dir.join("authToken.json");

    let json_data = fs::read_to_string(&auth_file)
        .expect("Failed to read auth file, Plase make sure to run `sequent generate-auth` first");
    serde_json::from_str(&json_data).expect("Failed to parse auth file")
}

/// Get keycloak pool
pub async fn get_keyckloak_pool() -> anyhow::Result<Pool> {
    let kc_cfg = PgConfig {
        host: Some(env::var("KC_DB_URL_HOST")?),
        port: Some(env::var("KC_DB_URL_PORT")?.parse::<u16>()?),
        user: Some(env::var("KC_DB_USERNAME")?),
        password: Some(env::var("KC_DB_PASSWORD")?),
        dbname: Some(env::var("KC_DB")?),
        ..Default::default()
    };

    kc_cfg
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .context("failed to create Keycloak Postgres pool")
}
